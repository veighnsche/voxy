use std::{
    collections::VecDeque,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Mutex,
    },
};

use minimp3::{Decoder as Mp3Decoder, Error as Mp3Error, Frame as Mp3Frame};

use crate::{
    adapters::cpal::CpalFrameSource,
    engine::session::{SessionSnapshot, SessionState},
    trace, AudioError, AudioFrameSource, AudioInput, AudioRoute, PcmFrame,
};

static READ_FRAME_SEQ: AtomicU64 = AtomicU64::new(0);
static NONE_FRAME_STREAK: AtomicU64 = AtomicU64::new(0);

pub struct InputEngine {
    session: Mutex<SessionState>,
    source: Mutex<Option<Box<dyn AudioFrameSource>>>,
    injected_samples: Mutex<Option<InjectedSampleQueue>>,
    latest_level_bits: AtomicU32,
}

#[derive(Debug)]
struct InjectedSampleQueue {
    sample_rate_hz: u32,
    channels: u16,
    samples: VecDeque<i16>,
}

impl InjectedSampleQueue {
    fn new(sample_rate_hz: u32, channels: u16) -> Self {
        Self {
            sample_rate_hz,
            channels,
            samples: VecDeque::new(),
        }
    }
}

impl Default for InputEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEngine {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(SessionState::default()),
            source: Mutex::new(None),
            injected_samples: Mutex::new(None),
            latest_level_bits: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    pub fn start_checked(&self) -> Result<(), AudioError> {
        let route = {
            let session = self
                .session
                .lock()
                .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
            if session.is_running() {
                return Ok(());
            }
            session.route()
        };
        trace::log("start", format!("start_checked route={route:?}"));

        self.rebuild_source_for_route(&route)?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
        session.start();
        trace::log("start", "session started");

        Ok(())
    }

    pub fn stop_checked(&self) -> Result<(), AudioError> {
        {
            let mut session = self
                .session
                .lock()
                .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
            session.stop();
        }
        trace::log("stop", "session stopped");

        let mut source = self
            .source
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
        *source = None;
        trace::log("stop", "source cleared");

        let mut injected_samples = self
            .injected_samples
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::injected_samples"))?;
        *injected_samples = None;
        trace::log("stop", "fixture injection cleared");
        self.latest_level_bits
            .store(0.0f32.to_bits(), Ordering::Relaxed);

        Ok(())
    }

