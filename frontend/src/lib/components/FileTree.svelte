<script lang="ts">
  import { onMount } from 'svelte';
  import { fsList, remoteLs, openFile, type FileEntry, type FsListResponse } from '$lib/tauri';
  import FileIcon from '$lib/components/FileIcon.svelte';

  export let initialPath: string = '/home';
  export let machineId: string = 'local';
  export let onSelectDir: (path: string) => void = () => {};

  interface FlatNode {
    entry: FileEntry;
    depth: number;
    expanded: boolean;
    childrenLoaded: boolean;
  }

  let nodes: FlatNode[] = [];
  let rootPath = initialPath;

  // Context menu state
  let contextMenu: { x: number; y: number; node: FlatNode } | null = null;

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}K`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}M`;
  }

  async function loadDir(path: string): Promise<FileEntry[]> {
    try {
      let result: FsListResponse;
      if (machineId === 'local') {
        result = await fsList(path);
      } else {
        result = await remoteLs(machineId, path);
      }
      return result.entries;
    } catch {
      return [];
    }
  }

  async function loadRoot() {
    const entries = await loadDir(rootPath);
    nodes = entries.map(e => ({
      entry: e,
      depth: 0,
      expanded: false,
      childrenLoaded: false,
    }));
  }

  async function toggleDir(index: number) {
    const node = nodes[index];
    if (!node.entry.is_dir) return;

    if (node.expanded) {
      let removeCount = 0;
      for (let i = index + 1; i < nodes.length; i++) {
        if (nodes[i].depth > node.depth) removeCount++;
        else break;
      }
      nodes = [...nodes.slice(0, index + 1), ...nodes.slice(index + 1 + removeCount)];
      nodes[index] = { ...node, expanded: false };
    } else {
      if (!node.childrenLoaded) {
        const children = await loadDir(node.entry.path);
        const childNodes: FlatNode[] = children.map(e => ({
          entry: e,
          depth: node.depth + 1,
          expanded: false,
          childrenLoaded: false,
        }));
        nodes = [
          ...nodes.slice(0, index + 1),
          ...childNodes,
          ...nodes.slice(index + 1),
        ];
        nodes[index] = { ...node, expanded: true, childrenLoaded: true };
      } else {
        nodes[index] = { ...node, expanded: true };
      }
    }
  }

  function selectEntry(node: FlatNode) {
    if (node.entry.is_dir) {
      onSelectDir(node.entry.path);
    } else if (machineId === 'local') {
      openFile(node.entry.path).catch(() => copyPath(node.entry.path));
    } else {
      copyPath(node.entry.path);
    }
  }

  function copyPath(path: string) {
    navigator.clipboard.writeText(path).catch(() => {});
  }

  async function setRoot(path: string) {
    rootPath = path;
    await loadRoot();
    onSelectDir(path);
  }

  function showContextMenu(e: MouseEvent, node: FlatNode) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, node };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  async function goUp() {
    rootPath = rootPath.replace(/\/[^/]+\/?$/, '') || '/';
    await loadRoot();
  }

  // Reload when machineId or initialPath changes
  export async function reload(path: string) {
    rootPath = path;
    await loadRoot();
  }

  onMount(loadRoot);

  $: dirName = rootPath.split('/').filter(Boolean).pop() || '/';
</script>

