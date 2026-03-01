use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum ModelLifecycleError {
    MissingDataDirectory,
    CreateModelsDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    RemoveModelFile {
        path: PathBuf,
        source: std::io::Error,
    },
    CreatePartialModelFile {
        path: PathBuf,
        source: std::io::Error,
    },
    DownloadRequest {
        url: String,
        message: String,
    },
    ReadDownloadStream {
        url: String,
        source: std::io::Error,
    },
    WritePartialModelFile {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteModelFile {
        path: PathBuf,
        source: std::io::Error,
    },
    FinalizeModelFile {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    EmptyDownload {
        url: String,
    },
}

impl fmt::Display for ModelLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDataDirectory => {
                write!(
                    f,
                    "no data directory available (missing XDG_DATA_HOME and HOME)"
                )
            }
            Self::CreateModelsDirectory { path, source } => {
                write!(
                    f,
                    "failed to create models directory '{}': {source}",
                    path.display()
                )
            }
            Self::RemoveModelFile { path, source } => {
                write!(
                    f,
                    "failed to remove model file '{}': {source}",
                    path.display()
                )
            }
            Self::CreatePartialModelFile { path, source } => {
                write!(
                    f,
                    "failed to create partial model file '{}': {source}",
                    path.display()
                )
            }
            Self::DownloadRequest { url, message } => {
                write!(f, "model download request failed for '{}': {message}", url)
            }
            Self::ReadDownloadStream { url, source } => {
                write!(
                    f,
                    "failed to read model download stream '{}': {source}",
                    url
                )
            }
            Self::WritePartialModelFile { path, source } => {
                write!(
                    f,
                    "failed to write partial model file '{}': {source}",
                    path.display()
                )
            }
            Self::WriteModelFile { path, source } => {
                write!(
                    f,
                    "failed to write model file '{}': {source}",
                    path.display()
                )
            }
            Self::FinalizeModelFile { from, to, source } => {
                write!(
                    f,
                    "failed to finalize model file from '{}' to '{}': {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::EmptyDownload { url } => {
                write!(f, "model download returned empty payload for '{}'", url)
            }
        }
    }
}

impl std::error::Error for ModelLifecycleError {}
