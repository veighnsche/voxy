use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum SttConfigError {
    MissingApiKey,
    DotenvRead {
        path: PathBuf,
        source: std::io::Error,
    },
    ApiKeyFileRead {
        path: PathBuf,
        source: std::io::Error,
    },
    ApiKeyFileEmpty {
        path: PathBuf,
    },
}

impl fmt::Display for SttConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApiKey => write!(
                f,
                "missing OpenAI API key; set VOXY_OPENAI_API_KEY, VOXY_OPENAI_API_KEY_FILE, or OPENAI_API_KEY (or place one in .env/.env.local)"
            ),
            Self::DotenvRead { path, source } => {
                write!(f, "failed to read dotenv file '{}': {source}", path.display())
            }
            Self::ApiKeyFileRead { path, source } => {
                write!(f, "failed to read API key file '{}': {source}", path.display())
            }
            Self::ApiKeyFileEmpty { path } => write!(
                f,
                "API key file '{}' exists but is empty after trimming",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SttConfigError {}
