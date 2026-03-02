# App Wiring

This document describes non-audio wiring inside `voxy-app`.

## Folder Structure
- `voxy-app/src/app/`
  - `behavior/`: UI-shell behavior primitives (GTK/window side effects)
    - `drag/`
      - `gesture.rs`: installs drag controller and emits direct position updates
      - `hit_test.rs`: blocks drag start over interactive widgets
      - `session.rs`: drag state primitive (active + base margins)
    - `surface/`
      - `layer_shell.rs`: Wayland layer-shell setup and capability checks
      - `placement.rs`: anchored margin placement helpers
    - `resize/`
      - `gesture.rs`: bottom-right resize handle drag wiring
    - `visibility/`
      - `close_request.rs`: close request policy (`close -> hide`)
      - `window_visibility.rs`: show/hide helpers
    - `system/`
      - `clipboard.rs`: clipboard integration
  - `controller.rs`: activation bootstrap and top-level orchestration
  - `lifecycle.rs`: app id and GTK application flags resolution
  - `view_sync.rs`: convert `CoreModel` to `ViewModel` and render
  - `error_path.rs`: explicit helper for mapping runtime failures into `AppState::Error`
- `voxy-app/src/tray/`
  - `status_notifier.rs`: StatusNotifier (system tray) adapter
  - `menu.rs`: tray menu actions (`Show/Hide`, `Reset`, `Size +`, `Size -`, `Quit`)
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
- `app/*` may orchestrate, but should not contain domain transition policy.
- `app/behavior/*` owns GTK/window side effects only; no core state transitions.
- `wiring/*` owns async/process plumbing.
- Domain transitions and state rules belong in `voxy-core`.
- Input-level timer loops may sample/meter audio, but stop decisions must come from `voxy-core` policy APIs.
- Widget controls emit raw user intent (events); persisted policy values are stored in `voxy-core::UiPrefs`.
- Visibility state source of truth is `voxy-core::UiPrefs.visible`.
- Visibility toggles must not rebuild/replace the window.
- Tray callbacks dispatch `AppEvent` only; no domain logic in tray adapters.
