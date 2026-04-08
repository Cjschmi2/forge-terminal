/**
 * Wire protocol token type constants.
 * @wire-protocol-dev @token-id-parity
 *
 * Each constant maps to the single-byte token_type field in the 21-byte
 * wire frame header. Values MUST match the Rust `TokenType` discriminant
 * in protocol/backend/crates/protocol-wire-core/src/token.rs.
 *
 * --- IMPORTANT ---
 * Do NOT change values here without verifying the Rust enum first.
 * Rust is the source of truth. See B-token-id-consistency.
 */
export const TOKEN = {
  // ── Standard LLM Streaming (0x00-0x0F) ──────────────────────────────

  /** UTF-8 content fragment (LLM output, general text). */
  CONTENT: 0x00,
  /** Role indicator for the message stream. */
  ROLE: 0x01,
  /** Tool invocation request. */
  TOOL_CALL: 0x02,
  /** Tool invocation result. */
  TOOL_RESULT: 0x03,
  /** Stream completed normally. */
  DONE: 0x04,
  /** Unrecoverable error on this stream. */
  ERROR: 0x05,
  /** Metadata about the current stream or message. */
  METADATA: 0x06,
  /** Thinking / chain-of-thought trace. */
  THINKING: 0x09,
  /** Thinking phase completed. */
  THINKING_DONE: 0x0a,
  /** System-level message injected by the platform. */
  SYSTEM_MESSAGE: 0x0c,

  // ── Orchestration / Multi-Agent (0x10-0x1F) ─────────────────────────

  /** A new agent has been spawned. */
  AGENT_SPAWN: 0x11,
  /** An agent has completed its work. */
  AGENT_COMPLETE: 0x12,
  /** Agent-level conversational message. */
  AGENT_MESSAGE: 0x16,
  /** Coordination checkpoint across agents. */
  COORDINATION_CHECKPOINT: 0x17,

  // ── Skills + Terminal (0x20-0x2F) ────────────────────────────────────

  /** Skill execution progress update. */
  SKILL_PROGRESS: 0x21,
  /** Terminal output captured from a PTY mirror. */
  TERMINAL_OUTPUT: 0x26,
  /** Keyboard / paste input flowing *into* a terminal session. */
  TERMINAL_INPUT: 0x2b,

  // ── UI Control (0x30-0x3F) ───────────────────────────────────────────

  /** Notification destined for the operator UI. */
  UI_NOTIFICATION: 0x32,
  /** Progress update destined for the operator UI. */
  UI_PROGRESS: 0x33,

  // ── PTY System (0x78-0x7A) ───────────────────────────────────────────

  /** Raw PTY data (stdout bytes from a session). */
  PTY_DATA: 0x78,
  /** PTY control signal (resize, signal, etc.). */
  PTY_CONTROL: 0x79,
  /** PTY status update (started, exited, error). */
  PTY_STATUS: 0x7a,

  // ── Session Protocol (0xAB-0xAE) ─────────────────────────────────────

  /** Session-level command (launch, stop, attach, etc.). */
  SESSION_COMMAND: 0xab,
  /** Session heartbeat / keepalive. */
  SESSION_HEARTBEAT: 0xae,
} as const;

export type TokenType = (typeof TOKEN)[keyof typeof TOKEN];

const TOKEN_NAMES: Record<number, string> = Object.fromEntries(
  Object.entries(TOKEN).map(([name, value]) => [value, name]),
);

/** Human-readable name for a token type byte, or `UNKNOWN(0xNN)`. */
export function tokenName(type: number): string {
  return TOKEN_NAMES[type] ?? `UNKNOWN(0x${type.toString(16).padStart(2, '0')})`;
}

/**
 * True when the token type represents data that should render in a
 * terminal emulator (PTY output, terminal mirror, or raw content).
 */
export function isTerminal(type: number): boolean {
  return (
    type === TOKEN.PTY_DATA ||
    type === TOKEN.TERMINAL_OUTPUT ||
    type === TOKEN.CONTENT
  );
}

/**
 * True when the token type signals that a stream has terminated.
 * Matches the Rust `TokenType::is_terminal_token()` semantics.
 */
export function isStreamTerminator(type: number): boolean {
  return (
    type === TOKEN.DONE ||
    type === TOKEN.ERROR ||
    type === TOKEN.AGENT_COMPLETE
  );
}
