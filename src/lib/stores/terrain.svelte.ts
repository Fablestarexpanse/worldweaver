// ── stores/terrain.svelte.ts ──────────────────────────────────────────────────
// Svelte 5 runes-based stores for terrain state shared across UI components.
// Uses a single exported $state object so properties can be mutated freely
// without hitting the "state_invalid_export" rune restriction.

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

// ── Single reactive state object ──────────────────────────────────────────────
// All fields are mutated in-place (obj.prop = val) — never reassign `s` itself.

export const s = $state({
  // Terrain generation config (bound to GeneratePanel inputs)
  config: { ...DEFAULT_CONFIG } as TerrainConfig,

  // Result info shown in header after generation
  terrainInfo: null as GenerateResult | null,

  // Active brush tool
  activeTool: null as BrushTool | null,

  // Brush parameters
  brushRadius:   30,
  brushStrength: 0.5,
  flattenTarget: 0.5,
  noiseScale:    0.05,

  // Status bar
  status:      'Ready',
  statusClass: '',

  // Generation in-progress flag
  generating: false,
});

// ── Actions ───────────────────────────────────────────────────────────────────

export async function generateTerrain() {
  s.generating  = true;
  s.status      = 'Generating terrain…';
  s.statusClass = 'busy';
  try {
    const result = await invoke<GenerateResult>('generate_terrain', { config: s.config });
    s.terrainInfo = result;
    s.status      = `World ${result.worldWidth}×${result.worldHeight} ready`;
    s.statusClass = 'ok';
  } catch (e) {
    s.status      = `Error: ${e}`;
    s.statusClass = 'err';
  } finally {
    s.generating = false;
  }
}

export async function selectTool(tool: BrushTool | null) {
  s.activeTool = tool;
  await invoke('set_active_tool', { tool }).catch(console.error);
}

export async function updateBrushParams() {
  await invoke('set_brush_params', {
    params: {
      radius:        s.brushRadius,
      strength:      s.brushStrength,
      flattenTarget: s.flattenTarget,
      noiseScale:    s.noiseScale,
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
  s.status      = 'Saving…';
  s.statusClass = 'busy';
  try {
    await invoke('save_world', { path });
    s.status      = 'World saved';
    s.statusClass = 'ok';
  } catch (e) {
    s.status      = `Save failed: ${e}`;
    s.statusClass = 'err';
  }
}

export async function loadWorld(path: string) {
  s.status      = 'Loading…';
  s.statusClass = 'busy';
  try {
    const result = await invoke<GenerateResult>('load_world', { path });
    s.terrainInfo = result;
    s.status      = `World ${result.worldWidth}×${result.worldHeight} loaded`;
    s.statusClass = 'ok';
  } catch (e) {
    s.status      = `Load failed: ${e}`;
    s.statusClass = 'err';
  }
}

export async function addVolcanoes(count: number, radius: number, height: number) {
  s.status      = 'Adding volcanoes…';
  s.statusClass = 'busy';
  try {
    await invoke('generate_volcanoes', { config: { count, radius, height } });
    s.status      = `${count} volcano(es) stamped`;
    s.statusClass = 'ok';
  } catch (e) {
    s.status      = `Volcano error: ${e}`;
    s.statusClass = 'err';
  }
}
