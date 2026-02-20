<script lang="ts">
  import { s, selectTool, undoStroke, type BrushTool } from '$lib/stores/terrain.svelte';

  const TOOLS: { id: BrushTool; label: string; icon: string; tip: string }[] = [
    { id: 'raise',   label: 'Raise',   icon: '▲', tip: 'Raise terrain' },
    { id: 'lower',   label: 'Lower',   icon: '▼', tip: 'Lower terrain' },
    { id: 'smooth',  label: 'Smooth',  icon: '≈', tip: 'Smooth terrain' },
    { id: 'flatten', label: 'Flatten', icon: '─', tip: 'Flatten to target' },
    { id: 'erode',   label: 'Erode',   icon: '~', tip: 'Hydraulic erosion' },
    { id: 'noise',   label: 'Noise',   icon: '✦', tip: 'Add noise' },
  ];

  function toggle(tool: BrushTool) {
    selectTool(s.activeTool === tool ? null : tool);
  }
</script>

<div class="toolbar">
  <div class="section-title">Tools</div>
  <div class="tool-grid">
    {#each TOOLS as t}
      <button
        class:active={s.activeTool === t.id}
        title={t.tip}
        onclick={() => toggle(t.id)}
      >
        <span class="icon">{t.icon}</span>
        <span class="lbl">{t.label}</span>
      </button>
    {/each}
  </div>

  <button class="undo-btn" onclick={undoStroke} title="Undo last stroke (Ctrl+Z)">
    ↩ Undo
  </button>
</div>

<style>
  .toolbar { padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .tool-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    margin-bottom: 6px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 12px;
  }
  .icon { font-size: 14px; width: 18px; text-align: center; }
  .undo-btn { width: 100%; justify-content: center; color: var(--text-muted); }
</style>
