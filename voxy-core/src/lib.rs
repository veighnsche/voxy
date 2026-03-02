pub mod buffer;
pub mod config;
pub mod events;
pub mod model;
pub mod recording_stop;
pub mod state;
pub mod transcription;
pub mod ui_prefs;

pub use buffer::BufferState;
pub use config::{
    clamp_silence_auto_stop_seconds, clamp_silence_gate_threshold, clamp_vad_silence_duration_ms,
    parse_max_recording_seconds, parse_silence_auto_stop_seconds, parse_vad_silence_ms,
    DEFAULT_MAX_RECORDING_SECONDS, DEFAULT_SILENCE_AUTO_STOP_SECONDS,
    DEFAULT_SILENCE_GATE_THRESHOLD, DEFAULT_VAD_SILENCE_DURATION_MS, MAX_SILENCE_AUTO_STOP_SECONDS,
    MAX_VAD_SILENCE_DURATION_MS, MIN_VAD_SILENCE_DURATION_MS,
};
pub use events::AppEvent;
pub use model::{CoreCommand, CoreModel};
pub use recording_stop::{visual_input_level, RecordingStopDecision, RecordingStopReason};
pub use state::{to_error_state, transition, AppState};
pub use transcription::TranscriptionModelId;
pub use ui_prefs::UiPrefs;
