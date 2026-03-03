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
    trace, AudioError, AudioFrameSource, PcmFrame,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
type ShutdownAction = Box<dyn FnOnce() + 'static>;

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
        Self::new_with_bootstrap(STARTUP_TIMEOUT, bootstrap_capture_stream)
    }

    fn new_with_bootstrap<F>(startup_timeout: Duration, bootstrap: F) -> Result<Self, AudioError>
    where
        F: FnOnce() -> Result<(Arc<CaptureBuffer>, ShutdownAction), AudioError> + Send + 'static,
    {
        let (startup_tx, startup_rx) = mpsc::channel::<Result<Arc<CaptureBuffer>, AudioError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let worker = thread::Builder::new()
            .name("voxy-cpal-capture".into())
            .spawn(move || {
                let startup = bootstrap();

                match startup {
                    Ok((buffer, shutdown)) => {
                        let _ = startup_tx.send(Ok(Arc::clone(&buffer)));
                        loop {
                            match stop_rx.recv_timeout(Duration::from_millis(100)) {
                                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                                Err(RecvTimeoutError::Timeout) => {}
                            }
                        }
                        shutdown();
                    }
                    Err(error) => {
                        let _ = startup_tx.send(Err(error));
                    }
                }
            })
            .map_err(AudioError::CpalThreadSpawn)?;

        let buffer = startup_rx
            .recv_timeout(startup_timeout)
            .map_err(|error| map_startup_recv_error(error, startup_timeout))??;

        Ok(Self {
            inner: Arc::new(CpalSourceInner {
                buffer,
                stop_tx: Mutex::new(Some(stop_tx)),
                worker: Mutex::new(Some(worker)),
            }),
        })
    }
}

fn bootstrap_capture_stream() -> Result<(Arc<CaptureBuffer>, ShutdownAction), AudioError> {
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

    let buffer = Arc::new(CaptureBuffer::new(sample_rate_hz, channels)?);
    let on_error = |error| {
        trace::log("cpal", format!("input stream error={error}"));
    };

    let stream = match sample_format {
        cpal::SampleFormat::I16 => {
            build_i16_stream(&device, &stream_config, Arc::clone(&buffer), on_error)?
        }
        cpal::SampleFormat::U16 => {
            build_u16_stream(&device, &stream_config, Arc::clone(&buffer), on_error)?
        }
        cpal::SampleFormat::F32 => {
            build_f32_stream(&device, &stream_config, Arc::clone(&buffer), on_error)?
        }
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

    let shutdown: ShutdownAction = Box::new(move || {
        drop(stream);
        trace::log("cpal", "stream stopped");
    });
    Ok((buffer, shutdown))
}

fn map_startup_recv_error(error: RecvTimeoutError, startup_timeout: Duration) -> AudioError {
    match error {
        RecvTimeoutError::Timeout => AudioError::CpalThreadStartupTimeout {
            timeout_ms: startup_timeout.as_millis() as u64,
        },
        RecvTimeoutError::Disconnected => AudioError::CpalThreadStartup,
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
        self.read_frame_checked().ok().flatten()
    }

    fn read_frame_checked(&self) -> Result<Option<PcmFrame>, AudioError> {
        self.inner.buffer.pop_frame()
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
                let _ = std::thread::Builder::new()
                    .name("voxy-cpal-capture-join".to_owned())
                    .spawn(move || {
                        let _ = handle.join();
                    });
            }
        }
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

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::RecvTimeoutError, sync::Arc};

    use super::{map_startup_recv_error, CpalFrameSource};
    use crate::{adapters::cpal::state::CaptureBuffer, AudioError};

    #[test]
    fn startup_maps_disconnected_worker_error() {
        let error = map_startup_recv_error(
            RecvTimeoutError::Disconnected,
            std::time::Duration::from_millis(5),
        );
        assert!(matches!(error, AudioError::CpalThreadStartup));
    }

    #[test]
    fn startup_maps_timeout_worker_error() {
        let error = map_startup_recv_error(
            RecvTimeoutError::Timeout,
            std::time::Duration::from_millis(12),
        );
        assert!(matches!(
            error,
            AudioError::CpalThreadStartupTimeout { timeout_ms: 12 }
        ));
    }

    #[test]
    fn startup_propagates_no_input_device_error() {
        let result =
            CpalFrameSource::new_with_bootstrap(std::time::Duration::from_millis(5), || {
                Err(AudioError::CpalNoInputDevice)
            });
        assert!(matches!(result, Err(AudioError::CpalNoInputDevice)));
    }

    #[test]
    fn startup_propagates_build_stream_error() {
        let result =
            CpalFrameSource::new_with_bootstrap(std::time::Duration::from_millis(5), || {
                Err(AudioError::CpalBuildStream(
                    cpal::BuildStreamError::StreamConfigNotSupported,
                ))
            });
        assert!(matches!(
            result,
            Err(AudioError::CpalBuildStream(
                cpal::BuildStreamError::StreamConfigNotSupported
            ))
        ));
    }

    #[test]
    fn startup_propagates_play_stream_error() {
        let result =
            CpalFrameSource::new_with_bootstrap(std::time::Duration::from_millis(5), || {
                Err(AudioError::CpalPlayStream(
                    cpal::PlayStreamError::DeviceNotAvailable,
                ))
            });
        assert!(matches!(
            result,
            Err(AudioError::CpalPlayStream(
                cpal::PlayStreamError::DeviceNotAvailable
            ))
        ));
    }

    #[test]
    fn startup_accepts_injected_buffer() {
        let result =
            CpalFrameSource::new_with_bootstrap(std::time::Duration::from_millis(50), || {
                let buffer = Arc::new(
                    CaptureBuffer::new(16_000, 1).expect("injected buffer should initialize"),
                );
                Ok((buffer, Box::new(|| {})))
            });
        assert!(result.is_ok());
    }
}
