use std::sync::Arc;
use futures::executor::block_on;
use winit::{dpi::PhysicalSize, window::Window};
use wgpu;


pub struct WgpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub window: Arc<Window>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl WgpuContext {
    pub(crate) fn update(&mut self) {
        //todo!()
    }
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let (device, queue, surface, surface_config) = init_wgpu(window.clone());

        Self {
            device,
            queue,
            surface,
            surface_config,
            window,
            size,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub(crate) fn get_surface_config(&self) -> wgpu::SurfaceConfiguration {
        self.surface_config.clone()
    }

    pub(crate) fn get_window(&self) -> Arc<Window> {
        self.window.clone()
    }
}

fn init_wgpu(window: Arc<Window>) -> (wgpu::Device, wgpu::Queue, wgpu::Surface<'static>, wgpu::SurfaceConfiguration){
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor{
        backends: wgpu::Backends::DX12, // need to specify the backend explicitly as of https://github.com/gfx-rs/wgpu/issues/3959
        ..Default::default()
    });
    let surface = instance.create_surface(window.clone()).expect("Failed to create a surface");

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions{
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    })).expect("Failed to create an adapter");

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor{
                required_features: wgpu::Features::empty(), // change this in the future to POLYGON_MODE_LINE (wireframe)
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                label: None,
            },
            None,
        )).unwrap();

    let surface_caps = surface.get_capabilities(&adapter);

    let surface_formats = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let surface_config = wgpu::SurfaceConfiguration{
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_formats,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &surface_config);

    (device, queue, surface, surface_config)
}