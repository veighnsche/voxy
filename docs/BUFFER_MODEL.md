# Buffer Model

## Structure

```rust
struct BufferState {
    confirmed_text: String,
    live_segment: String,
}
```

## Meaning
- `confirmed_text`: Stable text the user can edit.
- `live_segment`: Ephemeral uncommitted streaming tail.

## Rules
- Streaming modifies `live_segment` only.
- Streaming does not mutate `confirmed_text` directly.
- User edits affect `confirmed_text`.
- `commit_live()` appends `live_segment` to `confirmed_text` and clears `live_segment`.
- `reset_all()` clears both fields.
- Rendered text is always `confirmed_text + live_segment`.

## Operations
- `append_live(&mut self, text: &str)`
- `commit_live(&mut self)`
- `clear_live(&mut self)`
- `reset_all(&mut self)`
- `full_text(&self) -> String`
