<script lang="ts">
  import { onMount } from 'svelte';
  import Terminal from '$lib/components/Terminal.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import MachineConfig from '$lib/components/MachineConfig.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import { initTheme } from '$lib/stores/theme';
  import { launchSession, ptyKill, ptySend, getHomeDir, machinesList, remoteHome, type MachineConfig as MachineType } from '$lib/tauri';

  interface Tab {
    name: string;
    displayName: string;
    tool: string;
    workingDir: string;
    machine: string;
    machineName: string;
    treeRoot: string;
    agentName: string;
    state: 'launcher' | 'terminal';
  }

  let tabs: Tab[] = [];
  let activeTab = 0;
  let showTree = true;
  let treeWidth = 280;
  let renamingTab: number | null = null;
  let renameValue = '';
  let homeDir = '/home';
  let machines: MachineType[] = [];
  let showPlusMenu = false;
  let showMachineConfig = false;
  let showSettings = false;
  let windowWidth = 1400;
  let userHidTree = false;
  let isDragging = false;
  let sidebarMode: 'files' | 'rooms' = 'files';
  let namingTab: number | null = null;
  let namingTool: string = '';
  let namingValue: string = '';

  const tools = [
    { id: 'claude', label: 'Claude' },
    { id: 'codex', label: 'Codex' },
    { id: 'bash', label: 'Bash' },
  ];

  onMount(async () => {
    windowWidth = window.innerWidth;
    await initTheme();
    homeDir = await getHomeDir();
    machines = await machinesList();
    openLocal();
  });

  async function reloadMachines() {
    machines = await machinesList();
  }

  function openLocal() {
    const tab: Tab = {
      name: '',
      displayName: 'Local',
      tool: '',
      workingDir: homeDir,
      machine: 'local',
      machineName: 'Local',
      treeRoot: homeDir,
      agentName: '',
      state: 'launcher',
    };
    tabs = [...tabs, tab];
    activeTab = tabs.length - 1;
    showPlusMenu = false;
  }

  async function openRemote(m: MachineType) {
    showPlusMenu = false;
    let root = '/home';
    try { root = await remoteHome(m.id); } catch {}
    const tab: Tab = {
      name: '',
      displayName: m.name,
      tool: '',
      workingDir: root,
      machine: m.id,
      machineName: m.name,
      treeRoot: root,
      agentName: '',
      state: 'launcher',
    };
    tabs = [...tabs, tab];
    activeTab = tabs.length - 1;
  }

  function promptAgentName(toolId: string, tabIdx: number) {
    namingTab = tabIdx;
    namingTool = toolId;
    namingValue = '';
  }

  function confirmName() {
    if (namingTab !== null) {
      const idx = namingTab;
      const tool = namingTool;
      tabs[idx].agentName = namingValue.trim();
      namingTab = null;
      namingTool = '';
      namingValue = '';
      launch(tool, idx);
    }
  }

  function cancelName() {
    namingTab = null;
    namingTool = '';
    namingValue = '';
  }

  async function launch(toolId: string, tabIdx: number) {
    const tab = tabs[tabIdx];
    const name = `${toolId}-${Date.now()}`;
    const dirName = tab.workingDir.split('/').pop() || 'home';
    const machineLabel = tab.machine === 'local' ? '' : ` @${tab.machineName}`;
    const agentLabel = tab.agentName ? ` @${tab.agentName}` : '';

    try {
      await launchSession({
        name,
        tool: toolId,
        working_directory: tab.workingDir,
        machine: tab.machine,
      });

      // If agent name was provided, inject the register tag after a brief delay
      // to let the session initialize
      if (tab.agentName) {
        setTimeout(() => {
          ptySend(name, `[{register:@${tab.agentName}}]\n`).catch(() => {});
        }, 1500);
      }

      tabs[tabIdx] = {
        ...tab,
        name,
        displayName: tab.agentName ? `@${tab.agentName} (${dirName})` : `${toolId} (${dirName})${machineLabel}`,
        tool: toolId,
        state: 'terminal',
      };
      tabs = tabs;
    } catch (e) {
      console.error('Launch failed:', e);
    }
  }

  async function closeTab(idx: number) {
    const tab = tabs[idx];
    if (tab.name && tab.state === 'terminal') {
      try { await ptyKill(tab.name); } catch {}
    }
    tabs = tabs.filter((_, i) => i !== idx);
    if (tabs.length === 0) {
      openLocal();
    } else if (activeTab >= tabs.length) {
      activeTab = tabs.length - 1;
    }
  }

  function startRename(idx: number) {
    renamingTab = idx;
    renameValue = tabs[idx].displayName;
  }

  function finishRename() {
    if (renamingTab !== null && renameValue.trim()) {
      tabs[renamingTab].displayName = renameValue.trim();
      tabs = tabs;
    }
    renamingTab = null;
  }

  function onSelectDir(path: string) {
    if (tabs[activeTab]) {
      tabs[activeTab].workingDir = path;
      tabs = tabs;
    }
  }

  function toggleTree() {
    showTree = !showTree;
    userHidTree = !showTree;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'b') { e.preventDefault(); toggleTree(); }
    if (e.ctrlKey && e.key === 'w') { e.preventDefault(); if (tabs.length > 0) closeTab(activeTab); }
  }

  function handleResize() {
    windowWidth = window.innerWidth;
  }

  // Auto-collapse tree when narrow, auto-restore when wide (unless user explicitly hid it)
  $: {
    if (windowWidth < 900 && showTree) {
      showTree = false;
    } else if (windowWidth >= 900 && !showTree && !userHidTree) {
      showTree = true;
    }
  }

  // Clamp tree width so it never exceeds 40% of the window
  $: effectiveTreeWidth = Math.min(treeWidth, windowWidth * 0.4);

  $: currentTab = tabs[activeTab] ?? null;
