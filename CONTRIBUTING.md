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
just doctor
just hooks-install
cargo fmt --all
cargo check -p voxy-core -p voxy-audio -p voxy-stt
cargo test -p voxy-core
```

To install/check GTK prerequisites:

```bash
just deps
just doctor
```

## Git Hooks

Install repo-managed hooks once per clone:

```bash
just hooks-install
```

Run the pre-commit checks manually:

```bash
just hooks-run
```

`just hooks-run` forces the full pre-commit gate even with no staged files.

Temporary bypass options:
- Skip hook entirely: `VOXY_SKIP_PRECOMMIT=1 git commit ...`
- Skip tests only: `VOXY_PRECOMMIT_SKIP_TESTS=1 git commit ...`

## Pull Requests
- Keep changes focused and reviewable.
- Update docs when behavior, architecture, or boundaries change.
- Include tests for non-trivial domain logic changes.
