# App Wiring

This document describes non-audio wiring inside `voxy-app`.

## Folder Structure
- `voxy-app/src/app/`
  - `controller.rs`: activation bootstrap and top-level orchestration
  - `lifecycle.rs`: app id and GTK application flags resolution
  - `view_sync.rs`: convert `CoreModel` to `ViewModel` and render
  - `error_path.rs`: explicit helper for mapping runtime failures into `AppState::Error`
  - `window/`: window-only side effects
    - `layer_shell.rs`: Wayland layer-shell setup
    - `visibility.rs`: show/hide helpers
    - `close_policy.rs`: close request policy (`close -> hide`)
    - `drag.rs`: drag-handle move behavior
    - `clipboard.rs`: clipboard integration
- `voxy-app/src/tray/`
  - `status_notifier.rs`: StatusNotifier (system tray) adapter
  - `menu.rs`: tray menu actions (`Show/Hide`, `Reset`, `Quit`)
  - `mod.rs`: tray runtime lifecycle
- `voxy-app/src/wiring/`
  - `runtime.rs`: Tokio runtime construction
  - `channels.rs`: `tokio::mpsc` channel construction
  - `event_loop.rs`: GTK tick-based event drain loop
  - `command_bus.rs`: execute `CoreCommand` side effects
- `voxy-app/src/diagnostics/`
  - `smoke_hooks.rs`: environment-driven smoke-test hooks
- `voxy-app/src/ui/`
  - Atoms/molecules/organisms/templates/pages + `ViewModel`
  - No business logic
- `xtask/src/tasks/gui/`
  - `smoke.rs`: launch, verify running, SIGTERM, verify shutdown
  - `lifecycle.rs`: launch with auto-close hook and verify clean exit
  - `reset_flow.rs`: inject reset event + auto-close and verify clean exit
  - `visibility_toggle_flow.rs`: inject visibility toggle + auto-close and verify clean exit
  - `visibility_smoke.rs`: extra visibility toggle smoke path
  - `visibility_window_guard.rs`: repeated visibility toggles with one-window invariant check
  - `common.rs`: shared process helpers

## Guardrails
- `ui/*` must stay render-only.
- `app/*` may orchestrate, but should not contain domain transitions.
- `wiring/*` owns async/process plumbing.
- Domain transitions and state rules belong in `voxy-core`.
- Visibility state source of truth is `voxy-core::UiPrefs.visible`.
- Visibility toggles must not rebuild/replace the window.
- Tray callbacks dispatch `AppEvent` only; no domain logic in tray adapters.
