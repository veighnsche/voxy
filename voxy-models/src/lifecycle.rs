use std::{
    collections::HashMap,
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{error::ModelLifecycleError, ManagedModel};

const XDG_DATA_HOME_ENV: &str = "XDG_DATA_HOME";
const HOME_ENV: &str = "HOME";
const APP_DATA_DIR: &str = "voxy";
const MODELS_DIR: &str = "models";
const WHISPER_LARGE_V3_TURBO_URL_ENV: &str = "VOXY_MODEL_URL_WHISPER_LARGE_V3_TURBO";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallState {
    NotDownloaded,
    Downloaded,
}

pub type ModelLifecycleResult<T> = Result<T, ModelLifecycleError>;

#[derive(Debug, Clone, Default)]
pub struct ModelLifecycle {
    root: Option<PathBuf>,
    download_url_overrides: HashMap<ManagedModel, String>,
}

impl ModelLifecycle {
    pub fn from_env() -> Self {
        let root = xdg_data_home().map(|dir| dir.join(APP_DATA_DIR).join(MODELS_DIR));
        let mut download_url_overrides = HashMap::new();

        if let Some(url) = non_empty_env(WHISPER_LARGE_V3_TURBO_URL_ENV) {
            download_url_overrides.insert(ManagedModel::WhisperLargeV3Turbo, url);
        }

        Self {
            root,
            download_url_overrides,
        }
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            download_url_overrides: HashMap::new(),
        }
    }

    pub fn with_root_and_overrides(
        root: PathBuf,
        download_url_overrides: HashMap<ManagedModel, String>,
    ) -> Self {
        Self {
            root: Some(root),
            download_url_overrides,
        }
    }

    pub fn root_path(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn install_state(&self, model: ManagedModel) -> ModelLifecycleResult<InstallState> {
        let path = self.model_file_path(model)?;
        if path.exists() && is_non_empty_file(&path) {
            Ok(InstallState::Downloaded)
        } else {
            Ok(InstallState::NotDownloaded)
        }
    }

    pub fn perform_primary_action(
        &self,
        model: ManagedModel,
    ) -> ModelLifecycleResult<InstallState> {
        self.perform_primary_action_with_progress(model, |_| {})
    }

    pub fn perform_primary_action_with_progress<F>(
        &self,
        model: ManagedModel,
        mut on_progress: F,
    ) -> ModelLifecycleResult<InstallState>
    where
        F: FnMut(f32),
    {
        match self.install_state(model)? {
            InstallState::NotDownloaded => {
                self.download_model(model, &mut on_progress)?;
                Ok(InstallState::Downloaded)
            }
            InstallState::Downloaded => {
                self.remove_model(model)?;
                Ok(InstallState::NotDownloaded)
            }
        }
    }

    pub fn set_install_state(
        &self,
        model: ManagedModel,
        state: InstallState,
    ) -> ModelLifecycleResult<()> {
        match state {
            InstallState::NotDownloaded => self.remove_model(model),
            InstallState::Downloaded => self.write_placeholder_file(model),
        }
    }

    fn download_model<F>(
        &self,
        model: ManagedModel,
        on_progress: &mut F,
    ) -> ModelLifecycleResult<()>
    where
        F: FnMut(f32),
    {
        self.ensure_root_dir()?;
        let final_path = self.model_file_path(model)?;
        let partial_path = self.partial_file_path(model)?;
        let url = self.model_download_url(model);
        on_progress(0.0);

        if partial_path.exists() {
            fs::remove_file(&partial_path).map_err(|source| {
                ModelLifecycleError::RemoveModelFile {
                    path: partial_path.clone(),
                    source,
                }
            })?;
        }

        let response =
            ureq::get(&url)
                .call()
                .map_err(|error| ModelLifecycleError::DownloadRequest {
                    url: url.clone(),
                    message: error.to_string(),
                })?;
        let total_bytes = response
            .header("Content-Length")
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0);

        let mut reader = response.into_reader();
        let mut file = fs::File::create(&partial_path).map_err(|source| {
            ModelLifecycleError::CreatePartialModelFile {
                path: partial_path.clone(),
                source,
            }
        })?;

        let mut bytes_written = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        let mut unknown_total_fraction = 0.0_f32;
        loop {
            let read = reader.read(&mut buffer).map_err(|source| {
                ModelLifecycleError::ReadDownloadStream {
                    url: url.clone(),
                    source,
                }
            })?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).map_err(|source| {
                ModelLifecycleError::WritePartialModelFile {
                    path: partial_path.clone(),
                    source,
                }
            })?;
            bytes_written = bytes_written.saturating_add(read as u64);
            if let Some(total) = total_bytes {
                on_progress((bytes_written as f32 / total as f32).clamp(0.0, 1.0));
            } else {
                unknown_total_fraction = (unknown_total_fraction + 0.02).min(0.95);
                on_progress(unknown_total_fraction);
            }
        }

        file.flush()
            .map_err(|source| ModelLifecycleError::WritePartialModelFile {
                path: partial_path.clone(),
                source,
            })?;

        if bytes_written == 0 {
            let _ = fs::remove_file(&partial_path);
            return Err(ModelLifecycleError::EmptyDownload { url });
        }

        if final_path.exists() {
            fs::remove_file(&final_path).map_err(|source| {
                ModelLifecycleError::RemoveModelFile {
                    path: final_path.clone(),
                    source,
                }
            })?;
        }

        fs::rename(&partial_path, &final_path).map_err(|source| {
            ModelLifecycleError::FinalizeModelFile {
                from: partial_path,
                to: final_path,
                source,
            }
        })?;
        on_progress(1.0);

        Ok(())
    }

    fn remove_model(&self, model: ManagedModel) -> ModelLifecycleResult<()> {
        let final_path = self.model_file_path(model)?;
        let partial_path = self.partial_file_path(model)?;

        if final_path.exists() {
            fs::remove_file(&final_path).map_err(|source| {
                ModelLifecycleError::RemoveModelFile {
                    path: final_path.clone(),
                    source,
                }
            })?;
        }

        if partial_path.exists() {
            fs::remove_file(&partial_path).map_err(|source| {
                ModelLifecycleError::RemoveModelFile {
                    path: partial_path.clone(),
                    source,
                }
            })?;
        }

        Ok(())
    }

    fn write_placeholder_file(&self, model: ManagedModel) -> ModelLifecycleResult<()> {
        self.ensure_root_dir()?;
        let final_path = self.model_file_path(model)?;
        fs::write(&final_path, format!("{}\n", model.id())).map_err(|source| {
            ModelLifecycleError::WriteModelFile {
                path: final_path,
                source,
            }
        })
    }

    fn model_download_url(&self, model: ManagedModel) -> String {
        self.download_url_overrides
            .get(&model)
            .cloned()
            .unwrap_or_else(|| model.download_url().to_owned())
    }

    fn model_file_path(&self, model: ManagedModel) -> ModelLifecycleResult<PathBuf> {
        let root = self
            .root
            .as_ref()
            .ok_or(ModelLifecycleError::MissingDataDirectory)?;
        Ok(root.join(model.artifact_file_name()))
    }

    fn partial_file_path(&self, model: ManagedModel) -> ModelLifecycleResult<PathBuf> {
        let final_path = self.model_file_path(model)?;
        Ok(final_path.with_extension("partial"))
    }

    fn ensure_root_dir(&self) -> ModelLifecycleResult<()> {
        let root = self
            .root
            .as_ref()
            .ok_or(ModelLifecycleError::MissingDataDirectory)?;
        fs::create_dir_all(root).map_err(|source| ModelLifecycleError::CreateModelsDirectory {
            path: root.clone(),
            source,
        })
    }
}

