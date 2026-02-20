use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::state::{ViewportState, BrushState, BrushTool};
use crate::terrain::TerrainConfig;

/// All wgpu objects needed to render the terrain.
pub struct WgpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pub width: u32,
    pub height: u32,

    // Terrain render pipeline
    render_pipeline: wgpu::RenderPipeline,

    // GPU textures
    pub heightmap_texture: wgpu::Texture,
    pub heightmap_view: wgpu::TextureView,
    pub flow_texture: wgpu::Texture,
    pub flow_view: wgpu::TextureView,
    color_ramp_texture: wgpu::Texture,
    color_ramp_view: wgpu::TextureView,

    // Heightmap dimensions (may differ from canvas size)
    pub world_width: u32,
    pub world_height: u32,

    // Uniform buffer: viewport + render params
    uniform_buffer: wgpu::Buffer,
    render_bind_group: wgpu::BindGroup,
    render_bind_group_layout: wgpu::BindGroupLayout,

    // Compute brush pipelines (one per tool)
    pub brush_raise_pipeline: wgpu::ComputePipeline,
    pub brush_smooth_pipeline: wgpu::ComputePipeline,
    pub brush_flatten_pipeline: wgpu::ComputePipeline,
    pub brush_noise_pipeline: wgpu::ComputePipeline,
    pub brush_erode_pipeline: wgpu::ComputePipeline,
    pub brush_params_buffer: wgpu::Buffer,
    pub brush_bind_group_layout: wgpu::BindGroupLayout,
    pub brush_bind_group: wgpu::BindGroup,

    // Samplers
    sampler_linear: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
    /// NonFiltering sampler used for R32Float textures (heightmap, flow)
    sampler_nonfilter: wgpu::Sampler,
}

