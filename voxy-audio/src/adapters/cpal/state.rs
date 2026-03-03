use std::collections::VecDeque;
use std::sync::Mutex;

use crate::{
    adapters::cpal::config::{frame_ms, frame_samples, DEFAULT_MAX_BUFFER_FRAMES},
    AudioError, PcmFrame,
};

#[derive(Debug)]
pub struct CaptureBuffer {
    sample_rate_hz: u32,
    channels: u16,
    frame_samples: usize,
    max_samples: usize,
    queue: Mutex<VecDeque<i16>>,
}

impl CaptureBuffer {
    pub fn new(sample_rate_hz: u32, channels: u16) -> Result<Self, AudioError> {
        let frame_ms = frame_ms();
        let frame_samples = frame_samples(sample_rate_hz, channels, frame_ms).ok_or(
            AudioError::InvalidFrameConfig {
                frame_ms,
                sample_rate_hz,
                channels,
            },
        )?;
        let max_samples = frame_samples.checked_mul(DEFAULT_MAX_BUFFER_FRAMES).ok_or(
            AudioError::FrameBufferOverflow {
                frame_samples,
                max_frames: DEFAULT_MAX_BUFFER_FRAMES,
            },
        )?;
        Ok(Self {
            sample_rate_hz,
            channels,
            frame_samples,
            max_samples,
            queue: Mutex::new(VecDeque::new()),
        })
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn push_i16_samples(&self, samples: &[i16]) -> Result<(), AudioError> {
        if samples.is_empty() {
            return Ok(());
        }

        let mut queue = self
            .queue
            .lock()
            .map_err(|_| AudioError::LockPoisoned("cpal::capture_buffer"))?;
        queue.extend(samples.iter().copied());

        if queue.len() > self.max_samples {
            let overflow = queue.len() - self.max_samples;
            for _ in 0..overflow {
                let _ = queue.pop_front();
            }
        }

        Ok(())
    }

    pub fn pop_frame(&self) -> Result<Option<PcmFrame>, AudioError> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| AudioError::LockPoisoned("cpal::capture_buffer"))?;
        if queue.len() < self.frame_samples {
            return Ok(None);
        }

        let mut samples = Vec::with_capacity(self.frame_samples);
        for _ in 0..self.frame_samples {
            if let Some(sample) = queue.pop_front() {
                samples.push(sample);
            }
        }

        if samples.is_empty() {
            return Ok(None);
        }

        Ok(Some(PcmFrame::new(
            self.sample_rate_hz,
            self.channels,
            samples,
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AudioError;

    use super::CaptureBuffer;

    #[test]
    fn capture_buffer_emits_frames_when_enough_samples_arrive() {
        let buffer = CaptureBuffer::new(16_000, 1).expect("buffer should be created");
        // 20ms @ 16k mono = 320 samples.
        let half = vec![0i16; 160];
        let full = vec![1i16; 160];

        buffer.push_i16_samples(&half).expect("push should succeed");
        assert!(buffer.pop_frame().expect("pop should succeed").is_none());

        buffer.push_i16_samples(&full).expect("push should succeed");
        let frame = buffer
            .pop_frame()
            .expect("pop should succeed")
            .expect("frame should be ready");
        assert_eq!(frame.sample_rate_hz, 16_000);
        assert_eq!(frame.channels, 1);
        assert_eq!(frame.samples_i16.len(), 320);
    }

    #[test]
    fn push_reports_lock_poison() {
        let buffer = Arc::new(CaptureBuffer::new(16_000, 1).expect("buffer should be created"));
        let poisoned = Arc::clone(&buffer);
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.queue.lock().expect("queue lock should succeed");
            panic!("intentional poison");
        })
        .join();

        let result = buffer.push_i16_samples(&[1, 2, 3]);
        assert!(matches!(
            result,
            Err(AudioError::LockPoisoned("cpal::capture_buffer"))
        ));
    }

    #[test]
    fn pop_reports_lock_poison() {
        let buffer = Arc::new(CaptureBuffer::new(16_000, 1).expect("buffer should be created"));
        let poisoned = Arc::clone(&buffer);
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.queue.lock().expect("queue lock should succeed");
            panic!("intentional poison");
        })
        .join();

        let result = buffer.pop_frame();
        assert!(matches!(
            result,
            Err(AudioError::LockPoisoned("cpal::capture_buffer"))
        ));
    }
}
