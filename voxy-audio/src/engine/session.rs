use crate::AudioRoute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub running: bool,
    pub route: AudioRoute,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    running: bool,
    route: AudioRoute,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            running: false,
            route: AudioRoute::Microphone,
        }
    }
}

impl SessionState {
    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn set_route(&mut self, route: AudioRoute) {
        self.route = route;
    }

    pub fn route(&self) -> AudioRoute {
        self.route.clone()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            running: self.running,
            route: self.route.clone(),
        }
    }
}
