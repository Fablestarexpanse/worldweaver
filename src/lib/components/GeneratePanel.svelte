<script lang="ts">
  import { s, generateTerrain, DEFAULT_CONFIG } from '$lib/stores/terrain.svelte';

  let showAdvanced = $state(false);

  function randomSeed() {
    s.config.seed = Math.floor(Math.random() * 0xFFFFFFFF);
  }
</script>

<div class="section">
  <div class="section-title">World</div>

  <div class="field">
    <label>Width × Height (px)</label>
    <div class="row">
      <input type="number" min="128" max="4096" step="128" bind:value={s.config.worldWidth} />
      <span style="color:var(--text-muted); padding:0 4px">×</span>
      <input type="number" min="128" max="4096" step="128" bind:value={s.config.worldHeight} />
    </div>
  </div>

  <div class="field">
    <label>Seed</label>
    <div class="row">
      <input type="number" bind:value={s.config.seed} />
      <button onclick={randomSeed} title="Random seed" style="flex:0;padding:4px 8px">🎲</button>
    </div>
  </div>

  <div class="field">
    <label>Sea level ({(s.config.seaLevel * 100).toFixed(0)}%)</label>
    <input type="range" min="0.1" max="0.8" step="0.01" bind:value={s.config.seaLevel} />
  </div>

  <div class="field">
    <label>Max elevation (m)</label>
    <input type="number" min="100" max="9000" step="100" bind:value={s.config.maxElevation} />
  </div>

  <button class="toggle" onclick={() => showAdvanced = !showAdvanced}>
    {showAdvanced ? '▲' : '▼'} Advanced noise
  </button>

  {#if showAdvanced}
    <div class="advanced">
      <div class="field">
        <label>Octaves</label>
        <input type="range" min="1" max="12" step="1" bind:value={s.config.octaves} />
        <span class="val">{s.config.octaves}</span>
      </div>
      <div class="field">
        <label>Frequency</label>
        <input type="range" min="0.5" max="8" step="0.1" bind:value={s.config.frequency} />
        <span class="val">{s.config.frequency.toFixed(1)}</span>
      </div>
      <div class="field">
        <label>Persistence</label>
        <input type="range" min="0.1" max="0.9" step="0.05" bind:value={s.config.persistence} />
        <span class="val">{s.config.persistence.toFixed(2)}</span>
      </div>
      <div class="field">
        <label>Lacunarity</label>
        <input type="range" min="1" max="4" step="0.1" bind:value={s.config.lacunarity} />
        <span class="val">{s.config.lacunarity.toFixed(1)}</span>
      </div>
      <div class="field">
        <label>Erosion passes</label>
        <input type="range" min="0" max="20" step="1" bind:value={s.config.erosionPasses} />
        <span class="val">{s.config.erosionPasses}</span>
      </div>
    </div>
  {/if}

  <button
    class="gen-btn"
    onclick={generateTerrain}
    disabled={s.generating}
  >
    {s.generating ? '⏳ Generating…' : '⚡ Generate World'}
  </button>
</div>

<style>
  .field { margin-bottom: 8px; }
  .val { color: var(--accent); font-size: 11px; display: block; text-align: right; }
  .toggle {
    width: 100%;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 11px;
    text-align: left;
    padding: 2px 0;
    margin-bottom: 6px;
    cursor: pointer;
  }
  .advanced { padding-left: 4px; border-left: 2px solid var(--border); margin-bottom: 8px; }
  .gen-btn {
    width: 100%;
    padding: 8px;
    font-size: 13px;
    font-weight: 600;
    background: var(--accent-dim);
    border-color: var(--accent);
    color: #fff;
  }
  .gen-btn:hover:not(:disabled) { background: #2679d5; }
  .gen-btn:disabled { opacity: 0.5; cursor: wait; }
</style>
