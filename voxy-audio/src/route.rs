#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioRoute {
    Microphone,
    Fixture(String),
}

impl Default for AudioRoute {
    fn default() -> Self {
        Self::Microphone
    }
}

impl AudioRoute {
    pub fn fixture_test(id: u8) -> Self {
        Self::Fixture(format!("test_{id}"))
    }

    pub fn fixture_name(&self) -> Option<&str> {
        match self {
            Self::Fixture(name) => Some(name.as_str()),
            Self::Microphone => None,
        }
    }
}
