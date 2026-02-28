use std::{collections::VecDeque, fs::File, path::Path, sync::Mutex};

use minimp3::{Decoder, Error as Mp3Error};

use crate::{AudioError, AudioFrameSource, PcmFrame};

const DEFAULT_FRAME_MS: usize = 100;

#[derive(Debug)]
pub struct FixtureMp3Adapter {
    sample_rate_hz: u32,
    channels: u16,
    frames: Mutex<VecDeque<PcmFrame>>,
}

impl FixtureMp3Adapter {
    pub fn load(path: &Path) -> Result<Self, AudioError> {
        let (sample_rate_hz, channels, samples) = decode_fixture_mp3(path)?;
        let chunk_samples = ((sample_rate_hz as usize / (1000 / DEFAULT_FRAME_MS))
            * channels as usize)
            .max(channels as usize);

        let mut frames = VecDeque::new();
        for chunk in samples.chunks(chunk_samples) {
            frames.push_back(PcmFrame::new(sample_rate_hz, channels, chunk.to_vec()));
        }

        Ok(Self {
            sample_rate_hz,
            channels,
            frames: Mutex::new(frames),
        })
    }

    pub fn remaining_frames(&self) -> Result<usize, AudioError> {
        self.frames
            .lock()
            .map(|frames| frames.len())
            .map_err(|_| AudioError::LockPoisoned("fixture_mp3::frames"))
    }
}

impl AudioFrameSource for FixtureMp3Adapter {
    fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn read_frame(&self) -> Option<PcmFrame> {
        self.frames
            .lock()
            .ok()
            .and_then(|mut frames| frames.pop_front())
    }
}

fn decode_fixture_mp3(path: &Path) -> Result<(u32, u16, Vec<i16>), AudioError> {
    let file = File::open(path).map_err(|source| AudioError::FixtureOpen {
        path: path.to_path_buf(),
        source,
    })?;

    let mut decoder = Decoder::new(file);
    let mut sample_rate_hz = None;
    let mut channels = None;
    let mut samples_i16 = Vec::new();

    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                let frame_rate = frame.sample_rate as u32;
                let frame_channels = frame.channels as u16;

                if let Some(previous_rate) = sample_rate_hz {
                    if previous_rate != frame_rate {
                        return Err(AudioError::FixtureDecode {
                            path: path.to_path_buf(),
                            reason: format!(
                                "sample rate changed from {previous_rate} to {frame_rate}"
                            ),
                        });
                    }
                }

                if let Some(previous_channels) = channels {
                    if previous_channels != frame_channels {
                        return Err(AudioError::FixtureDecode {
                            path: path.to_path_buf(),
                            reason: format!(
                                "channel count changed from {previous_channels} to {frame_channels}"
                            ),
                        });
                    }
                }

                sample_rate_hz = Some(frame_rate);
                channels = Some(frame_channels);
                samples_i16.extend_from_slice(&frame.data);
            }
            Err(Mp3Error::Eof) => break,
            Err(error) => {
                return Err(AudioError::FixtureDecode {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                });
            }
        }
    }

    if samples_i16.is_empty() {
        return Err(AudioError::FixtureDecode {
            path: path.to_path_buf(),
            reason: "fixture has no decoded PCM samples".to_owned(),
        });
    }

    Ok((
        sample_rate_hz.unwrap_or(16_000),
        channels.unwrap_or(1),
        samples_i16,
    ))
}