// ── Uniform structs (must be Pod + Zeroable for bytemuck) ────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TerrainUniforms {
    pub translate:      [f32; 2],
    pub scale:          f32,
    pub _pad0:          f32,
    pub canvas_size:    [f32; 2],
    pub world_size:     [f32; 2],
    pub sea_level:      f32,
    pub max_elevation:  f32,
    pub contour_interval: f32,
    pub has_flow:       f32,
    pub hide_underwater: f32,
    pub sun_azimuth:    f32,
    pub _pad1:          [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BrushParamsGpu {
    pub center:   [f32; 2],
    pub radius:   f32,
    pub strength: f32,
    pub flatten_target: f32,
    pub noise_scale: f32,
    pub _pad: [f32; 2],
}

impl WgpuContext {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("no suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WorldWeaver Device"),
                    // TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES is needed for:
                    //   • R32Float as a read-write storage texture (compute brushes)
                    //   • R32Float as a filterable texture (render sampling)
                    // This is a native-only feature, available on DX12/Vulkan/Metal.
                    required_features:
                        wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // R32Float textures do NOT support filtering on most adapters unless
        // TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES is enabled.  Use a
        // NonFiltering sampler (Nearest) for the heightmap and flow textures.
        let sampler_nonfilter = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nonfilter sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ── Placeholder textures (1×1) — overwritten when terrain generates ──
        let (heightmap_texture, heightmap_view) =
            Self::create_heightmap_texture(&device, 1, 1);
        let (flow_texture, flow_view) =
            Self::create_flow_texture(&device, 1, 1);
        let (color_ramp_texture, color_ramp_view) =
            Self::create_color_ramp(&device, &queue);

        // ── Uniform buffer ────────────────────────────────────────────────────
        let default_uniforms = TerrainUniforms {
            translate: [0.0, 0.0],
            scale: 1.0,
            _pad0: 0.0,
            canvas_size: [width as f32, height as f32],
            world_size: [1.0, 1.0],
            sea_level: 0.5,
            max_elevation: 4000.0,
            contour_interval: 100.0,
            has_flow: 0.0,
            hide_underwater: 0.0,
            sun_azimuth: 315.0,
            _pad1: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("TerrainUniforms"),
            contents: bytemuck::bytes_of(&default_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Brush params buffer ───────────────────────────────────────────────
        let default_brush = BrushParamsGpu {
            center: [0.0, 0.0],
            radius: 30.0,
            strength: 0.5,
            flatten_target: 0.5,
            noise_scale: 0.05,
            _pad: [0.0; 2],
        };
        let brush_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BrushParams"),
            contents: bytemuck::bytes_of(&default_brush),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Render bind group layout ──────────────────────────────────────────
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("render bgl"),
                entries: &[
                    // binding 0: heightmap texture (R32Float — NonFilterable)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding 1: heightmap sampler (NonFiltering to match R32Float)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // binding 2: color ramp (Rgba8Unorm — filterable ok)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding 3: flow texture (R32Float — NonFilterable)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding 4: uniforms (needed by both vertex and fragment stages)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_bind_group = Self::make_render_bind_group(
            &device,
            &render_bind_group_layout,
            &heightmap_view,
            &sampler_nonfilter,
            &color_ramp_view,
            &flow_view,
            &uniform_buffer,
        );

        // ── Brush compute bind group layout ───────────────────────────────────
        let brush_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("brush bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let brush_bind_group = Self::make_brush_bind_group(
            &device, &brush_bind_group_layout, &heightmap_view, &brush_params_buffer,
        );

        // ── Render pipeline ───────────────────────────────────────────────────
        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/terrain.wgsl").into()
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain render layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ── Compute pipelines ─────────────────────────────────────────────────
        let make_compute = |src: &str, label: &str| -> wgpu::ComputePipeline {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[&brush_bind_group_layout],
                push_constant_ranges: &[],
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module: &module,
                entry_point: "cs_main",
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let brush_raise_pipeline   = make_compute(include_str!("shaders/brush_raise.wgsl"),   "brush_raise");
        let brush_smooth_pipeline  = make_compute(include_str!("shaders/brush_smooth.wgsl"),  "brush_smooth");
        let brush_flatten_pipeline = make_compute(include_str!("shaders/brush_flatten.wgsl"), "brush_flatten");
        let brush_noise_pipeline   = make_compute(include_str!("shaders/brush_noise.wgsl"),   "brush_noise");
        let brush_erode_pipeline   = make_compute(include_str!("shaders/brush_erode.wgsl"),   "brush_erode");

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            width,
            height,
            render_pipeline,
            heightmap_texture,
            heightmap_view,
            flow_texture,
            flow_view,
            color_ramp_texture,
            color_ramp_view,
            world_width: 1,
            world_height: 1,
            uniform_buffer,
            render_bind_group,
            render_bind_group_layout,
            brush_raise_pipeline,
            brush_smooth_pipeline,
            brush_flatten_pipeline,
            brush_noise_pipeline,
            brush_erode_pipeline,
            brush_params_buffer,
            brush_bind_group_layout,
            brush_bind_group,
            sampler_linear,
            sampler_nearest,
            sampler_nonfilter,
        })
    }

    // ── Public API ─────────────────────────────────────────────────────────────

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Upload (or re-upload) the full flat f32 heightmap to the GPU R32Float texture.
    pub fn upload_heightmap(&mut self, heights: &[f32], w: u32, h: u32) {
        if self.world_width != w || self.world_height != h {
            // Recreate texture at new size
            let (tex, view) = Self::create_heightmap_texture(&self.device, w, h);
            self.heightmap_texture = tex;
            self.heightmap_view = view;
            self.world_width = w;
            self.world_height = h;
            self.rebuild_bind_groups();
        }

        let bytes: &[u8] = bytemuck::cast_slice(heights);
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.heightmap_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4), // R32Float = 4 bytes/pixel
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// Upload the normalised flow accumulation texture (R32Float, same size as heightmap).
    pub fn upload_flow(&mut self, flow: &[f32], w: u32, h: u32) {
        if flow.is_empty() { return; }

        let (tex, view) = Self::create_flow_texture(&self.device, w, h);
        self.flow_texture = tex;
        self.flow_view = view;
        self.rebuild_bind_groups();

        let bytes: &[u8] = bytemuck::cast_slice(flow);
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.flow_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// Main render call — call once per frame.
    pub fn render(&mut self, vp: &ViewportState, config: Option<&TerrainConfig>, _brush: &BrushState) {
        // Update uniforms
        let sea_level = config.map(|c| c.sea_level).unwrap_or(0.5);
        let max_elev  = config.map(|c| c.max_elevation).unwrap_or(4000.0);
        let world_w   = config.map(|c| c.world_width as f32).unwrap_or(1.0);
        let world_h   = config.map(|c| c.world_height as f32).unwrap_or(1.0);

        let uniforms = TerrainUniforms {
            translate: vp.translate,
            scale: vp.scale,
            _pad0: 0.0,
            canvas_size: vp.canvas_size,
            world_size: [world_w, world_h],
            sea_level,
            max_elevation: max_elev,
            contour_interval: 100.0,
            has_flow: if self.world_width > 1 { 1.0 } else { 0.0 },
            hide_underwater: 0.0,
            sun_azimuth: 315.0,
            _pad1: [0.0; 2],
        };
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Render pass
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            Err(e) => { log::warn!("Surface error: {e}"); return; }
        };

        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.08, b: 0.15, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.render_bind_group, &[]);
            // Full-screen triangle: 3 vertices, no vertex buffer
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        output.present();
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn create_heightmap_texture(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("heightmap"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // R32Float gives exact float precision — no R+G/255 hack needed
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    fn create_flow_texture(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flow"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    fn create_color_ramp(device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::Texture, wgpu::TextureView) {
        // Build hypsometric color ramp identical to the current project
        let stops: &[(f32, [u8; 3])] = &[
            (0.00, [2,  20, 50]),
            (0.25, [4,  30, 66]),
            (0.35, [26, 84,144]),
            (0.45, [43,123,185]),
            (0.48, [136,201,240]),
            (0.495,[200,220,235]),
            (0.50, [245,230,200]),
            (0.51, [225,235,190]),
            (0.54, [212,231,176]),
            (0.58, [184,216,139]),
            (0.65, [154,199,119]),
            (0.70, [135,190,105]),
            (0.75, [180,170,130]),
            (0.80, [170,155,120]),
            (0.85, [155,140,110]),
            (0.89, [140,130,105]),
            (0.92, [180,175,165]),
            (0.95, [212,207,201]),
            (0.98, [235,232,228]),
            (1.00, [255,255,255]),
        ];

        let mut ramp = vec![0u8; 256 * 4];
        for i in 0..256usize {
            let t = i as f32 / 255.0;
            let mut lo = stops[0];
            let mut hi = stops[stops.len()-1];
            for w in stops.windows(2) {
                if t >= w[0].0 && t <= w[1].0 {
                    lo = w[0]; hi = w[1]; break;
                }
            }
            let f = if hi.0 > lo.0 { (t - lo.0) / (hi.0 - lo.0) } else { 0.0 };
            ramp[i*4+0] = (lo.1[0] as f32 * (1.0-f) + hi.1[0] as f32 * f) as u8;
            ramp[i*4+1] = (lo.1[1] as f32 * (1.0-f) + hi.1[1] as f32 * f) as u8;
            ramp[i*4+2] = (lo.1[2] as f32 * (1.0-f) + hi.1[2] as f32 * f) as u8;
            ramp[i*4+3] = 255;
        }

        let tex = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("color_ramp"),
                size: wgpu::Extent3d { width: 256, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &ramp,
        );
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    fn make_render_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        heightmap_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        color_ramp_view: &wgpu::TextureView,
        flow_view: &wgpu::TextureView,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(heightmap_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(color_ramp_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(flow_view) },
                wgpu::BindGroupEntry { binding: 4, resource: uniform_buffer.as_entire_binding() },
            ],
        })
    }

    fn make_brush_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        heightmap_view: &wgpu::TextureView,
        brush_params: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brush bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(heightmap_view) },
                wgpu::BindGroupEntry { binding: 1, resource: brush_params.as_entire_binding() },
            ],
        })
    }

    /// Rebuild bind groups after texture re-creation (new world size).
    pub fn rebuild_bind_groups(&mut self) {
        self.render_bind_group = Self::make_render_bind_group(
            &self.device,
            &self.render_bind_group_layout,
            &self.heightmap_view,
            &self.sampler_nonfilter,
            &self.color_ramp_view,
            &self.flow_view,
            &self.uniform_buffer,
        );
        self.brush_bind_group = Self::make_brush_bind_group(
            &self.device,
            &self.brush_bind_group_layout,
            &self.heightmap_view,
            &self.brush_params_buffer,
        );
    }

    /// Dispatch the appropriate compute shader for the given brush tool.
    pub fn dispatch_compute_brush(&self, tool: &BrushTool) {
        let pipeline = match tool {
            BrushTool::Raise | BrushTool::Lower => &self.brush_raise_pipeline,
            BrushTool::Smooth  => &self.brush_smooth_pipeline,
            BrushTool::Flatten => &self.brush_flatten_pipeline,
            BrushTool::Noise   => &self.brush_noise_pipeline,
            BrushTool::Erode   => &self.brush_erode_pipeline,
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.brush_bind_group, &[]);
            let dx = (self.world_width + 15) / 16;
            let dy = (self.world_height + 15) / 16;
            pass.dispatch_workgroups(dx, dy, 1);
        }
        self.queue.submit([encoder.finish()]);
    }
}
