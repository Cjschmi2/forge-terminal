# Secrets

## Secret Names and Purposes

No secrets are stored or required by the application itself. The project has no `.env` file, no config file with credentials, and no secret manager integration.

## Env-Var Secret Prevention

The primary security mechanism is **preventing secrets from leaking into child processes**:

- `filter_env()` in `session-pty-core` strips any env var whose name contains: PASSWORD, TOKEN, SECRET, KEY, CREDENTIAL, AUTH, PRIVATE, CERT, API_KEY, ACCESS_KEY
- Two reviewed exceptions: `AGENT_CALLSIGN` (agent identity), `MOTHERDUCK_TOKEN` (DuckDB access)
- Applied in `NativePtySession::spawn()` before `Command::spawn()`

## Secrets in Code Rules

1. Never commit `.env` files, API keys, credentials, or tokens (hard rule #4)
2. No secrets should appear in `knowledge-base/` files
3. The wire protocol `Auth` message carries a JWT token — this must never be logged at debug level
