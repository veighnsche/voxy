# Voxy Architecture

## Responsibility Boundaries

| Crate | Owns | Must Not Own |
| --- | --- | --- |
| `voxy-core` | Domain state, reducers, event contract, user-facing policy defaults/clamps, recording stop policy (silence + max duration), transcription model ids | GTK widgets, async runtime orchestration, device/network adapters |
| `voxy-app` | GTK UI composition/rendering, runtime/wiring, platform side effects (window/tray/clipboard), settings persistence adapters | Domain transition logic, stop-policy state machines, provider-specific STT behavior |
| `voxy-stt` | Streaming transcription abstraction + provider adapters (OpenAI realtime/dummy), reconnect/session behavior | App UI state, domain reducer logic |
| `voxy-audio` | Microphone capture/fixture injection, frame routing and level sampling | UI behavior, domain transitions, STT provider policy |

## Dependency Direction

- `voxy-app` depends on `voxy-core`, `voxy-stt`, `voxy-audio`.
- `voxy-core` is dependency-light and does not depend on UI/audio/STT crates.
- `voxy-stt` and `voxy-audio` are adapters and do not depend on `voxy-app`.

## Ownership Rules

- Domain behavior enters through `AppEvent` and is reduced in `CoreModel`.
- GTK callbacks dispatch events only; they do not decide domain transitions.
- Timer/poll loops in `voxy-app` sample inputs and delegate policy evaluation to `voxy-core`.
- Widget atoms can collect raw user input, but persisted policy values live in `voxy-core::UiPrefs`.
- Settings normalization (ranges/defaults/parsing) is defined in `voxy-core` and reused by app adapters.

## Runtime Flow

1. UI/tray/system inputs emit `AppEvent`.
2. `voxy-core::CoreModel::reduce` updates state and returns `CoreCommand` side effects.
3. `voxy-app::wiring::command_bus` executes commands via window/audio/stt/platform adapters.
4. `voxy-stt` emits transcript/runtime events back into the same event channel.
5. `voxy-app::view_sync` renders pure `ViewModel` snapshots.
6. Input-level polling delegates stop-decision policy to `voxy-core` and only dispatches resulting events.

## Boundary Guardrail Checklist

- If a change adds/changes business rules, implement it in `voxy-core` first.
- If a change touches audio network/window side effects, keep it in adapters (`voxy-app`, `voxy-stt`, `voxy-audio`).
- If the same numeric range/default appears in more than one crate, move it to `voxy-core`.
- Keep UI components render/input focused; no reducer-like state machines in widgets.
