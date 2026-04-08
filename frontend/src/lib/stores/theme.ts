import { writable, get } from 'svelte/store';
import { settingsLoad, settingsSave } from '$lib/tauri';

export interface ThemeSettings {
  // Terminal
  termFontFamily: string;
  termFontSize: number;
  termForeground: string;
  termBackground: string;
  termCursor: string;
  termSelection: string;
  termOpacity: number;

  // UI chrome
  uiFontFamily: string;
  uiFontSize: number;
  bgBase: string;
  bgSurface: string;
  bgElevated: string;
  borderColor: string;
  textPrimary: string;
  textMuted: string;
  accent: string;

  // File tree
  treeFontSize: number;
  treeFontFamily: string;
  treeBg: string;
  treeText: string;
  treeDirColor: string;

  // ANSI palette (xterm.js 16-color)
  ansiBlack: string;
  ansiRed: string;
  ansiGreen: string;
  ansiYellow: string;
  ansiBlue: string;
  ansiMagenta: string;
  ansiCyan: string;
  ansiWhite: string;
  ansiBrightBlack: string;
  ansiBrightRed: string;
  ansiBrightGreen: string;
  ansiBrightYellow: string;
  ansiBrightBlue: string;
  ansiBrightMagenta: string;
  ansiBrightCyan: string;
  ansiBrightWhite: string;
}

export const defaults: ThemeSettings = {
  termFontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', ui-monospace, monospace",
  termFontSize: 14,
  termForeground: '#d4daf0',
  termBackground: '#080c18',
  termCursor: '#5eead4',
  termSelection: 'rgba(99, 102, 241, 0.3)',
  termOpacity: 88,

  uiFontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  uiFontSize: 13,
  bgBase: '#0a0e1a',
  bgSurface: '#111628',
  bgElevated: '#1a2038',
  borderColor: '#1e2640',
  textPrimary: '#d0d6e8',
  textMuted: '#8890a8',
  accent: '#7c8af7',

  treeFontSize: 12,
  treeFontFamily: "'JetBrains Mono', 'Fira Code', ui-monospace, monospace",
  treeBg: '#0e1224',
  treeText: '#b8bfd4',
  treeDirColor: '#7c8af7',

  ansiBlack: '#1e293b',
  ansiRed: '#f87171',
  ansiGreen: '#4ade80',
  ansiYellow: '#fbbf24',
  ansiBlue: '#60a5fa',
  ansiMagenta: '#c084fc',
  ansiCyan: '#22d3ee',
  ansiWhite: '#f1f5f9',
  ansiBrightBlack: '#8892a8',
  ansiBrightRed: '#fca5a5',
  ansiBrightGreen: '#86efac',
  ansiBrightYellow: '#fde68a',
  ansiBrightBlue: '#93c5fd',
  ansiBrightMagenta: '#d8b4fe',
  ansiBrightCyan: '#67e8f9',
  ansiBrightWhite: '#f8fafc',
};

export const theme = writable<ThemeSettings>({ ...defaults });

function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace('#', '');
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Apply theme settings to CSS custom properties on :root */
export function applyTheme(t: ThemeSettings) {
  const root = document.documentElement;
  const alpha = t.termOpacity / 100;

  root.style.setProperty('--font-mono', t.termFontFamily);
  root.style.setProperty('--term-font-size', `${t.termFontSize}px`);
  root.style.setProperty('--term-fg', t.termForeground);
  root.style.setProperty('--term-bg', t.termBackground);
  root.style.setProperty('--term-cursor', t.termCursor);
  root.style.setProperty('--term-selection', t.termSelection);
  root.style.setProperty('--term-opacity', `${alpha}`);

  root.style.setProperty('--ui-font', t.uiFontFamily);
  root.style.setProperty('--ui-font-size', `${t.uiFontSize}px`);

  // Apply opacity to all background surfaces for see-through effect
  root.style.setProperty('--bg-base', hexToRgba(t.bgBase, alpha));
  root.style.setProperty('--bg-surface', hexToRgba(t.bgSurface, alpha));
  root.style.setProperty('--bg-elevated', hexToRgba(t.bgElevated, alpha));
  root.style.setProperty('--border', hexToRgba(t.borderColor, Math.min(1, alpha + 0.1)));
  root.style.setProperty('--text-primary', t.textPrimary);
  root.style.setProperty('--text-muted', t.textMuted);
  root.style.setProperty('--accent', t.accent);

  root.style.setProperty('--tree-font-size', `${t.treeFontSize}px`);
  root.style.setProperty('--tree-font', t.treeFontFamily);
  root.style.setProperty('--tree-bg', hexToRgba(t.treeBg, alpha));
  root.style.setProperty('--tree-text', t.treeText);
  root.style.setProperty('--tree-dir-color', t.treeDirColor);

  // Tell the HTML element to be transparent for Tauri window compositing
  root.style.setProperty('background', alpha < 1 ? 'transparent' : hexToRgba(t.bgBase, 1));
  document.body.style.background = alpha < 1 ? 'transparent' : hexToRgba(t.bgBase, 1);
}

let subscribed = false;

/** Load settings from disk, apply, and subscribe for future changes */
export async function initTheme() {
  try {
    const raw = await settingsLoad();
    const saved = JSON.parse(raw);
    if (saved && typeof saved === 'object') {
      const merged = { ...defaults, ...saved };
      theme.set(merged);
    }
  } catch {
    // defaults already in store
  }

  // Auto-apply on any change — subscribe once
  if (!subscribed) {
    subscribed = true;
    theme.subscribe((t) => {
      if (typeof document !== 'undefined') {
        applyTheme(t);
      }
    });
  }
}

/** Save current settings to disk */
export async function saveTheme() {
  const current = get(theme);
  await settingsSave(JSON.stringify(current, null, 2));
}
