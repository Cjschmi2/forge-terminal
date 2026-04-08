<script lang="ts">
  export let name: string;
  export let isDir: boolean = false;
  export let size: number = 15;

  function getColor(name: string, isDir: boolean): string {
    if (isDir) return '#6b7285';
    const ext = name.split('.').pop()?.toLowerCase() ?? '';
    switch (ext) {
      case 'rs': return '#f97316';
      case 'ts': case 'tsx': return '#60a5fa';
      case 'js': case 'jsx': case 'mjs': return '#facc15';
      case 'svelte': return '#ff3e00';
      case 'json': return '#94a3b8';
      case 'toml': case 'yaml': case 'yml': return '#6ee7b7';
      case 'md': case 'mdx': return '#93c5fd';
      case 'css': case 'scss': case 'less': return '#c084fc';
      case 'html': case 'htm': return '#fb923c';
      case 'py': return '#60a5fa';
      case 'sh': case 'bash': case 'zsh': return '#4ade80';
      case 'lock': return '#4b5563';
      case 'png': case 'jpg': case 'jpeg': case 'gif': case 'svg': case 'webp': case 'ico': return '#f472b6';
      case 'sql': return '#fbbf24';
      case 'env': return '#fbbf24';
      case 'log': return '#6b7280';
      case 'txt': return '#9ca3af';
      case 'xml': return '#fb923c';
      case 'wasm': return '#a78bfa';
      case 'zip': case 'tar': case 'gz': case 'bz2': return '#78716c';
      default: return '#94a3b8';
    }
  }

  $: color = getColor(name, isDir);
</script>

{#if isDir}
  <!-- Clean folder icon -->
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path d="M2 3.5C2 2.95 2.45 2.5 3 2.5H6.17L7.67 4H13C13.55 4 14 4.45 14 5V12C14 12.55 13.55 13 13 13H3C2.45 13 2 12.55 2 12V3.5Z" fill={color} fill-opacity="0.5"/>
    <path d="M2 5.5H14V12C14 12.55 13.55 13 13 13H3C2.45 13 2 12.55 2 12V5.5Z" fill={color} fill-opacity="0.7"/>
  </svg>
{:else}
  <!-- Minimal file indicator -->
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
    <rect x="4" y="2" width="8" height="12" rx="1" fill={color} fill-opacity="0.12" stroke={color} stroke-opacity="0.35" stroke-width="0.7"/>
  </svg>
{/if}
