use anyhow::{Result, bail};
use image::GenericImageView;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupLayoutEntry,
    BindingResource::{self},
    BlendState, Buffer, BufferUsages, Color, ColorTargetState, ColorWrites, Device,
    DeviceDescriptor, ExperimentalFeatures, Extent3d, Features, FragmentState, Limits,
    MultisampleState, Operations, Origin3d, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPassColorAttachment, RenderPipeline, RenderPipelineDescriptor,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderStages, Surface, SurfaceConfiguration,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureDescriptor, TextureUsages,
    TextureViewDescriptor, VertexState,
    util::{BufferInitDescriptor, DeviceExt},
};

use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::vertex::Vertex;

pub struct State {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
    color: wgpu::Color,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_vertices: u32,
    num_indicies: u32,
    diffuse_bind_group: BindGroup,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            display: None,
            backend_options: Default::default(),
        });

        // Surface is he part of the window we draw to. We need it to draw directly to the screen.
        let surface = instance.create_surface(window.clone()).unwrap();

        // the adapter is a handle for our actual graphics card. You can use this to get information
        // from the gpu
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: Default::default(),
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: None,
                required_features: Features::empty(), // allows us to specify what features we need.
                // we can get a list of features supported by
                // adapter.features()
                experimental_features: ExperimentalFeatures::disabled(), // features that are not
                // stable yet
                required_limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                memory_hints: Default::default(), // hints to the device about the memory allocation
                // stratergy
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT, // descibes how the surface will be used.
            format: surface_format,                  // how SurfaceTextures are stored on the  GPU,
            width: size.width,                       // width and height should not be zero
            height: size.height,
            present_mode: surface_caps.present_modes[0], /*determines how to sync the surface with
                                                         the display, current option is vsync. to let the users pick we can list all get_capabilities
                                                         using &surface_caps.present_modes
                                                         */
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Srgb, // Srgb for now
        };

        let diffuse_bytes = include_bytes!("minecraft-soil.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();
        let dimmentions = diffuse_image.dimensions();

        let texture_size = Extent3d {
            width: dimmentions.0,
            height: dimmentions.1,
            // all textures are stored as 3d, we represent our 2D texture by setting depth to 1
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // texture binding tells that we want to use this texture in shaders
            // copy_dst means tha twe want to copy data to this texture
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimmentions.0),
                rows_per_image: Some(dimmentions.1),
            },
            texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            // determines what to do if the sampler gets a texture coordinate thats outside the
            // textures itself. Possible option: ClampToEdge, Repear, MirrorRepeat
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // describe what to do when the sample footprint is smaller or larger than one textel
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    // has two slots one for the the texture and one for the sampler.
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT, // makes these only visible to the fragment
                        // shader
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[Some(&texture_bind_group_layout)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fr_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let new_triangle = Vertex::new_triangle();

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&(new_triangle.0)),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&(new_triangle.1)),
            usage: BufferUsages::INDEX,
        });

        Ok(Self {
            window: window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            color: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_vertices: new_triangle.0.len() as u32,
            num_indicies: new_triangle.1.len() as u32,
            diffuse_bind_group: diffuse_bind_group,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    pub fn render(&mut self) -> Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                bail!("lost device");
            }
        };

        let veiw = output
            .texture
            .create_view(&wgpu::wgt::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &veiw,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(self.color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indicies, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(output);

        Ok(())
    }

    pub fn update(&mut self) {}
}
