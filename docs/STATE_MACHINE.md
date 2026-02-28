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
- `Any State + ResetRequested -> Idle`
- `LiveText(_)` does not change state
- `CommitRequested` outside `Processing` does not change state
- `Error(_)` only leaves error state on `ResetRequested`

No implicit transitions are allowed.
