# API Key Ingestion

## Goal
Keep OpenAI API key ingestion deterministic, explicit, and isolated to `voxy-stt`.

Runtime callers use `voxy-stt::config::load_api_key()`.

## Lookup Order
1. `VOXY_OPENAI_API_KEY`
2. `VOXY_OPENAI_API_KEY_FILE` (file contents, trimmed)
3. `OPENAI_API_KEY`
4. `.env`
5. `.env.local` (overrides `.env`)

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
