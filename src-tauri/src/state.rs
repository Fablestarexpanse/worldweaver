use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use crate::terrain::TerrainConfig;

/// Shared application state — held behind Arc<Mutex<>> and accessed
/// from both the wgpu render thread and Tauri command handlers.
#[derive(Default)]
pub struct AppState {
    pub terrain: Option<TerrainData>,
    pub brush: BrushState,
    pub viewport: ViewportState,
    pub undo_stack: UndoStack,
}

// ── Terrain ──────────────────────────────────────────────────────────────────

/// The full world heightmap as a flat f32 array.
/// No chunks — just world_width × world_height values in row-major order.
#[derive(Clone)]
pub struct TerrainData {
    pub config: TerrainConfig,
    /// Heights in [0.0, 1.0]. Length = world_width * world_height.
    pub heights: Vec<f32>,
    /// Flow accumulation, normalised [0.0, 1.0]. Same size as heights.
    pub flow: Vec<f32>,
    /// Biome IDs, one per pixel.
    pub biomes: Vec<u8>,
    /// Marks the GPU as needing a full texture re-upload.
    pub dirty: bool,
}

impl TerrainData {
    pub fn new(config: TerrainConfig) -> Self {
        let n = (config.world_width * config.world_height) as usize;
        Self {
            heights: vec![0.0; n],
            flow: vec![0.0; n],
            biomes: vec![0; n],
            dirty: true,
            config,
        }
    }
}

// ── Brush ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrushTool {
    Raise,
    Lower,
    Smooth,
    Flatten,
    Erode,
    Noise,
}

impl Default for BrushTool {
    fn default() -> Self { BrushTool::Raise }
}

#[derive(Debug, Clone)]
pub struct BrushState {
    pub active_tool: Option<BrushTool>,
    pub radius: f32,
    pub strength: f32,
    pub flatten_target: f32,
    pub noise_scale: f32,
    /// True while the left mouse button is held over the render window.
    pub is_painting: bool,
    /// World-space cursor position, updated every mouse-move.
    pub cursor_world: Option<(f32, f32)>,
}

impl Default for BrushState {
    fn default() -> Self {
        Self {
            active_tool: None,
            radius: 30.0,
            strength: 0.5,
            flatten_target: 0.5,
            noise_scale: 0.05,
            is_painting: false,
            cursor_world: None,
        }
    }
}

// ── Viewport ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ViewportState {
    pub translate: [f32; 2],
    pub scale: f32,
    pub canvas_size: [f32; 2],
    pub min_scale: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            translate: [0.0, 0.0],
            scale: 1.0,
            canvas_size: [1200.0, 900.0],
            min_scale: 0.1,
        }
    }
}

impl ViewportState {
    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        (
            (sx - self.translate[0]) / self.scale,
            (sy - self.translate[1]) / self.scale,
        )
    }

    pub fn fit_world(&mut self, world_w: f32, world_h: f32) {
        let sx = self.canvas_size[0] / world_w;
        let sy = self.canvas_size[1] / world_h;
        let fit = sx.min(sy);
        self.min_scale = fit;
        self.scale = fit;
        let sw = world_w * fit;
        let sh = world_h * fit;
        self.translate[0] = (self.canvas_size[0] - sw) / 2.0;
        self.translate[1] = (self.canvas_size[1] - sh) / 2.0;
    }

    pub fn zoom_at(&mut self, sx: f32, sy: f32, factor: f32) {
        let new_scale = (self.scale * factor).max(self.min_scale).min(50.0);
        let ratio = new_scale / self.scale;
        self.translate[0] = sx - (sx - self.translate[0]) * ratio;
        self.translate[1] = sy - (sy - self.translate[1]) * ratio;
        self.scale = new_scale;
        self.clamp_translate();
    }

    fn clamp_translate(&mut self) {
        // Prevent the world from flying off screen entirely — keeps at least
        // a small portion visible. Full clamping happens in viewport.rs.
    }
}

// ── Undo ─────────────────────────────────────────────────────────────────────

pub struct UndoRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// zstd-compressed f32 heights for this region.
    pub compressed: Vec<u8>,
}

pub struct UndoStack {
    pub regions: VecDeque<UndoRegion>,
    pub max_depth: usize,
    /// Set to true by the Tauri undo command; cleared by the render thread.
    pub pending_undo: bool,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self {
            regions: VecDeque::new(),
            max_depth: 50,
            pending_undo: false,
        }
    }
}

impl UndoStack {
    pub fn push(&mut self, region: UndoRegion) {
        if self.regions.len() >= self.max_depth {
            self.regions.pop_front();
        }
        self.regions.push_back(region);
    }

    pub fn pop(&mut self) -> Option<UndoRegion> {
        self.regions.pop_back()
    }
}
