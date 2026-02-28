use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

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

    pub fn read_next_frame(&self) -> Result<Option<PcmFrame>, AudioError> {
        let source = self
            .source
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
        let frame = source.as_ref().and_then(|source| source.read_frame());
        let seq = READ_FRAME_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
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