fn xdg_data_home() -> Option<PathBuf> {
    if let Some(dir) = non_empty_env(XDG_DATA_HOME_ENV) {
        return Some(PathBuf::from(dir));
    }

    non_empty_env(HOME_ENV).map(|home| PathBuf::from(home).join(".local").join("share"))
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .and_then(|value| if value.is_empty() { None } else { Some(value) })
}

fn is_non_empty_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::{Path, PathBuf},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{InstallState, ManagedModel, ModelLifecycle};
    use crate::error::ModelLifecycleError;

    struct TempModelsRoot {
        path: PathBuf,
    }

    impl TempModelsRoot {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "voxy-models-tests-{}-{}",
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&path).expect("temporary model root should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempModelsRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn spawn_single_response_server(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("test server should bind loopback port");
        let address = listener
            .local_addr()
            .expect("server should expose local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .expect("server should accept one connection");
            let mut request_buffer = [0_u8; 4096];
            let _ = stream.read(&mut request_buffer);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream
                .write_all(headers.as_bytes())
                .expect("server should write headers");
            stream
                .write_all(&body)
                .expect("server should write response body");
            stream.flush().expect("server should flush stream");
        });
        (format!("http://{address}/model.safetensors"), handle)
    }

    #[test]
    fn install_state_defaults_to_not_downloaded() {
        let temp = TempModelsRoot::new();
        let lifecycle = ModelLifecycle::with_root(temp.path().to_path_buf());

        let state = lifecycle
            .install_state(ManagedModel::WhisperLargeV3Turbo)
            .expect("install state should be readable");

        assert_eq!(state, InstallState::NotDownloaded);
    }

    #[test]
    fn set_install_state_creates_and_removes_model_file() {
        let temp = TempModelsRoot::new();
        let lifecycle = ModelLifecycle::with_root(temp.path().to_path_buf());
        let model = ManagedModel::WhisperLargeV3Turbo;

        lifecycle
            .set_install_state(model, InstallState::Downloaded)
            .expect("downloaded state should be written");
        assert_eq!(
            lifecycle
                .install_state(model)
                .expect("install state should be readable"),
            InstallState::Downloaded
        );

        lifecycle
            .set_install_state(model, InstallState::NotDownloaded)
            .expect("not-downloaded state should remove file");
        assert_eq!(
            lifecycle
                .install_state(model)
                .expect("install state should be readable"),
            InstallState::NotDownloaded
        );
    }

    #[test]
    fn primary_action_downloads_then_removes_with_override_url() {
        let temp = TempModelsRoot::new();
        let model = ManagedModel::WhisperLargeV3Turbo;
        let body = b"dummy-model-bytes".to_vec();
        let (url, server_handle) = spawn_single_response_server(body.clone());

        let mut overrides = HashMap::new();
        overrides.insert(model, url);
        let lifecycle =
            ModelLifecycle::with_root_and_overrides(temp.path().to_path_buf(), overrides);

        let first = lifecycle
            .perform_primary_action(model)
            .expect("primary action should download model");
        assert_eq!(first, InstallState::Downloaded);

        let model_path = temp.path().join(model.artifact_file_name());
        assert!(model_path.exists());
        assert_eq!(
            fs::read(&model_path).expect("downloaded model should be readable"),
            body
        );
        server_handle
            .join()
            .expect("download test server should exit cleanly");

        let second = lifecycle
            .perform_primary_action(model)
            .expect("primary action should remove model");
        assert_eq!(second, InstallState::NotDownloaded);
        assert!(!model_path.exists());
    }

    #[test]
    fn download_reports_progress() {
        let temp = TempModelsRoot::new();
        let model = ManagedModel::WhisperLargeV3Turbo;
        let body = vec![42_u8; 1024 * 8];
        let (url, server_handle) = spawn_single_response_server(body);

        let mut overrides = HashMap::new();
        overrides.insert(model, url);
        let lifecycle =
            ModelLifecycle::with_root_and_overrides(temp.path().to_path_buf(), overrides);

        let mut observed = Vec::new();
        let state = lifecycle
            .perform_primary_action_with_progress(model, |fraction| observed.push(fraction))
            .expect("download should complete");
        assert_eq!(state, InstallState::Downloaded);

        server_handle
            .join()
            .expect("download test server should exit cleanly");

        assert!(!observed.is_empty());
        let first = observed.first().copied().expect("first progress exists");
        let last = observed.last().copied().expect("last progress exists");
        assert!(first >= 0.0 && first <= 1.0);
        assert!((last - 1.0).abs() < 0.0001);
    }

    #[test]
    fn install_state_persists_across_instances() {
        let temp = TempModelsRoot::new();
        let model = ManagedModel::WhisperLargeV3Turbo;

        let lifecycle_a = ModelLifecycle::with_root(temp.path().to_path_buf());
        lifecycle_a
            .set_install_state(model, InstallState::Downloaded)
            .expect("downloaded state should be written");

        let lifecycle_b = ModelLifecycle::with_root(temp.path().to_path_buf());
        let state = lifecycle_b
            .install_state(model)
            .expect("install state should be read by a new instance");
        assert_eq!(state, InstallState::Downloaded);
    }

    #[test]
    fn zero_length_file_is_not_considered_downloaded() {
        let temp = TempModelsRoot::new();
        let model = ManagedModel::WhisperLargeV3Turbo;
        let path = temp.path().join(model.artifact_file_name());
        fs::write(&path, b"").expect("zero-byte file should be created");

        let lifecycle = ModelLifecycle::with_root(temp.path().to_path_buf());
        let state = lifecycle
            .install_state(model)
            .expect("install state should be readable");
        assert_eq!(state, InstallState::NotDownloaded);
    }

    #[test]
    fn missing_data_directory_is_an_error() {
        let lifecycle = ModelLifecycle::default();
        let error = lifecycle
            .install_state(ManagedModel::WhisperLargeV3Turbo)
            .expect_err("install state should fail without configured root");

        assert!(matches!(error, ModelLifecycleError::MissingDataDirectory));
    }
}
