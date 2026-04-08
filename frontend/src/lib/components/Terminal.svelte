<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ptySend, ptyResize, onPtyOutput, type PtyOutputEvent } from '$lib/tauri';
  import { theme, type ThemeSettings } from '$lib/stores/theme';
  import 'xterm/css/xterm.css';

  export let sessionName: string;

  let terminalEl: HTMLDivElement;
  let term: import('xterm').Terminal | null = null;
  let fitAddon: import('xterm-addon-fit').FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let unlisten: (() => void) | null = null;
  let windowUnsub: (() => void) | null = null;
  let themeUnsub: (() => void) | null = null;

  function doFit() {
    requestAnimationFrame(() => {
      if (!fitAddon || !term) return;
      try {
        fitAddon.fit();
        ptyResize(sessionName, term.cols, term.rows).catch(() => {});
      } catch {}
    });
  }

  function hexToRgba(hex: string, alpha: number): string {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  function applyTermTheme(t: ThemeSettings) {
    if (!term) return;
    const alpha = t.termOpacity / 100;
    term.options.fontFamily = t.termFontFamily;
    term.options.fontSize = t.termFontSize;
    term.options.theme = {
      background: hexToRgba(t.termBackground, alpha),
      foreground: t.termForeground,
      cursor: t.termCursor,
      selectionBackground: t.termSelection,
      black: t.ansiBlack,
      red: t.ansiRed,
      green: t.ansiGreen,
      yellow: t.ansiYellow,
      blue: t.ansiBlue,
      magenta: t.ansiMagenta,
      cyan: t.ansiCyan,
      white: t.ansiWhite,
      brightBlack: t.ansiBrightBlack,
      brightRed: t.ansiBrightRed,
      brightGreen: t.ansiBrightGreen,
      brightYellow: t.ansiBrightYellow,
      brightBlue: t.ansiBrightBlue,
      brightMagenta: t.ansiBrightMagenta,
      brightCyan: t.ansiBrightCyan,
      brightWhite: t.ansiBrightWhite,
    };
    doFit();
  }

  onMount(async () => {
    const { Terminal } = await import('xterm');
    const { FitAddon } = await import('xterm-addon-fit');

    const t = $theme;
    term = new Terminal({
      cursorBlink: true,
      fontFamily: t.termFontFamily,
      fontSize: t.termFontSize,
      lineHeight: 1.3,
      scrollback: 10000,
      allowProposedApi: true,
      theme: {
        background: t.termBackground,
        foreground: t.termForeground,
        cursor: t.termCursor,
        selectionBackground: t.termSelection,
        black: t.ansiBlack,
        red: t.ansiRed,
        green: t.ansiGreen,
        yellow: t.ansiYellow,
        blue: t.ansiBlue,
        magenta: t.ansiMagenta,
        cyan: t.ansiCyan,
        white: t.ansiWhite,
        brightBlack: t.ansiBrightBlack,
        brightRed: t.ansiBrightRed,
        brightGreen: t.ansiBrightGreen,
        brightYellow: t.ansiBrightYellow,
        brightBlue: t.ansiBrightBlue,
        brightMagenta: t.ansiBrightMagenta,
        brightCyan: t.ansiBrightCyan,
        brightWhite: t.ansiBrightWhite,
      },
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalEl);

    setTimeout(doFit, 50);
    setTimeout(doFit, 200);

    resizeObserver = new ResizeObserver(doFit);
    resizeObserver.observe(terminalEl);

    const onWindowResize = () => doFit();
    window.addEventListener('resize', onWindowResize);
    windowUnsub = () => window.removeEventListener('resize', onWindowResize);

    // Subscribe to theme changes — update xterm live
    themeUnsub = theme.subscribe((newTheme) => {
      applyTermTheme(newTheme);
    });

    // PTY output from Rust -> xterm.js
    const unlistenFn = await onPtyOutput((event: PtyOutputEvent) => {
      if (event.session_name === sessionName) {
        term?.write(new Uint8Array(event.data));
      }
    });
    unlisten = unlistenFn;

    // Keyboard input -> Rust PTY
    term.onData((data: string) => {
      ptySend(sessionName, data).catch(() => {});
    });

    // Paste support
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.ctrlKey && (e.key === 'v' || e.key === 'V') && e.type === 'keydown') {
        navigator.clipboard.readText().then((text) => {
          if (text) ptySend(sessionName, text).catch(() => {});
        });
        return false;
      }
      return true;
    });
  });

  onDestroy(() => {
    themeUnsub?.();
    unlisten?.();
    windowUnsub?.();
    resizeObserver?.disconnect();
    term?.dispose();
  });
</script>

<div bind:this={terminalEl} class="terminal-pane"></div>

<style>
  .terminal-pane {
    flex: 1;
    min-height: 0;
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    position: relative;
  }

  .terminal-pane :global(.xterm) {
    height: 100%;
    width: 100%;
    padding: 4px;
    position: absolute;
    inset: 0;
  }

  .terminal-pane :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