    pub fn set_route_checked(&self, route: AudioRoute) -> Result<(), AudioError> {
        let should_rebuild = {
            let session = self
                .session
                .lock()
                .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
            session.is_running()
        };

        if should_rebuild {
            trace::log(
                "route",
                format!("set_route_checked rebuild route={route:?}"),
            );
            self.rebuild_source_for_route(&route)?;
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
        session.set_route(route);
        trace::log("route", format!("route set to {:?}", session.route()));

        Ok(())
    }

    pub fn route(&self) -> AudioRoute {
        self.session
            .lock()
            .map(|session| session.route())
            .unwrap_or(AudioRoute::Microphone)
    }

    pub fn snapshot(&self) -> Result<SessionSnapshot, AudioError> {
        self.session
            .lock()
            .map(|session| session.snapshot())
            .map_err(|_| AudioError::LockPoisoned("input_engine::session"))
    }

    pub fn latest_input_level(&self) -> f32 {
        f32::from_bits(self.latest_level_bits.load(Ordering::Relaxed))
    }

    pub fn inject_fixture_checked(&self, fixture_id: u8) -> Result<(), AudioError> {
        let (target_sample_rate_hz, target_channels) = {
            let source = self
                .source
                .lock()
                .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
            let Some(source) = source.as_ref() else {
                return Err(AudioError::FixtureInjectWhileStopped);
            };
            (source.sample_rate_hz(), source.channels())
        };

        let fixture_path = fixture_audio_path(fixture_id);
        if !fixture_path.is_file() {
            return Err(AudioError::FixtureNotFound(
                fixture_path.display().to_string(),
            ));
        }

        let sample_buffer =
            decode_fixture_samples(&fixture_path, target_sample_rate_hz, target_channels)?;
        if sample_buffer.is_empty() {
            return Err(AudioError::FixtureDecode {
                path: fixture_path.display().to_string(),
                message: "decoded fixture audio is empty".to_owned(),
            });
        }

        let mut injected_samples = self
            .injected_samples
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::injected_samples"))?;
        if injected_samples
            .as_ref()
            .map(|queue| {
                queue.sample_rate_hz != target_sample_rate_hz || queue.channels != target_channels
            })
            .unwrap_or(true)
        {
            *injected_samples = Some(InjectedSampleQueue::new(
                target_sample_rate_hz,
                target_channels,
            ));
        }

        let Some(queue) = injected_samples.as_mut() else {
            return Err(AudioError::FixtureDecode {
                path: fixture_path.display().to_string(),
                message: "internal invariant failed: injected sample queue was not initialized"
                    .to_owned(),
            });
        };
        let added_samples = sample_buffer.len();
        queue.samples.extend(sample_buffer);
        trace::log(
            "inject",
            format!(
                "fixture queued id={} sample_rate={} channels={} added_samples={} queued_samples={}",
                fixture_id,
                target_sample_rate_hz,
                target_channels,
                added_samples,
                queue.samples.len()
            ),
        );
        Ok(())
    }

    pub fn read_next_frame(&self) -> Result<Option<PcmFrame>, AudioError> {
        let source = self
            .source
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
        let mut frame = source.as_ref().and_then(|source| source.read_frame());
        let seq = READ_FRAME_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some(frame) = frame.as_mut() {
            self.mix_injected_samples(frame, seq)?;
            self.update_level_from_frame(frame);
        } else {
            self.decay_level();
        }
        match frame.as_ref() {
            Some(frame) => {
                NONE_FRAME_STREAK.store(0, Ordering::Relaxed);
                if trace::should_log_noisy(seq) {
                    trace::log(
                        "frame",
                        format!(
                            "read_next_frame#{} sample_rate={} channels={} samples={}",
                            seq,
                            frame.sample_rate_hz,
                            frame.channels,
                            frame.samples_i16.len()
                        ),
                    );
                }
            }
            None => {
                let streak = NONE_FRAME_STREAK.fetch_add(1, Ordering::Relaxed) + 1;
                let sparse_every = trace::noisy_every().saturating_mul(10).max(100);
                if streak == 1 || streak % sparse_every == 0 {
                    trace::log(
                        "frame",
                        format!("read_next_frame none streak={} total_seq={}", streak, seq),
                    );
                }
            }
        }
        Ok(frame)
    }

    fn update_level_from_frame(&self, frame: &PcmFrame) {
        let next = frame_peak_normalized(&frame.samples_i16);
        self.latest_level_bits
            .store(next.to_bits(), Ordering::Relaxed);
    }

    fn decay_level(&self) {
        let current = self.latest_input_level();
        let next = (current - 0.015).max(0.0);
        self.latest_level_bits
            .store(next.to_bits(), Ordering::Relaxed);
    }

    fn mix_injected_samples(&self, frame: &mut PcmFrame, seq: u64) -> Result<(), AudioError> {
        let mut injected_samples = self
            .injected_samples
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::injected_samples"))?;
        let Some(queue) = injected_samples.as_mut() else {
            return Ok(());
        };

        if queue.sample_rate_hz != frame.sample_rate_hz || queue.channels != frame.channels {
            trace::log(
                "inject",
                format!(
                    "injection format mismatch; dropping queue queued_rate={} queued_channels={} frame_rate={} frame_channels={}",
                    queue.sample_rate_hz, queue.channels, frame.sample_rate_hz, frame.channels
                ),
            );
            *injected_samples = None;
            return Ok(());
        }

        let mut mixed = 0usize;
        for sample in &mut frame.samples_i16 {
            let Some(injected) = queue.samples.pop_front() else {
                break;
            };
            *sample = mix_i16(*sample, injected);
            mixed += 1;
        }

        if mixed > 0 && trace::should_log_noisy(seq) {
            trace::log(
                "inject",
                format!(
                    "mixed fixture samples frame_seq={} mixed_samples={} remaining_samples={}",
                    seq,
                    mixed,
                    queue.samples.len()
                ),
            );
        }

        if queue.samples.is_empty() {
            trace::log("inject", "fixture queue drained");
            *injected_samples = None;
        }

        Ok(())
    }

    fn rebuild_source_for_route(&self, route: &AudioRoute) -> Result<(), AudioError> {
        trace::log(
            "source",
            format!("rebuild_source_for_route route={route:?}"),
        );
        let new_source: Option<Box<dyn AudioFrameSource>> = match route {
            AudioRoute::Microphone => {
                trace::log("source", "using CpalFrameSource");
                Some(Box::new(CpalFrameSource::new()?))
            }
        };

        let mut source = self
            .source
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
        *source = new_source;

        Ok(())
    }
}

fn mix_i16(mic: i16, injected: i16) -> i16 {
    (mic as i32 + injected as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn frame_peak_normalized(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let peak = samples
        .iter()
        .map(|sample| sample.saturating_abs() as f32 / i16::MAX as f32)
        .fold(0.0, f32::max);

    peak.clamp(0.0, 1.0)
}

fn decode_fixture_samples(
    fixture_path: &Path,
    target_sample_rate_hz: u32,
    target_channels: u16,
) -> Result<Vec<i16>, AudioError> {
    let raw = fs::read(fixture_path).map_err(|source| AudioError::FixtureRead {
        path: fixture_path.display().to_string(),
        source,
    })?;

    let mut decoder = Mp3Decoder::new(Cursor::new(raw));
    let mut converted_samples = Vec::new();

    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                let converted = convert_mp3_frame(&frame, target_sample_rate_hz, target_channels);
                converted_samples.extend(converted);
            }
            Err(Mp3Error::Eof) => break,
            Err(error) => {
                return Err(AudioError::FixtureDecode {
                    path: fixture_path.display().to_string(),
                    message: error.to_string(),
                });
            }
        }
    }

