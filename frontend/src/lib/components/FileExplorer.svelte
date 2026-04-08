<script lang="ts">
  import { onMount } from 'svelte';
  import { fsList, type FileEntry } from '$lib/tauri';

  export let onSelect: (path: string) => void = () => {};
  export let initialPath: string = '/home';

  interface TreeNode {
    entry: FileEntry;
    children: TreeNode[] | null; // null = not loaded, [] = loaded empty
    expanded: boolean;
  }

  let rootPath = initialPath;
  let rootEntries: FileEntry[] = [];
  let tree: TreeNode[] = [];

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}K`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}M`;
  }

  async function loadDir(path: string): Promise<FileEntry[]> {
    try {
      const result = await fsList(path);
      return result.entries;
    } catch {
      return [];
    }
  }

  function entriesToNodes(entries: FileEntry[]): TreeNode[] {
    return entries.map(e => ({
      entry: e,
      children: e.is_dir ? null : [],
      expanded: false,
    }));
  }

  async function toggleNode(node: TreeNode) {
    if (!node.entry.is_dir) {
      onSelect(node.entry.path);
      return;
    }

    if (node.expanded) {
      node.expanded = false;
      tree = tree;
      return;
    }

    if (node.children === null) {
      const entries = await loadDir(node.entry.path);
      node.children = entriesToNodes(entries);
    }
    node.expanded = true;
    tree = tree;
  }

  function copyPath(path: string, e: MouseEvent) {
    e.stopPropagation();
    navigator.clipboard.writeText(path).catch(() => {});
  }

  async function navigateUp() {
    const parent = rootPath.replace(/\/[^/]+\/?$/, '') || '/';
    rootPath = parent;
    const entries = await loadDir(rootPath);
    tree = entriesToNodes(entries);
  }

  onMount(async () => {
    const entries = await loadDir(rootPath);
    tree = entriesToNodes(entries);
  });

  $: dirName = rootPath.split('/').filter(Boolean).pop() || '/';
</script>

<div class="explorer">
  <div class="explorer-header">
    <button class="back-btn" on:click={navigateUp}>&lt;</button>
    <span class="dir-name">{dirName}</span>
    <div class="spacer"></div>
    <button class="action-btn" on:click={(e) => copyPath(rootPath, e)}>Copy</button>
  </div>
  <div class="explorer-path">{rootPath}</div>
  <div class="divider"></div>

  <div class="tree-list">
    {#each tree as node}
      {@const depth = 0}
      <svelte:self this={TreeRow} {node} {depth} onToggle={toggleNode} {onSelect} {copyPath} />
    {/each}
  </div>
</div>

<!-- Recursive tree rows rendered inline -->
{#snippet TreeRow(node: TreeNode, depth: number, onToggle: (n: TreeNode) => void, onSelect: (p: string) => void, copyPath: (p: string, e: MouseEvent) => void)}
  <!-- not using snippets, using flat recursion below -->
{/snippet}

<style>
  .explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-surface);
    border-left: 1px solid var(--border);
    overflow: hidden;
  }

  .explorer-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    flex-shrink: 0;
  }

  .back-btn {
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--text-muted);
  }
  .back-btn:hover { background: var(--bg-elevated); color: var(--text-primary); }

  .dir-name { font-family: var(--font-mono); font-size: 13px; font-weight: 600; }
  .spacer { flex: 1; }

  .action-btn {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .action-btn:hover { background: var(--bg-elevated); color: var(--text-primary); }

  .explorer-path {
    padding: 0 10px 4px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
  }

  .divider { height: 1px; background: var(--border); flex-shrink: 0; }

  .tree-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
</style>
