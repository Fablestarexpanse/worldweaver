<script lang="ts">
  import {
    brushRadius, brushStrength, flattenTarget, noiseScale,
    activeTool, updateBrushParams
  } from '$lib/stores/terrain';

  // Reactively push params to Rust whenever any slider changes
  $effect(() => {
    void updateBrushParams();
  });
</script>

<div class="section">
  <div class="section-title">Brush</div>

  <div class="field">
    <label>Radius</label>
    <div class="row">
      <input type="range" min="4" max="200" step="1" bind:value={brushRadius} />
      <span class="val">{brushRadius}px</span>
    </div>
  </div>

  <div class="field">
    <label>Strength</label>
    <div class="row">
      <input type="range" min="0.01" max="1" step="0.01" bind:value={brushStrength} />
      <span class="val">{(brushStrength * 100).toFixed(0)}%</span>
    </div>
  </div>

  {#if activeTool === 'flatten'}
    <div class="field">
      <label>Flatten target</label>
      <div class="row">
        <input type="range" min="0" max="1" step="0.01" bind:value={flattenTarget} />
        <span class="val">{(flattenTarget * 100).toFixed(0)}%</span>
      </div>
    </div>
  {/if}

  {#if activeTool === 'noise'}
    <div class="field">
      <label>Noise scale</label>
      <div class="row">
        <input type="range" min="0.005" max="0.3" step="0.005" bind:value={noiseScale} />
        <span class="val">{noiseScale.toFixed(3)}</span>
      </div>
    </div>
  {/if}

  {#if !activeTool}
    <p class="hint">Select a tool above to start painting</p>
  {/if}
</div>

<style>
  .field { margin-bottom: 8px; }
  .hint { color: var(--text-muted); font-size: 11px; font-style: italic; padding-top: 4px; }
</style>
