<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { saveWorld, loadWorld, addVolcanoes, resetView } from '$lib/stores/terrain.svelte';

  let volcCount    = $state(3);
  let volcRadius   = $state(80);
  let volcHeight   = $state(0.95);

  async function onSave() {
    const path = await save({
      filters: [{ name: 'WorldWeaver World', extensions: ['wwdb'] }],
    });
    if (path) await saveWorld(path);
  }

  async function onLoad() {
    const path = await open({
      filters: [{ name: 'WorldWeaver World', extensions: ['wwdb'] }],
      multiple: false,
    });
    if (typeof path === 'string') await loadWorld(path);
  }
</script>

<div class="section">
  <div class="section-title">File</div>
  <div class="row">
    <button onclick={onSave}>💾 Save</button>
    <button onclick={onLoad}>📂 Load</button>
  </div>
  <button style="width:100%;margin-top:4px" onclick={resetView}>🔭 Fit view</button>
</div>

<div class="section">
  <div class="section-title">Volcanoes</div>
  <div class="field">
    <label>Count</label>
    <div class="row">
      <input type="range" min="1" max="10" step="1" bind:value={volcCount} />
      <span class="val">{volcCount}</span>
    </div>
  </div>
  <div class="field">
    <label>Radius (px)</label>
    <div class="row">
      <input type="range" min="20" max="300" step="10" bind:value={volcRadius} />
      <span class="val">{volcRadius}</span>
    </div>
  </div>
  <div class="field">
    <label>Peak height ({(volcHeight * 100).toFixed(0)}%)</label>
    <input type="range" min="0.5" max="1.0" step="0.01" bind:value={volcHeight} />
  </div>
  <button
    style="width:100%"
    onclick={() => addVolcanoes(volcCount, volcRadius, volcHeight)}
  >
    🌋 Stamp Volcanoes
  </button>
</div>

<style>
  .field { margin-bottom: 8px; }
  .val { color: var(--accent); font-size: 11px; min-width: 32px; text-align: right; }
</style>
