// ── stores/terrain.ts ─────────────────────────────────────────────────────────
// Svelte 5 runes-based stores for terrain state shared across UI components.

import { invoke } from '@tauri-apps/api/core';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface TerrainConfig {
  worldWidth:      number;
  worldHeight:     number;
  seed:            number;
  octaves:         number;
  frequency:       number;
  persistence:     number;
  lacunarity:      number;
  amplitude:       number;
  seaLevel:        number;
  maxElevation:    number;
  erosionPasses:   number;
  contourInterval: number;
  sunAzimuth:      number;
}

export interface GenerateResult {
  worldWidth:  number;
  worldHeight: number;
  seaLevel:    number;
}

export type BrushTool = 'raise' | 'lower' | 'smooth' | 'flatten' | 'erode' | 'noise';

export interface BrushParams {
  radius?:        number;
  strength?:      number;
  flattenTarget?: number;
  noiseScale?:    number;
}

// ── Default config ────────────────────────────────────────────────────────────

export const DEFAULT_CONFIG: TerrainConfig = {
  worldWidth:      1024,
  worldHeight:     768,
  seed:            42,
  octaves:         8,
  frequency:       2.0,
  persistence:     0.5,
  lacunarity:      2.0,
  amplitude:       1.0,
  seaLevel:        0.42,
  maxElevation:    4000,
  erosionPasses:   5,
  contourInterval: 100,
  sunAzimuth:      315,
};

// ── Reactive state (Svelte 5 runes) ──────────────────────────────────────────

export let terrainConfig = $state<TerrainConfig>({ ...DEFAULT_CONFIG });
export let terrainInfo   = $state<GenerateResult | null>(null);
export let activeTool    = $state<BrushTool | null>(null);
export let brushRadius   = $state(30);
export let brushStrength = $state(0.5);
export let flattenTarget = $state(0.5);
export let noiseScale    = $state(0.05);
export let status        = $state('Ready');
export let statusClass   = $state('');
export let generating    = $state(false);

// ── Actions ───────────────────────────────────────────────────────────────────

export async function generateTerrain() {
  generating = true;
  status = 'Generating terrain…';
  statusClass = 'busy';
  try {
    const result: GenerateResult = await invoke('generate_terrain', { config: terrainConfig });
    terrainInfo = result;
    status = `World ${result.worldWidth}×${result.worldHeight} ready`;
    statusClass = 'ok';
  } catch (e) {
    status = `Error: ${e}`;
    statusClass = 'err';
  } finally {
    generating = false;
  }
}

export async function selectTool(tool: BrushTool | null) {
  activeTool = tool;
  await invoke('set_active_tool', { tool }).catch(console.error);
}

export async function updateBrushParams() {
  await invoke('set_brush_params', {
    params: {
      radius:        brushRadius,
      strength:      brushStrength,
      flattenTarget: flattenTarget,
      noiseScale:    noiseScale,
    }
  }).catch(console.error);
}

export async function undoStroke() {
  await invoke('undo_stroke').catch(console.error);
}

export async function resetView() {
  await invoke('reset_view').catch(console.error);
}

export async function saveWorld(path: string) {
  status = 'Saving…';
  statusClass = 'busy';
  try {
    await invoke('save_world', { path });
    status = 'World saved';
    statusClass = 'ok';
  } catch (e) {
    status = `Save failed: ${e}`;
    statusClass = 'err';
  }
}

export async function loadWorld(path: string) {
  status = 'Loading…';
  statusClass = 'busy';
  try {
    const result: GenerateResult = await invoke('load_world', { path });
    terrainInfo = result;
    status = `World ${result.worldWidth}×${result.worldHeight} loaded`;
    statusClass = 'ok';
  } catch (e) {
    status = `Load failed: ${e}`;
    statusClass = 'err';
  }
}

export async function addVolcanoes(count: number, radius: number, height: number) {
  status = 'Adding volcanoes…';
  statusClass = 'busy';
  try {
    await invoke('generate_volcanoes', { config: { count, radius, height } });
    status = `${count} volcano(es) stamped`;
    statusClass = 'ok';
  } catch (e) {
    status = `Volcano error: ${e}`;
    statusClass = 'err';
  }
}
