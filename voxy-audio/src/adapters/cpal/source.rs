use std::{
    sync::{
        mpsc::{self, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{
    adapters::cpal::{convert, state::CaptureBuffer},
    trace, AudioError, AudioFrameSource, AudioInput, AudioRoute, PcmFrame,
};

#[derive(Clone)]
pub struct CpalFrameSource {
    inner: Arc<CpalSourceInner>,
}

impl std::fmt::Debug for CpalFrameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalFrameSource")
            .field("sample_rate_hz", &self.inner.buffer.sample_rate_hz())
            .field("channels", &self.inner.buffer.channels())
            .finish()
    }
}

impl CpalFrameSource {
    pub fn new() -> Result<Self, AudioError> {
        let (startup_tx, startup_rx) = mpsc::channel::<Result<Arc<CaptureBuffer>, AudioError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let worker = thread::Builder::new()
            .name("voxy-cpal-capture".into())
            .spawn(move || {
                let startup = (|| {
                    let host = cpal::default_host();
                    let device = host
                        .default_input_device()
                        .ok_or(AudioError::CpalNoInputDevice)?;
                    let default_config = device
                        .default_input_config()
                        .map_err(AudioError::CpalDefaultInputConfig)?;
                    let sample_format = default_config.sample_format();
                    let stream_config: cpal::StreamConfig = default_config.into();
                    let sample_rate_hz = stream_config.sample_rate.0;
                    let channels = stream_config.channels;

                    let buffer = Arc::new(CaptureBuffer::new(sample_rate_hz, channels));
                    let on_error = |error| {
                        trace::log("cpal", format!("input stream error={error}"));
                    };

                    let stream = match sample_format {
                        cpal::SampleFormat::I16 => build_i16_stream(
                            &device,
                            &stream_config,
                            Arc::clone(&buffer),
                            on_error,
                        )?,
                        cpal::SampleFormat::U16 => build_u16_stream(
                            &device,
                            &stream_config,
                            Arc::clone(&buffer),
                            on_error,
                        )?,
                        cpal::SampleFormat::F32 => build_f32_stream(
                            &device,
                            &stream_config,
                            Arc::clone(&buffer),
                            on_error,
                        )?,
                        other => {
                            return Err(AudioError::CpalUnsupportedSampleFormat(format!(
                                "{other:?}"
                            )));
                        }
                    };

                    stream.play().map_err(AudioError::CpalPlayStream)?;
                    trace::log(
                        "cpal",
                        format!(
                            "stream started sample_rate={} channels={} format={sample_format:?}",
                            sample_rate_hz, channels
                        ),
                    );
                    Ok((buffer, stream))
                })();

                match startup {
                    Ok((buffer, stream)) => {
                        let _ = startup_tx.send(Ok(Arc::clone(&buffer)));
                        loop {
                            match stop_rx.recv_timeout(Duration::from_millis(100)) {
                                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                                Err(RecvTimeoutError::Timeout) => {}
                            }
                        }
                        drop(stream);
                        trace::log("cpal", "stream stopped");
                    }
                    Err(error) => {
                        let _ = startup_tx.send(Err(error));
                    }
                }
            })
            .map_err(AudioError::CpalThreadSpawn)?;

        let buffer = startup_rx
            .recv()
            .map_err(|_| AudioError::CpalThreadStartup)??;

        Ok(Self {
            inner: Arc::new(CpalSourceInner {
                buffer,
                stop_tx: Mutex::new(Some(stop_tx)),
                worker: Mutex::new(Some(worker)),
            }),
        })
    }
}

impl AudioFrameSource for CpalFrameSource {
    fn sample_rate_hz(&self) -> u32 {
        self.inner.buffer.sample_rate_hz()
    }

    fn channels(&self) -> u16 {
        self.inner.buffer.channels()
    }

    fn read_frame(&self) -> Option<PcmFrame> {
        self.inner.buffer.pop_frame().ok().flatten()
    }
}

#[derive(Debug)]
struct CpalSourceInner {
    buffer: Arc<CaptureBuffer>,
    stop_tx: Mutex<Option<Sender<()>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for CpalSourceInner {
    fn drop(&mut self) {
        if let Ok(mut stop_tx) = self.stop_tx.lock() {
            if let Some(tx) = stop_tx.take() {
                let _ = tx.send(());
            }
        }
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpalAudioInput;

impl AudioInput for CpalAudioInput {
    fn start(&self) {}

    fn stop(&self) {}

    fn current_route(&self) -> AudioRoute {
        AudioRoute::Microphone
    }
}

fn build_i16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<CaptureBuffer>,
    on_error: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, AudioError> {
    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                let _ = buffer.push_i16_samples(data);
            },
            on_error,
            None,
        )
        .map_err(AudioError::CpalBuildStream)
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<CaptureBuffer>,
    on_error: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, AudioError> {
    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                let converted = convert::convert_u16_buffer(data);
                let _ = buffer.push_i16_samples(&converted);
            },
            on_error,
            None,
        )
        .map_err(AudioError::CpalBuildStream)
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<CaptureBuffer>,
    on_error: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, AudioError> {
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                let converted = convert::convert_f32_buffer(data);
                let _ = buffer.push_i16_samples(&converted);
            },
            on_error,
            None,
        )
        .map_err(AudioError::CpalBuildStream)
}
