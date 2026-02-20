<script lang="ts">
  import { s, updateBrushParams } from '$lib/stores/terrain.svelte';

  // Local copies so we can bind:value without binding to object properties that
  // might not trigger $effect chains perfectly across components.
  // We sync them back to the store on every change via $effect.
  let radius   = $state(s.brushRadius);
  let strength = $state(s.brushStrength);
  let flatten  = $state(s.flattenTarget);
  let noise    = $state(s.noiseScale);

  // Push local values back to store + Tauri whenever they change
  $effect(() => {
    s.brushRadius   = radius;
    s.brushStrength = strength;
    s.flattenTarget = flatten;
    s.noiseScale    = noise;
    void updateBrushParams();
  });
</script>

<div class="section">
  <div class="section-title">Brush</div>

  <div class="field">
    <label>Radius</label>
    <div class="row">
      <input type="range" min="4" max="200" step="1" bind:value={radius} />
      <span class="val">{radius}px</span>
    </div>
  </div>

  <div class="field">
    <label>Strength</label>
    <div class="row">
      <input type="range" min="0.01" max="1" step="0.01" bind:value={strength} />
      <span class="val">{(strength * 100).toFixed(0)}%</span>
    </div>
  </div>

  {#if s.activeTool === 'flatten'}
    <div class="field">
      <label>Flatten target</label>
      <div class="row">
        <input type="range" min="0" max="1" step="0.01" bind:value={flatten} />
        <span class="val">{(flatten * 100).toFixed(0)}%</span>
      </div>
    </div>
  {/if}

  {#if s.activeTool === 'noise'}
    <div class="field">
      <label>Noise scale</label>
      <div class="row">
        <input type="range" min="0.005" max="0.3" step="0.005" bind:value={noise} />
        <span class="val">{noise.toFixed(3)}</span>
      </div>
    </div>
  {/if}

  {#if !s.activeTool}
    <p class="hint">Select a tool above to start painting</p>
  {/if}
</div>

<style>
  .field { margin-bottom: 8px; }
  .hint { color: var(--text-muted); font-size: 11px; font-style: italic; padding-top: 4px; }
</style>
