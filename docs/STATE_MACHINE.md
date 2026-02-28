# App State Machine

## States
- `Idle`
- `Recording`
- `Processing`
- `Error(String)`

## ASCII Diagram

```text
                 MicToggled
+------+ ----------------------------> +-----------+
| Idle |                               | Recording |
+------+ <---------------------------- +-----------+
   ^            CommitRequested             |
   |                                        | MicToggled
   | ResetRequested                         v
   |                                 +-------------+
   +-------------------------------- | Processing  |
                                     +-------------+

+-------------+
| Error(msg)  |
+-------------+
      |
      | ResetRequested
      v
    +------+
    | Idle |
    +------+
```

## Explicit Transition Rules
- `Idle + MicToggled -> Recording`
- `Recording + MicToggled -> Processing`
- `Processing + CommitRequested -> Idle`
- `Any State + ResetRequested -> Idle` (except `Error` handling below)
- `Error(_)` only leaves error state on `ResetRequested`
- `RuntimeError(_)`, `ErrorCleared`, `LiveText(_)`, `CopyRequested`, `QuitRequested`, `ShowRequested`, `HideRequested`, and `VisibilityToggled` do not change `AppState`
- `CommitRequested` outside `Processing` does not change `AppState`

## Orthogonal UI Preference State
- Window visibility is tracked in `UiPrefs.visible`.
- `VisibilityToggled`, `ShowRequested`, and `HideRequested` modify `UiPrefs.visible` in `CoreModel`.
- Visibility changes do not mutate `AppState`.

No implicit transitions are allowed.
