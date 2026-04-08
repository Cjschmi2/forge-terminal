<script lang="ts">
  import { machinesList, machinesSave, type MachineConfig } from '$lib/tauri';
  import { onMount } from 'svelte';

  export let onClose: () => void = () => {};
  export let onSaved: () => void = () => {};

  let machines: MachineConfig[] = [];
  let editing: MachineConfig | null = null;
  let isNew = false;

  function newMachine(): MachineConfig {
    return {
      id: `machine-${Date.now()}`,
      name: '',
      type: 'ssh',
      host: '',
      user: '',
      port: 22,
      identity_file: '',
    };
  }

  async function load() {
    machines = await machinesList();
  }

  function startAdd() {
    editing = newMachine();
    isNew = true;
  }

  function startEdit(m: MachineConfig) {
    editing = { ...m };
    isNew = false;
  }

  async function save() {
    if (!editing || !editing.name.trim() || !editing.host.trim()) return;
    editing.id = editing.id || `machine-${Date.now()}`;
    if (!editing.user.trim()) editing.user = 'ubuntu';

    if (isNew) {
      machines = [...machines, editing];
    } else {
      machines = machines.map(m => m.id === editing!.id ? editing! : m);
    }

    await machinesSave(machines);
    editing = null;
    onSaved();
  }

  async function remove(id: string) {
    machines = machines.filter(m => m.id !== id);
    await machinesSave(machines);
    onSaved();
  }

  function cancel() {
    editing = null;
  }

  onMount(load);
</script>

<div class="modal-overlay" on:click={onClose} on:keydown={() => {}} role="presentation">
  <div class="modal" on:click|stopPropagation on:keydown|stopPropagation role="dialog">
    <div class="modal-header">
      <h2>Machines</h2>
      <button class="close-btn" on:click={onClose}>&times;</button>
    </div>

    {#if editing}
      <div class="form">
        <label>
          <span>Name</span>
          <input bind:value={editing.name} placeholder="My Server" />
        </label>
        <label>
          <span>Type</span>
          <div class="type-row">
            <button class="type-btn" class:active={editing.type === 'ssh'} on:click={() => editing && (editing.type = 'ssh')}>SSH</button>
            <button class="type-btn" class:active={editing.type === 'tailscale'} on:click={() => editing && (editing.type = 'tailscale')}>Tailscale</button>
          </div>
        </label>
        <label>
          <span>Host</span>
          <input bind:value={editing.host} placeholder="192.168.1.100 or hostname.ts.net" />
        </label>
        <label>
          <span>User</span>
          <input bind:value={editing.user} placeholder="ubuntu" />
        </label>
        {#if editing.type === 'ssh'}
          <label>
            <span>Port</span>
            <input type="number" bind:value={editing.port} />
          </label>
          <label>
            <span>Identity File</span>
            <input bind:value={editing.identity_file} placeholder="~/.ssh/id_rsa (optional)" />
          </label>
        {/if}
        <div class="form-actions">
          <button class="btn-save" on:click={save}>Save</button>
          <button class="btn-cancel" on:click={cancel}>Cancel</button>
        </div>
      </div>
    {:else}
      <div class="machine-list">
        {#each machines as m}
          <div class="machine-item">
            <div class="machine-info">
              <span class="machine-name">{m.name}</span>
              <span class="machine-detail">{m.user}@{m.host} ({m.type})</span>
            </div>
            <button class="edit-btn" on:click={() => startEdit(m)}>Edit</button>
            <button class="delete-btn" on:click={() => remove(m.id)}>Delete</button>
          </div>
        {/each}
        {#if machines.length === 0}
          <p class="empty-msg">No machines configured.</p>
        {/if}
      </div>
      <button class="btn-add" on:click={startAdd}>Add Machine</button>
    {/if}
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 20px;
    min-width: 400px;
    max-width: 500px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-header h2 {
    font-family: var(--font-mono);
    font-size: 16px;
    font-weight: 600;
  }

  .close-btn { font-size: 18px; color: var(--text-muted); padding: 4px; }
  .close-btn:hover { color: var(--text-primary); }

  .form { display: flex; flex-direction: column; gap: 12px; }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label span {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  input {
    background: var(--bg-base);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 6px 10px;
    border-radius: 4px;
    outline: none;
  }

  input:focus { border-color: var(--accent); }

  .type-row { display: flex; gap: 4px; }

  .type-btn {
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .type-btn:hover { background: var(--bg-elevated); }
  .type-btn.active { border-color: var(--accent); color: var(--accent); }

  .form-actions { display: flex; gap: 8px; margin-top: 8px; }

  .btn-save {
    padding: 6px 16px;
    background: var(--accent);
    color: white;
    border-radius: 4px;
    font-size: 12px;
    font-family: var(--font-mono);
  }
  .btn-save:hover { opacity: 0.9; }

  .btn-cancel {
    padding: 6px 16px;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .machine-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }

  .machine-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg-base);
    border-radius: 6px;
  }

  .machine-info { flex: 1; }
  .machine-name { font-family: var(--font-mono); font-size: 13px; font-weight: 600; display: block; }
  .machine-detail { font-size: 11px; color: var(--text-muted); }

  .edit-btn, .delete-btn {
    padding: 3px 8px;
    font-size: 11px;
    color: var(--text-muted);
    border-radius: 4px;
  }
  .edit-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .delete-btn:hover { color: var(--error); background: var(--bg-elevated); }

  .btn-add {
    width: 100%;
    padding: 8px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-muted);
  }
  .btn-add:hover { border-color: var(--accent); color: var(--accent); }

  .empty-msg { text-align: center; color: var(--text-muted); font-size: 12px; padding: 12px; }
</style>
