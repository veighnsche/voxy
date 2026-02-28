# API Key Ingestion

## Goal
Keep OpenAI API key ingestion deterministic, explicit, and isolated to `voxy-stt`.

## Lookup Order
1. `VOXY_OPENAI_API_KEY`
2. `VOXY_OPENAI_API_KEY_FILE` (file contents, trimmed)
3. `OPENAI_API_KEY`

## Rules
- `voxy-core` must not store secrets.
- `voxy-app` UI must not read or display raw API keys.
- Missing key must surface as an explicit runtime error event.
- Never print key material in logs.

## Failure Modes
- Missing env/file: return `MissingApiKey`.
- File unreadable: return file read error with path.
- File empty after trimming: return file empty error.

## Future Hardening
- Prefer short-lived ephemeral tokens for production deployments.
