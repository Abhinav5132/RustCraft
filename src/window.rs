use anyhow::Result;
use std::sync::Arc;
use wgpu::{
    Color, Device, DeviceDescriptor, ExperimentalFeatures, Features, Limits, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages,
};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct State {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
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

        Ok(Self {
            window: window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
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

    pub fn render(&mut self) {
        self.window.request_redraw();
    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {}
}
