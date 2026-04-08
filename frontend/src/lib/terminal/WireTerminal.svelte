<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import 'xterm/css/xterm.css';
  import type { WireClient } from '$lib/wire/client';
  import { TOKEN } from '$lib/wire/tokens';
  import type { TokenFrame } from '$lib/wire/frame';

  /** The wire client that delivers PTY_DATA / TERMINAL_OUTPUT frames. */
  export let client: WireClient;

  /** Display label shown in the shell chrome. */
  export let sessionName = '';

  /** Optional stream ID filter — 0 means accept all streams. */
  export let streamId = 0;

  let terminalEl: HTMLDivElement;
  let term: import('xterm').Terminal | null = null;
  let fitAddon: import('xterm-addon-fit').FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let unsubFrame: (() => void) | null = null;
  let unsubConnect: (() => void) | null = null;
  let inputSequence = 0;

  onMount(async () => {
    const { Terminal } = await import('xterm');
    const { FitAddon } = await import('xterm-addon-fit');

    term = new Terminal({
      cursorBlink: true,
      fontFamily: '"SFMono-Regular", "JetBrains Mono", ui-monospace, monospace',
      fontSize: 13,
      lineHeight: 1.35,
      theme: {
        background: '#0f172a',
        foreground: '#e2e8f0',
        cursor: '#38bdf8',
        selectionBackground: 'rgba(56, 189, 248, 0.28)',
        black: '#1e293b',
        red: '#f87171',
        green: '#4ade80',
        yellow: '#fbbf24',
        blue: '#60a5fa',
        magenta: '#c084fc',
        cyan: '#22d3ee',
        white: '#f1f5f9',
        brightBlack: '#475569',
        brightRed: '#fca5a5',
        brightGreen: '#86efac',
        brightYellow: '#fde68a',
        brightBlue: '#93c5fd',
        brightMagenta: '#d8b4fe',
        brightCyan: '#67e8f9',
        brightWhite: '#f8fafc',
      },
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalEl);

    // Fit once the element has a layout.
    requestAnimationFrame(() => {
      fitAddon?.fit();
    });

    // Re-fit when the container resizes.
    resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        fitAddon?.fit();
      });
    });
    resizeObserver.observe(terminalEl);

    // Wire frames → terminal output.
    const textDecoder = new TextDecoder();
    unsubFrame = client.onFrame((frame: TokenFrame) => {
      if (streamId !== 0 && frame.streamId !== streamId) return;
      if (
        frame.tokenType === TOKEN.PTY_DATA ||
        frame.tokenType === TOKEN.TERMINAL_OUTPUT ||
        frame.tokenType === TOKEN.CONTENT
      ) {
        term?.write(textDecoder.decode(frame.data));
      }
      if (frame.tokenType === TOKEN.ERROR) {
        term?.write(`\r\n\x1b[31m[ERROR] ${textDecoder.decode(frame.data)}\x1b[0m\r\n`);
      }
      if (frame.tokenType === TOKEN.DONE) {
        term?.write('\r\n\x1b[90m--- stream ended ---\x1b[0m\r\n');
      }
    });

    // Terminal keyboard input → wire frames.
    const textEncoder = new TextEncoder();
    term.onData((data: string) => {
      inputSequence++;
      client.send({
        tokenType: TOKEN.TERMINAL_INPUT,
        streamId: streamId || 0,
        sequence: inputSequence,
        timestampNs: BigInt(Date.now()) * 1_000_000n,
        data: textEncoder.encode(data),
      });
    });

    // Show a reconnect notice when the wire client reconnects.
    unsubConnect = client.onConnect(() => {
      term?.write('\r\n\x1b[33m[reconnected]\x1b[0m\r\n');
    });
  });

  onDestroy(() => {
    unsubFrame?.();
    unsubConnect?.();
    resizeObserver?.disconnect();
    term?.dispose();
    term = null;
    fitAddon = null;
  });
</script>

<div class="wire-terminal-container">
  {#if sessionName}
    <div class="wire-terminal-bar">
      <div class="wire-terminal-dots" aria-hidden="true">
        <span></span><span></span><span></span>
      </div>
      <strong class="wire-terminal-title">{sessionName}</strong>
      <span class="wire-terminal-badge">{client.connected ? 'live' : 'disconnected'}</span>
    </div>
  {/if}
  <div bind:this={terminalEl} class="wire-terminal-body"></div>
</div>

<style>
  .wire-terminal-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    border-radius: 0.75rem;
    overflow: hidden;
    background: #0f172a;
  }

  .wire-terminal-bar {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.5rem 0.85rem;
    background: #1e293b;
    border-bottom: 1px solid rgba(148, 163, 184, 0.12);
    flex-shrink: 0;
  }

  .wire-terminal-dots {
    display: flex;
    gap: 0.35rem;
  }

  .wire-terminal-dots span {
    display: block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: rgba(148, 163, 184, 0.22);
  }

  .wire-terminal-dots span:first-child {
    background: #f87171;
  }
  .wire-terminal-dots span:nth-child(2) {
    background: #fbbf24;
  }
  .wire-terminal-dots span:nth-child(3) {
    background: #4ade80;
  }

  .wire-terminal-title {
    flex: 1;
    font-size: 0.78rem;
    color: #94a3b8;
    font-family: var(--font-mono, monospace);
    letter-spacing: 0.02em;
  }

  .wire-terminal-badge {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    background: rgba(74, 222, 128, 0.14);
    color: #4ade80;
  }

  .wire-terminal-body {
    flex: 1;
    min-height: 0;
    padding: 0.35rem;
  }

  /* xterm.js injects its own stylesheet; these overrides keep it flush. */
  .wire-terminal-body :global(.xterm) {
    height: 100%;
  }
  .wire-terminal-body :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
