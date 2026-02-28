use std::{path::PathBuf, sync::Mutex};

use crate::{
    adapters::{cpal::CpalFrameSource, fixture_mp3::FixtureMp3Adapter},
    engine::session::{SessionSnapshot, SessionState},
    fixtures::resolver,
    AudioError, AudioFrameSource, AudioInput, AudioRoute, PcmFrame,
};

pub struct InputEngine {
    fixture_root: PathBuf,
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
        Self::with_fixture_root(resolver::default_fixture_root())
    }

    pub fn with_fixture_root(fixture_root: PathBuf) -> Self {
        Self {
            fixture_root,
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

        self.rebuild_source_for_route(&route)?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
        session.start();

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

        let mut source = self
            .source
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::source"))?;
        *source = None;
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
            self.rebuild_source_for_route(&route)?;
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| AudioError::LockPoisoned("input_engine::session"))?;
        session.set_route(route);

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
        Ok(source.as_ref().and_then(|source| source.read_frame()))
    }

    fn rebuild_source_for_route(&self, route: &AudioRoute) -> Result<(), AudioError> {
        let new_source: Option<Box<dyn AudioFrameSource>> = match route {
            AudioRoute::Microphone => Some(Box::new(CpalFrameSource::default())),
            AudioRoute::Fixture(fixture_name) => {
                let fixture_path = resolver::resolve_fixture_mp3(&self.fixture_root, fixture_name)?;
                let fixture_adapter = FixtureMp3Adapter::load(&fixture_path)?;
                Some(Box::new(fixture_adapter))
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
