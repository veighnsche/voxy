# API Key Ingestion

## Goal
Keep OpenAI API key ingestion deterministic, explicit, and isolated to `voxy-stt`.

`xtask gui stt-e2e` now reuses `voxy-stt` key ingestion as the primary path.

## Lookup Order
1. `VOXY_OPENAI_API_KEY`
2. `VOXY_OPENAI_API_KEY_FILE` (file contents, trimmed)
3. `OPENAI_API_KEY`

## Dev Convenience (`xtask gui stt-e2e`)
If no key is found via normal env lookup, `xtask` falls back to reading:
1. `.env`
2. `.env.local` (overrides `.env`)

Supported keys in dotenv files:
- `VOXY_OPENAI_API_KEY`
- `VOXY_OPENAI_API_KEY_FILE` (relative paths resolve from the dotenv file directory)
- `OPENAI_API_KEY`

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
