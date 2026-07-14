use anyhow::{Result, bail};
use cgmath::Deg;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupLayoutEntry,
    BindingResource::{self},
    BlendState, Buffer, BufferUsages, Color, ColorTargetState, ColorWrites, Device,
    DeviceDescriptor, ExperimentalFeatures, Features, FragmentState, Limits, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPassColorAttachment, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderStages, Surface, SurfaceConfiguration, TextureUsages, VertexState,
    util::{BufferInitDescriptor, DeviceExt},
};

use winit::{
    event::{MouseButton, MouseScrollDelta},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::Window,
};

use crate::{
    camera::{
        camera::Camera, camera_controller::CameraController, camera_state::CameraState,
        projection::Projection,
    },
    inputs::keybinds::KeyBindings,
    texture,
    vertex::Vertex,
};

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
    texture: texture::Texture,
    camera: Camera,
    camera_state: CameraState,
    pub camera_controller: CameraController,
    pub mouse_pressed: bool,
    projection: Projection,
    bindings: KeyBindings,
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

        let diffuse_bytes = include_bytes!("minecraft-soil.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "grass_block").unwrap();
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let camera = Camera::new((0.0, 5.0, 10.0), Deg(-90.0), Deg(-20.0));
        let projection = Projection::new(
            config.width as f32,
            config.height as f32,
            Deg(45.0),
            0.1,
            100.0,
        );
        let camera_controller = CameraController::new(4.0, 0.4);
        let mut camera_state = CameraState::get_camera_init_state(&device);
        camera_state
            .camera_uniform
            .update_view_proj(&camera, &projection);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[
                Some(&texture_bind_group_layout),
                Some(&camera_state.camera_bind_group_layout),
            ],
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

        let bindings = KeyBindings::default(); // Change this later

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
            texture: diffuse_texture,
            camera: camera,
            camera_state: camera_state,
            camera_controller: camera_controller,
            mouse_pressed: false,
            projection: projection,
            bindings: bindings,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.projection.resize(width as f32, height as f32);
            self.is_surface_configured = true;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if let Some(action) = self.bindings.get_action(&code) {
            if !self.camera_controller.handle_key(is_pressed, action) {
                match (code, is_pressed) {
                    (KeyCode::Escape, true) => event_loop.exit(),
                    _ => {}
                }
            }
        } else {
            return;
        }
    }

    pub fn handle_mouse(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Middle => self.mouse_pressed = pressed, //TODO move this key defination
            //into keybinds struct
            _ => {}
        }
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.camera_controller.handle_mouse_scroll(delta);
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
            render_pass.set_bind_group(1, &self.camera_state.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indicies, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(output);

        Ok(())
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_state
            .camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            self.camera_state.get_camera_buffer(),
            0,
            bytemuck::cast_slice(&[self.camera_state.camera_uniform]),
        );
    }
}
