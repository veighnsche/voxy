# voxy-app

GTK4 application shell for Voxy.

## Responsibilities
- Build widgets and window layout
- Dispatch UI actions as `AppEvent`
- Execute side-effect commands produced by `voxy-core`
- Render view state

## Internal Layout
- `src/app/`: application orchestration and lifecycle boundaries
  - Includes Wayland pin backend adapter (`pin_backend.rs`)
- `src/wiring/`: channels, runtime, event pump, command execution
- `src/ui/`: atomic UI components and render-only composition
- `src/diagnostics/`: opt-in smoke test hooks

## Non-Responsibilities
- No business/domain rules in GTK callbacks
- No direct STT logic
- No direct audio pipeline logic
