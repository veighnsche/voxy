# voxy-core

Core domain module for Voxy.

## Contains
- `BufferState` model
- `AppState` state machine and pure transitions
- `AppEvent` contract
- `UiPrefs` non-stream UI preferences (for example pin state)
- runtime UI error message state (`runtime_error`) for transient app-layer failures
- `CoreModel` reducer and `CoreCommand` side-effect plan

All app behavior should be expressed here first.
