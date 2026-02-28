# Contributing

## Scope
Voxy is currently scaffold-first. Contributions should preserve strict module boundaries:
- `voxy-core`: domain logic (state machine, buffer model, reducer)
- `voxy-app`: UI wiring and rendering only
- `voxy-stt`: transcriber abstraction and implementation
- `voxy-audio`: audio abstraction and implementation

## Ground Rules
- Keep behavior in `voxy-core`.
- Keep GTK callbacks thin (dispatch/render only).
- Avoid hidden state and implicit destructive actions.
- Do not introduce DBus, portals, global shortcuts, or background daemons unless explicitly planned.

## Development
```bash
cargo fmt --all
cargo check -p voxy-core -p voxy-audio -p voxy-stt
cargo test -p voxy-core
```

To compile `voxy-app`, install GTK4 development libraries and ensure `pkg-config` can find `gtk4.pc`.

## Pull Requests
- Keep changes focused and reviewable.
- Update docs when behavior, architecture, or boundaries change.
- Include tests for non-trivial domain logic changes.