</script>

<svelte:window on:keydown={handleKeydown} on:resize={handleResize} />

<div class="app">
  <!-- Tab bar -->
  <div class="tab-bar">
    <div class="tab-scroll">
      {#each tabs as tab, i}
        <div
          class="tab"
          class:active={i === activeTab}
          role="tab"
          tabindex="0"
          on:click={() => activeTab = i}
          on:dblclick={() => startRename(i)}
          on:keydown={(e) => { if (e.key === 'Enter') activeTab = i; }}
        >
          {#if renamingTab === i}
            <input
              class="rename-input"
              bind:value={renameValue}
              on:blur={finishRename}
              on:keydown={(e) => { if (e.key === 'Enter') finishRename(); if (e.key === 'Escape') { renamingTab = null; } }}
              autofocus
            />
          {:else}
            <span class="tab-name">{tab.displayName}</span>
          {/if}
          <button class="tab-close" on:click|stopPropagation={() => closeTab(i)}>&times;</button>
        </div>
      {/each}

      <!-- + button with machine dropdown -->
      <div class="dropdown-wrapper">
        <button class="tab-plus" on:click={() => showPlusMenu = !showPlusMenu}>+</button>
        {#if showPlusMenu}
          <div class="dropdown-menu">
            <button class="dropdown-item" on:click={openLocal}>
              Local
              <span class="dropdown-detail">This machine</span>
            </button>
            {#each machines as m}
              <button class="dropdown-item" on:click={() => openRemote(m)}>
                {m.name}
                <span class="dropdown-detail">{m.user}@{m.host}</span>
              </button>
            {/each}
            <div class="dropdown-divider"></div>
            <button class="dropdown-item" on:click={() => { showPlusMenu = false; showMachineConfig = true; }}>
              Configure...
            </button>
          </div>
        {/if}
      </div>
    </div>

    <div class="tab-spacer"></div>

    <div class="tab-actions">
      <button class="tab-btn" on:click={() => showSettings = true}>Settings</button>
      <button class="tab-btn" class:active-mode={showTree && sidebarMode === 'files'} on:click={() => { sidebarMode = 'files'; if (!showTree) { showTree = true; userHidTree = false; } else if (sidebarMode === 'files') toggleTree(); }}>Files</button>
      <button class="tab-btn" class:active-mode={showTree && sidebarMode === 'rooms'} on:click={() => { sidebarMode = 'rooms'; if (!showTree) { showTree = true; userHidTree = false; } else if (sidebarMode === 'rooms') toggleTree(); }}>Rooms</button>
    </div>
  </div>

  <!-- Main content -->
  <div class="main">
    <div class="terminal-area">
      {#each tabs as tab, i}
        <div class="tab-content" class:visible={i === activeTab}>
          {#if tab.state === 'launcher'}
            <div class="launcher">
              {#if namingTab === i}
                <h2 class="naming-title">Name this agent</h2>
                <div class="naming-row">
                  <input
                    class="naming-input"
                    type="text"
                    placeholder="agent name"
                    bind:value={namingValue}
                    on:keydown={(e) => { if (e.key === 'Enter') confirmName(); if (e.key === 'Escape') cancelName(); }}
                    autofocus
                  />
                </div>
                <div class="naming-actions">
                  <button class="naming-btn naming-skip" on:click={confirmName}>
                    {namingValue.trim() ? 'Start' : 'Skip'}
                  </button>
                  <button class="naming-btn naming-cancel" on:click={cancelName}>Cancel</button>
                </div>
              {:else}
                <h1>Forge Terminal</h1>
                <div class="launcher-tools">
                  {#each tools as tool}
                    <button class="tool-pill" on:click={() => promptAgentName(tool.id, i)}>
                      {tool.label}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {:else}
            <Terminal sessionName={tab.name} />
          {/if}
        </div>
      {/each}
    </div>

    {#if showTree && currentTab}
      <!-- Drag handle -->
      <div
        class="pane-divider"
        role="separator"
        on:mousedown={(e) => {
          e.preventDefault();
          isDragging = true;
          const startX = e.clientX;
          const startW = treeWidth;
          const onMove = (ev) => { treeWidth = Math.max(120, Math.min(600, startW - (ev.clientX - startX))); };
          const onUp = () => {
            isDragging = false;
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
            window.dispatchEvent(new Event('resize'));
          };
          window.addEventListener('mousemove', onMove);
          window.addEventListener('mouseup', onUp);
        }}
      ></div>
      <div class="tree-area" class:dragging={isDragging} style="width: {effectiveTreeWidth}px;">
        {#if sidebarMode === 'files'}
          {#key `${currentTab.machine}-${activeTab}`}
            <FileTree
              initialPath={currentTab.treeRoot}
              machineId={currentTab.machine}
              {onSelectDir}
            />
          {/key}
        {:else}
          <div class="rooms-panel">
            <div class="rooms-header">
              <span class="rooms-title">Rooms</span>
            </div>
            <div class="rooms-empty">
              <p>Room view shows active agent rooms when network mode is enabled.</p>
              <p class="rooms-hint">Agents register with <code>{'[{register:@name}]'}</code></p>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if showPlusMenu}
  <div class="overlay" on:click={() => showPlusMenu = false} on:keydown={() => {}} role="presentation"></div>
{/if}

{#if showMachineConfig}
  <MachineConfig
    onClose={() => showMachineConfig = false}
    onSaved={reloadMachines}
  />
{/if}

{#if showSettings}
  <Settings onClose={() => showSettings = false} />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-base);
  }

  /* ── Tab bar ─────────────────────────────────────────────────── */
  .tab-bar {
    display: flex;
    align-items: center;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    padding: 0 8px;
    height: 34px;
    flex-shrink: 0;
    gap: 1px;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    min-width: 0;
  }

  .tab-scroll {
    display: flex;
    align-items: center;
    gap: 1px;
    overflow-x: auto;
    overflow-y: hidden;
    flex: 1;
    min-width: 0;
    scrollbar-width: none;
  }
  .tab-scroll::-webkit-scrollbar { display: none; }

  .tab-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 14px;
    border-radius: 6px 6px 0 0;
    font-size: 12px;
    color: var(--text-muted);
    cursor: pointer;
    max-width: 220px;
    min-width: 80px;
    flex-shrink: 0;
    transition: all 0.12s;
  }
  .tab:hover { background: rgba(255,255,255,0.04); color: var(--text-primary); }
  .tab.active { background: var(--bg-base); color: var(--text-primary); border-bottom: 2px solid var(--accent); }

  .tab-name {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  .rename-input {
    background: var(--bg-base);
    border: 1px solid var(--accent);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    width: 130px;
    outline: none;
  }

  .tab-close {
    font-size: 13px;
    color: var(--text-muted);
    padding: 0 3px;
    line-height: 1;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .tab:hover .tab-close { opacity: 1; }
  .tab-close:hover { color: #f87171; }

  .tab-spacer { flex: 1; }

  .tab-btn {
    padding: 4px 12px;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    border-radius: 5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    transition: all 0.12s;
  }
  .tab-btn:hover { background: rgba(255,255,255,0.06); color: var(--text-primary); }
  .tab-btn.active-mode { color: var(--accent); }

  /* ── + dropdown ──────────────────────────────────────────────── */
  .dropdown-wrapper { position: relative; }

  .tab-plus {
    width: 26px;
    height: 26px;
    border-radius: 5px;
    font-size: 15px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s;
  }
  .tab-plus:hover { background: rgba(255,255,255,0.06); color: var(--text-primary); }

  .dropdown-menu {
    position: absolute;
    top: 30px;
    left: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    min-width: 220px;
    z-index: 100;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    backdrop-filter: blur(16px);
  }

  .dropdown-item {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 8px 14px;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    border-radius: 6px;
    transition: background 0.1s;
  }
  .dropdown-item:hover { background: rgba(255,255,255,0.05); }

  .dropdown-detail { font-size: 10px; color: var(--text-muted); margin-top: 1px; }
  .dropdown-divider { height: 1px; background: var(--border); margin: 4px 8px; }

  .overlay { position: fixed; inset: 0; z-index: 99; }

  /* ── Main layout ─────────────────────────────────────────────── */
  .main {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-area {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .tab-content {
    position: absolute;
    inset: 0;
    display: none;
  }
  .tab-content.visible {
    display: flex;
    position: relative;
    flex: 1;
  }

  .pane-divider {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    flex-shrink: 0;
    transition: background 0.15s;
    position: relative;
  }
  .pane-divider::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 1px;
    width: 1px;
    background: var(--border);
  }
  .pane-divider:hover { background: rgba(124, 138, 247, 0.15); }
  .pane-divider:hover::after { background: var(--accent); }

  .tree-area {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    transition: width 0.15s ease;
  }
  .tree-area.dragging {
    transition: none;
  }

  /* ── Launcher ────────────────────────────────────────────────── */
  .launcher {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
    padding: 20px;
    min-width: 0;
  }

  .launcher h1 {
    font-family: var(--font-mono);
    font-size: 26px;
    font-weight: 300;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .naming-title {
    font-family: var(--font-mono);
    font-size: 18px;
    font-weight: 400;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }
  .naming-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .naming-input {
    background: rgba(255,255,255,0.03);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 16px;
    font-family: var(--font-mono);
    font-size: 15px;
    color: var(--text-primary);
    width: 240px;
    outline: none;
    transition: border-color 0.15s;
  }
  .naming-input:focus { border-color: var(--accent); }
  .naming-input::placeholder { color: var(--text-muted); }
  .naming-actions {
    display: flex;
    gap: 8px;
  }
  .naming-btn {
    padding: 8px 20px;
    border-radius: 8px;
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 500;
    transition: all 0.12s;
  }
  .naming-skip {
    background: var(--accent);
    color: white;
  }
  .naming-skip:hover { opacity: 0.9; }
  .naming-cancel {
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .naming-cancel:hover { color: var(--text-primary); border-color: var(--text-muted); }

  .launcher-tools { display: flex; flex-wrap: wrap; justify-content: center; gap: 10px; }

  .tool-pill {
    padding: 14px 28px;
    background: rgba(255,255,255,0.03);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    transition: all 0.15s;
    letter-spacing: 0.01em;
  }
  .tool-pill:hover {
    border-color: var(--accent);
    background: rgba(124, 138, 247, 0.08);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(99, 102, 241, 0.15);
  }

  /* ── Rooms panel ──────────────────────────────────────────────── */
  .rooms-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tree-bg);
    border-left: 1px solid var(--border);
    color: var(--text-primary);
  }
  .rooms-header {
    display: flex;
    align-items: center;
    padding: 10px 12px 6px;
    flex-shrink: 0;
  }
  .rooms-title {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .rooms-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 20px;
    text-align: center;
    gap: 8px;
  }
  .rooms-empty p {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .rooms-hint code {
    font-family: var(--font-mono);
    background: rgba(255,255,255,0.04);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
  }

  .launcher-dir { font-size: 11px; color: var(--text-muted); }
  .launcher-dir code {
    font-family: var(--font-mono);
    background: rgba(255,255,255,0.04);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
  }
</style>