<div class="file-tree">
  <div class="tree-header">
    <button class="back-btn" on:click={goUp}>&lt;</button>
    <button class="path-btn" on:click={() => copyPath(rootPath)} title="Click to copy path">
      {rootPath}
    </button>
  </div>
  <div class="divider"></div>

  <div class="tree-scroll">
    {#each nodes as node, i}
      <div
        class="tree-row"
        class:is-dir={node.entry.is_dir}
        style="padding-left: {12 + node.depth * 16}px"
        role="treeitem"
        tabindex="0"
        on:click={() => node.entry.is_dir ? toggleDir(i) : selectEntry(node)}
        on:dblclick={() => selectEntry(node)}
        on:contextmenu={(e) => showContextMenu(e, node)}
        on:keydown={(e) => { if (e.key === 'Enter') { node.entry.is_dir ? toggleDir(i) : selectEntry(node); }}}
      >
        {#if node.entry.is_dir}
          <span class="arrow">{node.expanded ? '\u25BE' : '\u25B8'}</span>
        {:else}
          <span class="arrow-spacer"></span>
        {/if}
        <span class="icon"><FileIcon name={node.entry.name} isDir={node.entry.is_dir} size={15} /></span>
        <span class="name">{node.entry.name}</span>
        {#if !node.entry.is_dir}
          <span class="size">{formatSize(node.entry.size)}</span>
        {/if}
      </div>
    {/each}
  </div>
</div>

{#if contextMenu}
  <div class="ctx-overlay" on:click={closeContextMenu} on:contextmenu|preventDefault={closeContextMenu} on:keydown={() => {}} role="presentation"></div>
  <div class="ctx-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
    <button class="ctx-item" on:click={() => { copyPath(contextMenu.node.entry.path); closeContextMenu(); }}>
      Copy Path
    </button>
    {#if contextMenu.node.entry.is_dir}
      <button class="ctx-item" on:click={() => { setRoot(contextMenu.node.entry.path); closeContextMenu(); }}>
        Set as Root
      </button>
    {/if}
  </div>
{/if}

<style>
  .file-tree {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tree-bg);
    border-left: 1px solid var(--border);
    color: #e8ecf4;
  }

  .tree-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px 6px;
    flex-shrink: 0;
  }

  .back-btn {
    padding: 3px 8px;
    border-radius: 5px;
    font-family: var(--tree-font);
    font-size: 13px;
    color: rgba(255,255,255,0.7);
    font-weight: 500;
  }
  .back-btn:hover { background: rgba(255,255,255,0.06); color: #fff; }

  .path-btn {
    flex: 1;
    text-align: left;
    font-family: var(--tree-font);
    font-size: 11px;
    font-weight: 500;
    color: rgba(255,255,255,0.75);
    padding: 3px 6px;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .path-btn:hover { background: rgba(255,255,255,0.06); color: rgba(255,255,255,0.85); }

  .divider { height: 1px; background: rgba(255,255,255,0.06); flex-shrink: 0; }

  .tree-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    cursor: pointer;
    font-family: var(--tree-font);
    font-size: var(--tree-font-size);
    color: #dde1ec;
    user-select: none;
    border-radius: 4px;
    margin: 0 4px;
    transition: background 0.1s;
  }

  .tree-row:hover { background: rgba(255,255,255,0.05); }
  .tree-row.is-dir .name { color: rgba(255,255,255,0.75); font-weight: 500; }

  .arrow { width: 10px; font-size: 8px; color: rgba(255,255,255,0.5); flex-shrink: 0; text-align: center; }
  .arrow-spacer { width: 10px; flex-shrink: 0; }
  .icon { width: 16px; flex-shrink: 0; display: flex; align-items: center; }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .size { font-size: 9px; color: rgba(255,255,255,0.45); flex-shrink: 0; font-family: var(--tree-font); }

  /* Context menu */
  .ctx-overlay { position: fixed; inset: 0; z-index: 299; }
  .ctx-menu {
    position: fixed;
    z-index: 300;
    background: var(--bg-surface, #12151e);
    border: 1px solid var(--border, #252836);
    border-radius: 6px;
    padding: 4px;
    min-width: 160px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    backdrop-filter: blur(16px);
  }
  .ctx-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    text-align: left;
    font-family: var(--tree-font);
    font-size: 12px;
    color: #dde1ec;
    border-radius: 4px;
    cursor: pointer;
  }
  .ctx-item:hover { background: rgba(255,255,255,0.06); }
</style>
