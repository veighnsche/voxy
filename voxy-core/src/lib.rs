pub mod buffer;
pub mod events;
pub mod model;
pub mod state;
pub mod ui_prefs;

pub use buffer::BufferState;
pub use events::AppEvent;
pub use model::{CoreCommand, CoreModel};
pub use state::{to_error_state, transition, AppState};
pub use ui_prefs::UiPrefs;
