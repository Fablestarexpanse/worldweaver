<script lang="ts">
  import '../app.css';
  import GeneratePanel from '$lib/components/GeneratePanel.svelte';
  import ToolBar      from '$lib/components/ToolBar.svelte';
  import BrushPanel   from '$lib/components/BrushPanel.svelte';
  import FilePanel    from '$lib/components/FilePanel.svelte';

  import { status, statusClass, terrainInfo, undoStroke } from '$lib/stores/terrain';

  // Global Ctrl+Z → undo
  function onKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
      e.preventDefault();
      undoStroke();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  <!-- ── Header ──────────────────────────────────────────────────────── -->
  <header>
    <span class="logo">🌍 WorldWeaver</span>
    {#if terrainInfo}
      <span class="world-info">
        {terrainInfo.worldWidth}×{terrainInfo.worldHeight}
        · Sea {(terrainInfo.seaLevel * 100).toFixed(0)}%
      </span>
    {/if}
  </header>

  <!-- ── Scrollable panel body ───────────────────────────────────────── -->
  <div class="panel-body">
    <GeneratePanel />
    <ToolBar />
    <BrushPanel />
    <FilePanel />
  </div>

  <!-- ── Status bar ─────────────────────────────────────────────────── -->
  <footer>
    <span class="status {statusClass}">{status}</span>
  </footer>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 300px;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }
  .logo {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--accent);
  }
  .world-info {
    font-size: 10px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
  }

  footer {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg);
    padding: 6px 12px;
  }
  .status { font-size: 11px; color: var(--text-muted); }
  .status.busy { color: var(--orange); }
  .status.ok   { color: var(--green); }
  .status.err  { color: var(--red); }
</style>
