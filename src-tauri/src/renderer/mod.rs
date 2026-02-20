mod context;
mod pipelines;
mod textures;
mod viewport;
mod brush_gpu;
mod undo_gpu;

pub use context::WgpuContext;

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::AppHandle;
use crate::state::AppState;

use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, MouseButton, ElementState, MouseScrollDelta},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

/// Entry point for the dedicated wgpu render thread.
/// Creates a native window (no WebView), initialises wgpu on it,
/// then runs the event/render loop forever.
pub fn run_render_window(_app: AppHandle, state: Arc<Mutex<AppState>>) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = RenderApp {
        state,
        ctx: None,
        window: None,
        mouse_pos: (0.0, 0.0),
        last_pan: (0.0, 0.0),
        right_held: false,
        left_held: false,
    };

    event_loop.run_app(&mut app).expect("event loop error");
}

// ── winit ApplicationHandler ──────────────────────────────────────────────────

struct RenderApp {
    state:      Arc<Mutex<AppState>>,
    ctx:        Option<WgpuContext>,
    window:     Option<Arc<Window>>,
    mouse_pos:  (f32, f32),
    last_pan:   (f32, f32),
    right_held: bool,
    left_held:  bool,
}

impl ApplicationHandler for RenderApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; } // already initialised

        let attrs = WindowAttributes::default()
            .with_title("WorldWeaver — Map")
            .with_inner_size(winit::dpi::LogicalSize::new(1200u32, 900u32))
            .with_position(winit::dpi::LogicalPosition::new(300, 0));

        let window = Arc::new(
            event_loop.create_window(attrs).expect("failed to create window")
        );

        let ctx = pollster::block_on(WgpuContext::new(window.clone()))
            .expect("failed to init wgpu");

        // Initialise viewport canvas size from window
        let size = window.inner_size();
        self.state.lock().viewport.canvas_size = [size.width as f32, size.height as f32];

        self.window = Some(window);
        self.ctx    = Some(ctx);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let (ctx, window) = match (self.ctx.as_mut(), self.window.as_ref()) {
            (Some(c), Some(w)) => (c, w),
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                ctx.resize(size.width, size.height);
                let mut st = self.state.lock();
                st.viewport.canvas_size = [size.width as f32, size.height as f32];
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (px, py) = (position.x as f32, position.y as f32);

                // Right-drag → pan
                if self.right_held {
                    let dx = px - self.last_pan.0;
                    let dy = py - self.last_pan.1;
                    let mut st = self.state.lock();
                    viewport::pan(&mut st.viewport, dx, dy);
                }
                self.last_pan = (px, py);
                self.mouse_pos = (px, py);

                // Left-drag → brush paint
                if self.left_held {
                    let mut st = self.state.lock();
                    if let Some(ref tool) = st.brush.active_tool.clone() {
                        let (wx, wz) = st.viewport.screen_to_world(px, py);
                        st.brush.cursor_world = Some((wx, wz));
                        if let Some(ref terrain) = st.terrain {
                            let ww = terrain.config.world_width as f32;
                            let wh = terrain.config.world_height as f32;
                            if wx >= 0.0 && wx < ww && wz >= 0.0 && wz < wh {
                                drop(st);
                                brush_gpu::dispatch_brush(ctx, &self.state, px, py);
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state: btn_state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        self.left_held = btn_state == ElementState::Pressed;
                        if self.left_held {
                            undo_gpu::snapshot_brush_region(ctx, &self.state, self.mouse_pos.0, self.mouse_pos.1);
                            brush_gpu::dispatch_brush(ctx, &self.state, self.mouse_pos.0, self.mouse_pos.1);
                        }
                    }
                    MouseButton::Right => {
                        self.right_held = btn_state == ElementState::Pressed;
                        self.last_pan = self.mouse_pos;
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) * 0.01,
                };
                let factor = if scroll > 0.0 {
                    1.1f32.powf(scroll)
                } else {
                    0.9f32.powf(-scroll)
                };
                let mut st = self.state.lock();
                st.viewport.zoom_at(self.mouse_pos.0, self.mouse_pos.1, factor);
            }

            WindowEvent::RedrawRequested => {
                // Handle pending undo from UI panel (Ctrl+Z)
                {
                    let pending = self.state.lock().undo_stack.pending_undo;
                    if pending {
                        self.state.lock().undo_stack.pending_undo = false;
                        undo_gpu::apply_undo(ctx, &self.state);
                    }
                }

                // Upload terrain to GPU if dirty
                {
                    let mut st = self.state.lock();
                    if let Some(ref mut t) = st.terrain {
                        if t.dirty {
                            ctx.upload_heightmap(&t.heights, t.config.world_width, t.config.world_height);
                            ctx.upload_flow(&t.flow, t.config.world_width, t.config.world_height);
                            t.dirty = false;
                        }
                    }
                }

                let vp          = self.state.lock().viewport.clone();
                let terrain_cfg = self.state.lock().terrain.as_ref().map(|t| t.config.clone());
                let brush       = self.state.lock().brush.clone();

                ctx.render(&vp, terrain_cfg.as_ref(), &brush);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
