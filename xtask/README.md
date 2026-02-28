# xtask

Workspace task runner for Voxy.

## Category Layout

Tasks are organized by category folders:

- `src/tasks/gui/`: GUI-related automation

Add future categories as sibling folders under `src/tasks/`.

## GUI Tasks

`gui smoke`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Launches the GUI binary
3. Waits briefly for startup
4. Sends a termination signal
5. Verifies the process exits within timeout

`gui lifecycle`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Launches the GUI with smoke ready marker + auto-close hook
3. Verifies startup and clean process exit

`gui reset-flow`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Launches the GUI with reset injection hook + auto-close hook
3. Verifies startup and clean process exit

`gui visibility-toggle-flow`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Launches GUI with synthetic visibility-toggle injection + auto-close hook
3. Verifies startup and clean process exit

`gui visibility-smoke`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Exercises visibility-toggle injection path with clean exit checks

`gui visibility-window-guard`

Behavior:
1. Builds `voxy-app` (unless `--no-build` is passed)
2. Injects repeated visibility toggles and auto-closes
3. Asserts exactly one window-creation marker was emitted
4. Fails if visibility flow recreates/rebuilds the window

## Usage

```bash
cargo run -p xtask -- gui smoke
cargo run -p xtask -- gui lifecycle
cargo run -p xtask -- gui reset-flow
cargo run -p xtask -- gui visibility-toggle-flow
cargo run -p xtask -- gui visibility-smoke
cargo run -p xtask -- gui visibility-window-guard
# or run the full root validation pipeline
just validate
```
