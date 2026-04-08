<script lang="ts">
  import { theme, defaults, saveTheme, type ThemeSettings } from '$lib/stores/theme';

  export let onClose: () => void = () => {};

  let t: ThemeSettings = { ...$theme };
  let activeSection = 'terminal';

  const sections = [
    { id: 'terminal', label: 'Terminal' },
    { id: 'interface', label: 'Interface' },
    { id: 'filetree', label: 'File Tree' },
  ];

  async function save() {
    theme.set({ ...t });
    await saveTheme();
    onClose();
  }

  function reset() {
    t = { ...defaults };
    theme.set({ ...defaults });
  }

  // Live preview
  $: theme.set({ ...t });
</script>

<div class="overlay" on:click={onClose} on:keydown={() => {}} role="presentation">
  <div class="panel" on:click|stopPropagation on:keydown|stopPropagation role="dialog">

    <!-- Header -->
    <div class="header">
      <h2>Settings</h2>
      <button class="close" on:click={onClose}>&times;</button>
    </div>

    <!-- Section tabs -->
    <div class="tabs">
      {#each sections as s}
        <button
          class="tab"
          class:active={activeSection === s.id}
          on:click={() => activeSection = s.id}
        >{s.label}</button>
      {/each}
    </div>

    <!-- Content -->
    <div class="body">

      {#if activeSection === 'terminal'}
        <div class="row">
          <div class="field full">
            <span class="label">Font</span>
            <select bind:value={t.termFontFamily}>
              <option value="'JetBrains Mono', ui-monospace, monospace">JetBrains Mono</option>
              <option value="'SF Mono', ui-monospace, monospace">SF Mono</option>
              <option value="'Cascadia Code', ui-monospace, monospace">Cascadia Code</option>
              <option value="'Fira Code', ui-monospace, monospace">Fira Code</option>
              <option value="'Source Code Pro', ui-monospace, monospace">Source Code Pro</option>
              <option value="ui-monospace, monospace">System Mono</option>
            </select>
          </div>
        </div>

        <div class="row">
          <div class="field">
            <span class="label">Size</span>
            <div class="stepper">
              <button on:click={() => t.termFontSize = Math.max(8, t.termFontSize - 1)}>-</button>
              <span class="stepper-val">{t.termFontSize}px</span>
              <button on:click={() => t.termFontSize = Math.min(32, t.termFontSize + 1)}>+</button>
            </div>
          </div>
          <div class="field">
            <span class="label">Opacity</span>
            <div class="slider-row">
              <input type="range" bind:value={t.termOpacity} min="30" max="100" />
              <span class="slider-val">{t.termOpacity}%</span>
            </div>
          </div>
        </div>

        <div class="row">
          <div class="field">
            <span class="label">Text</span>
            <button class="swatch-btn" style="--swatch: {t.termForeground}">
              <span class="swatch"></span>
              <input type="color" bind:value={t.termForeground} />
              <span class="hex">{t.termForeground}</span>
            </button>
          </div>
          <div class="field">
            <span class="label">Background</span>
            <button class="swatch-btn" style="--swatch: {t.termBackground}">
              <span class="swatch"></span>
              <input type="color" bind:value={t.termBackground} />
              <span class="hex">{t.termBackground}</span>
            </button>
          </div>
        </div>

        <div class="row">
          <div class="field">
            <span class="label">Cursor</span>
            <button class="swatch-btn" style="--swatch: {t.termCursor}">
              <span class="swatch"></span>
              <input type="color" bind:value={t.termCursor} />
              <span class="hex">{t.termCursor}</span>
            </button>
          </div>
        </div>

        <!-- ANSI Palette -->
        <details class="ansi-details">
          <summary class="ansi-summary">ANSI Colors</summary>
          <div class="color-grid ansi-grid">
            {#each [
              { label: 'Black', key: 'ansiBlack' },
              { label: 'Red', key: 'ansiRed' },
              { label: 'Green', key: 'ansiGreen' },
              { label: 'Yellow', key: 'ansiYellow' },
              { label: 'Blue', key: 'ansiBlue' },
              { label: 'Magenta', key: 'ansiMagenta' },
              { label: 'Cyan', key: 'ansiCyan' },
              { label: 'White', key: 'ansiWhite' },
              { label: 'Bright Black', key: 'ansiBrightBlack' },
              { label: 'Bright Red', key: 'ansiBrightRed' },
              { label: 'Bright Green', key: 'ansiBrightGreen' },
              { label: 'Bright Yellow', key: 'ansiBrightYellow' },
              { label: 'Bright Blue', key: 'ansiBrightBlue' },
              { label: 'Bright Magenta', key: 'ansiBrightMagenta' },
              { label: 'Bright Cyan', key: 'ansiBrightCyan' },
              { label: 'Bright White', key: 'ansiBrightWhite' },
            ] as item}
              <div class="color-cell">
                <span class="label">{item.label}</span>
                <button class="swatch-btn" style="--swatch: {t[item.key]}">
                  <span class="swatch"></span>
                  <input type="color" bind:value={t[item.key]} />
                  <span class="hex">{t[item.key]}</span>
                </button>
              </div>
            {/each}
          </div>
        </details>

        <!-- Preview -->
        <div class="preview" style="background: {t.termBackground}; color: {t.termForeground}; font-family: {t.termFontFamily}; font-size: {t.termFontSize}px; opacity: {t.termOpacity / 100};">
          <div>$ claude --version</div>
          <div style="color: {t.ansiGreen};">Claude Code v2.1.94</div>
          <div style="color: {t.ansiBrightBlack};">// dim comment from AI tool</div>
          <div>$ <span style="color: {t.termCursor};">|</span></div>
        </div>
      {/if}

      {#if activeSection === 'interface'}
        <div class="row">
          <div class="field full">
            <span class="label">Font</span>
            <select bind:value={t.uiFontFamily}>
              <option value="-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif">System Sans</option>
              <option value="'Inter', sans-serif">Inter</option>
              <option value="'JetBrains Mono', monospace">JetBrains Mono</option>
            </select>
          </div>
        </div>

        <div class="row">
          <div class="field">
            <span class="label">Size</span>
            <div class="stepper">
              <button on:click={() => t.uiFontSize = Math.max(10, t.uiFontSize - 1)}>-</button>
              <span class="stepper-val">{t.uiFontSize}px</span>
              <button on:click={() => t.uiFontSize = Math.min(20, t.uiFontSize + 1)}>+</button>
            </div>
          </div>
        </div>

        <div class="color-grid">
          {#each [
            { label: 'Background', key: 'bgBase' },
            { label: 'Surface', key: 'bgSurface' },
            { label: 'Elevated', key: 'bgElevated' },
            { label: 'Border', key: 'borderColor' },
            { label: 'Text', key: 'textPrimary' },
            { label: 'Muted', key: 'textMuted' },
            { label: 'Accent', key: 'accent' },
          ] as item}
            <div class="color-cell">
              <span class="label">{item.label}</span>
              <button class="swatch-btn" style="--swatch: {t[item.key]}">
                <span class="swatch"></span>
                <input type="color" bind:value={t[item.key]} />
                <span class="hex">{t[item.key]}</span>
              </button>
            </div>
          {/each}
        </div>
      {/if}

      {#if activeSection === 'filetree'}
        <div class="row">
          <div class="field full">
            <span class="label">Font</span>
            <select bind:value={t.treeFontFamily}>
              <option value="'JetBrains Mono', ui-monospace, monospace">JetBrains Mono</option>
              <option value="'SF Mono', ui-monospace, monospace">SF Mono</option>
              <option value="ui-monospace, monospace">System Mono</option>
            </select>
          </div>
        </div>

        <div class="row">
          <div class="field">
            <span class="label">Size</span>
            <div class="stepper">
              <button on:click={() => t.treeFontSize = Math.max(8, t.treeFontSize - 1)}>-</button>
              <span class="stepper-val">{t.treeFontSize}px</span>
              <button on:click={() => t.treeFontSize = Math.min(20, t.treeFontSize + 1)}>+</button>
            </div>
          </div>
        </div>

        <div class="color-grid">
          {#each [
            { label: 'Background', key: 'treeBg' },
            { label: 'Text', key: 'treeText' },
            { label: 'Directories', key: 'treeDirColor' },
          ] as item}
            <div class="color-cell">
              <span class="label">{item.label}</span>
              <button class="swatch-btn" style="--swatch: {t[item.key]}">
                <span class="swatch"></span>
                <input type="color" bind:value={t[item.key]} />
                <span class="hex">{t[item.key]}</span>
              </button>
            </div>
          {/each}
        </div>

        <!-- Tree preview -->
        <div class="preview" style="background: {t.treeBg}; color: {t.treeText}; font-family: {t.treeFontFamily}; font-size: {t.treeFontSize}px;">
          <div>
            <span style="color: var(--text-muted);">▸</span>
            <span style="color: {t.treeDirColor};">📁 src</span>
          </div>
          <div>
            <span style="color: var(--text-muted);">▸</span>
            <span style="color: {t.treeDirColor};">📁 lib</span>
          </div>
          <div>&nbsp;&nbsp;&nbsp;&nbsp;README.md</div>
          <div>&nbsp;&nbsp;&nbsp;&nbsp;package.json</div>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="footer">
      <button class="btn-reset" on:click={reset}>Reset</button>
      <div class="spacer"></div>
      <button class="btn-cancel" on:click={onClose}>Cancel</button>
      <button class="btn-save" on:click={save}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 480px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px 10px;
  }

  .header h2 { font-family: var(--font-mono); font-size: 15px; font-weight: 600; }
  .close { font-size: 18px; color: var(--text-muted); padding: 4px; }
  .close:hover { color: var(--text-primary); }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 0;
    padding: 0 18px;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    padding: 8px 16px;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  /* Body */
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .row { display: flex; gap: 12px; }
  .field { flex: 1; display: flex; flex-direction: column; gap: 4px; }
  .field.full { flex: 1; }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  select, input[type="text"], input:not([type]) {
    background: var(--bg-base);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 5px;
    outline: none;
    width: 100%;
  }
  select:focus, input:focus { border-color: var(--accent); }

  /* Stepper */
  .stepper {
    display: flex;
    align-items: center;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 5px;
    overflow: hidden;
  }

  .stepper button {
    padding: 5px 12px;
    font-size: 14px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .stepper button:hover { background: var(--bg-elevated); color: var(--text-primary); }

  .stepper-val {
    flex: 1;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 5px 0;
  }

  /* Slider */
  .slider-row { display: flex; align-items: center; gap: 8px; }
  .slider-row input[type="range"] { flex: 1; accent-color: var(--accent); }
  .slider-val { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); width: 36px; text-align: right; }

  /* Swatch color picker */
  .swatch-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 5px;
    cursor: pointer;
    position: relative;
  }
  .swatch-btn:hover { border-color: var(--accent); }

  .swatch {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    background: var(--swatch);
    border: 1px solid rgba(255,255,255,0.1);
    flex-shrink: 0;
  }

  .swatch-btn input[type="color"] {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
    height: 100%;
  }

  .hex {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
  }

  /* ANSI palette collapsible */
  .ansi-details {
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .ansi-summary {
    padding: 8px 12px;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    cursor: pointer;
    user-select: none;
  }
  .ansi-summary:hover { color: var(--text-primary); }
  .ansi-details[open] .ansi-summary { border-bottom: 1px solid var(--border); }
  .ansi-grid { padding: 10px 12px; }

  /* Color grid */
  .color-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .color-cell { display: flex; flex-direction: column; gap: 4px; }

  /* Preview */
  .preview {
    padding: 10px 14px;
    border-radius: 6px;
    border: 1px solid var(--border);
    line-height: 1.5;
    margin-top: 4px;
  }

  /* Footer */
  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border-top: 1px solid var(--border);
  }

  .spacer { flex: 1; }

  .btn-save {
    padding: 6px 18px;
    background: var(--accent);
    color: white;
    border-radius: 5px;
    font-size: 12px;
    font-family: var(--font-mono);
  }
  .btn-save:hover { opacity: 0.9; }

  .btn-cancel {
    padding: 6px 14px;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
  }
  .btn-cancel:hover { color: var(--text-primary); }

  .btn-reset {
    padding: 6px 12px;
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono);
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .btn-reset:hover { color: var(--error); border-color: var(--error); }
</style>
