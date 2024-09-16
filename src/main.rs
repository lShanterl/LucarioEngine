mod engine;
mod renderer;
mod object;
mod texture;
mod core;

use std::sync::Arc;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::dpi::PhysicalSize;
use crate::core::wgpu_context::WgpuContext;
use crate::core::pipeline_manager::PipelineManager;
use crate::object::Mesh;
use crate::renderer::renderer::Renderer;

pub struct Client {
    wgpu_context: WgpuContext,
    pipeline_manager: PipelineManager,
    renderer: Renderer,
}

impl Client {
    pub fn new(window: Arc<Window>) -> Self {
        let wgpu_context = WgpuContext::new(window.clone());
        let pipeline_manager = PipelineManager::new();
        let renderer = Renderer::new();

        Self {
            wgpu_context,
            pipeline_manager,
            renderer,
        }
    }


    fn create_texture_bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.wgpu_context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }

    fn create_diffuse_bind_group(&self, texture: &texture::Texture, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        self.wgpu_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        })
    }

    fn create_pipeline_layout(&self, bind_group_layout: &wgpu::BindGroupLayout) -> wgpu::PipelineLayout {
        self.wgpu_context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main pipeline layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        })
    }
}

pub fn run() {

    let event_loop = EventLoop::new().unwrap_or_else(|e| panic!("Failed to initialize event loop: {}", e));
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    window.set_title("LucarioProject - Voxel engine");

    let mut client = Client::new(window.clone());

    let shader = client.wgpu_context.device.create_shader_module(wgpu::include_wgsl!("./shaders/test_shader.wgsl"));
    let diffuse_texture = texture::Texture::from_bytes(&client.wgpu_context.device, &client.wgpu_context.queue, include_bytes!("./assets/textures/grid_02.png"), "temporary.png").unwrap();
    let texture_bind_group_layout = client.create_texture_bind_group_layout();
    let diffuse_bind_group = client.create_diffuse_bind_group(&diffuse_texture, &texture_bind_group_layout);

    let render_pipeline_layout = client.create_pipeline_layout(&texture_bind_group_layout);
    let main_pipeline_handle = client.pipeline_manager.create_pipeline(&client.wgpu_context.device, &render_pipeline_layout, &shader, &client.wgpu_context.surface_config);

    let meshes = [&Mesh::new(&client.wgpu_context.device, object::VERTICES, object::INDICES)];
    let mut new_size: Option<PhysicalSize<u32>> = None;

    event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent { ref event, window_id }
        if window_id == client.wgpu_context.get_window().id() =>
            {
                match event {
                    WindowEvent::Resized(size) => new_size = Some(*size),
                    WindowEvent::RedrawRequested => {
                        if let Some(size) = new_size.take() {
                            client.wgpu_context.resize(size);
                        }
                        client.wgpu_context.update();
                        if let Err(e) = client.renderer.render(
                            &client.wgpu_context,
                            client.pipeline_manager.get_pipeline(&main_pipeline_handle).unwrap(),
                            &meshes,
                            &client.wgpu_context.surface,
                            &diffuse_bind_group
                        ) {
                            match e {
                                wgpu::SurfaceError::Lost => client.wgpu_context.resize(client.wgpu_context.size),
                                wgpu::SurfaceError::OutOfMemory => control_flow.exit(),
                                _ => eprintln!("{:?}", e),
                            }
                        }
                    }
                    WindowEvent::CloseRequested => control_flow.exit(),
                    _ => {}
                }
            }
        Event::AboutToWait => client.wgpu_context.get_window().request_redraw(),
        _ => {}
    }).expect("Failed to create a window");
}
fn main() {
    run();
}