    Ok(converted_samples)
}

fn convert_mp3_frame(
    frame: &Mp3Frame,
    target_sample_rate_hz: u32,
    target_channels: u16,
) -> Vec<i16> {
    if frame.sample_rate <= 0 || frame.channels <= 0 || frame.data.is_empty() {
        return Vec::new();
    }

    let source_sample_rate_hz = frame.sample_rate as u32;
    let source_channels = frame.channels as u16;
    let resampled = resample_interleaved_i16(
        &frame.data,
        source_sample_rate_hz,
        source_channels,
        target_sample_rate_hz,
    );
    convert_channels_interleaved(&resampled, source_channels, target_channels)
}

fn resample_interleaved_i16(
    samples: &[i16],
    source_sample_rate_hz: u32,
    channels: u16,
    target_sample_rate_hz: u32,
) -> Vec<i16> {
    if samples.is_empty() || channels == 0 {
        return Vec::new();
    }
    if source_sample_rate_hz == target_sample_rate_hz {
        return samples.to_vec();
    }

    let channels = channels as usize;
    let source_frames = samples.len() / channels;
    if source_frames == 0 {
        return Vec::new();
    }

    let target_frames = ((source_frames as u64 * target_sample_rate_hz as u64)
        .div_ceil(source_sample_rate_hz as u64)) as usize;
    let mut output = Vec::with_capacity(target_frames * channels);

    for target_index in 0..target_frames {
        let source_index = ((target_index as u64 * source_sample_rate_hz as u64)
            / target_sample_rate_hz as u64) as usize;
        let source_index = source_index.min(source_frames - 1);
        let frame_offset = source_index * channels;
        output.extend_from_slice(&samples[frame_offset..frame_offset + channels]);
    }

    output
}

fn convert_channels_interleaved(
    samples: &[i16],
    source_channels: u16,
    target_channels: u16,
) -> Vec<i16> {
    if samples.is_empty() || source_channels == 0 || target_channels == 0 {
        return Vec::new();
    }
    if source_channels == target_channels {
        return samples.to_vec();
    }

    let source_channels = source_channels as usize;
    let target_channels = target_channels as usize;
    let frames = samples.len() / source_channels;
    let mut output = Vec::with_capacity(frames * target_channels);

    for frame_index in 0..frames {
        let source_frame_offset = frame_index * source_channels;
        let source_frame = &samples[source_frame_offset..source_frame_offset + source_channels];

        match (source_channels, target_channels) {
            (1, n) => output.extend(std::iter::repeat_n(source_frame[0], n)),
            (n, 1) if n > 1 => {
                let sum: i32 = source_frame.iter().map(|sample| *sample as i32).sum();
                output.push((sum / n as i32) as i16);
            }
            _ => {
                for channel in 0..target_channels {
                    let source_channel = channel.min(source_channels - 1);
                    output.push(source_frame[source_channel]);
                }
            }
        }
    }

    output
}

fn fixture_audio_path(fixture_id: u8) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("audio")
        .join(format!("test_{fixture_id}.mp3"))
}

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(manifest_dir)
}

impl AudioInput for InputEngine {
    fn start(&self) {
        let _ = self.start_checked();
    }

    fn stop(&self) {
        let _ = self.stop_checked();
    }

    fn set_route(&self, route: AudioRoute) -> Result<(), AudioError> {
        self.set_route_checked(route)
    }

    fn current_route(&self) -> AudioRoute {
        self.route()
    }
}

impl AudioFrameSource for InputEngine {
    fn sample_rate_hz(&self) -> u32 {
        self.source
            .lock()
            .ok()
            .and_then(|source| source.as_ref().map(|source| source.sample_rate_hz()))
            .unwrap_or(16_000)
    }

    fn channels(&self) -> u16 {
        self.source
            .lock()
            .ok()
            .and_then(|source| source.as_ref().map(|source| source.channels()))
            .unwrap_or(1)
    }

    fn read_frame(&self) -> Option<PcmFrame> {
        self.read_next_frame().ok().flatten()
    }
}
