# Roadmap

## Phase 1: Stub Working
- Workspace and module boundaries
- GTK shell UI controls
- Channel-based event loop
- Dummy streaming transcriber
- No-op audio input

## Phase 2: Real Audio
- Replace no-op audio with actual capture pipeline
- Keep trait boundary unchanged

## Phase 3: Real Streaming STT
- Replace dummy transcriber with OpenAI streaming implementation
- Keep `StreamingTranscriber` interface stable where practical
- Follow `docs/STT_CONNECTION_PLAN.md` protocol mapping and sequencing

## Phase 4: Tail Reconciliation
- Improve live tail behavior (partial/final chunk reconciliation)
- Preserve editability and deterministic buffer rules

## Phase 5: Error Handling
- Structured error propagation
- Better error display and recovery paths
- Retries and failure-safe transitions
