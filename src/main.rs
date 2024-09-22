mod renderer;
mod object;
mod texture;
mod core;

use std::sync::Arc;
use renderer::{camera::{self, Camera, CameraUniform}, renderer::{RenderContext, Scene}};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::dpi::PhysicalSize;
use crate::core::graphics_resource_manager::GraphicsResourceManager;
use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;
use crate::renderer::renderer::Renderer;


pub struct Client {
    wgpu_context: WgpuContext,
    graphics_resource_manager: GraphicsResourceManager,
    renderer: Renderer,
    camera: Camera,
}

impl Client {
    pub fn new(window: Arc<Window>) -> Self {
        let wgpu_context = WgpuContext::new(window.clone());
        let graphics_resource_manager = GraphicsResourceManager::new();

        let renderer = Renderer::new();
        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: wgpu_context.get_surface_config().width as f32 / wgpu_context.get_surface_config().height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        //let camera_uniform_handle = resource_manager.create_uniform_buffer(&wgpu_context.device, &camera_uniform);
 
        
        Self {
            wgpu_context,
            graphics_resource_manager,
            renderer,
            camera,
        }
    }
}

pub fn run() {

    let event_loop = EventLoop::new().unwrap_or_else(|e| panic!("Failed to initialize event loop: {}", e));
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    window.set_title("LucarioProject - Voxel engine");

    let mut client = Client::new(window.clone());

    let shader = client.wgpu_context.device.create_shader_module(wgpu::include_wgsl!("./shaders/test_shader.wgsl"));

    let diff_texture = texture::Texture::from_bytes(&client.wgpu_context.device, &client.wgpu_context.queue, include_bytes!("./assets/textures/grid_02.png"), "temporary.png").unwrap();
    let texture_bind_group_layout = client.graphics_resource_manager.create_bind_group_layout(&client.wgpu_context.device, &[
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
    ]); // TODO: make some presets for the layout entries

    let diffuse_bind_group = client.graphics_resource_manager.create_bind_group(texture_bind_group_layout, &client.wgpu_context.device, &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&diff_texture.view),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&diff_texture.sampler),
        },
    ]);
    
    // TODO: use the Arc more frequently

    let render_pipeline_layout_handle = client.graphics_resource_manager.create_pipeline_layout(&client.wgpu_context.device, &[&texture_bind_group_layout]);
    let render_pipeline_handle = client.graphics_resource_manager.create_pipeline(&client.wgpu_context.device, render_pipeline_layout_handle, &shader, &client.wgpu_context.surface_config);

    let meshes = [&Mesh::new(&client.wgpu_context.device, object::VERTICES, object::INDICES)];
    let scene = Scene{
        meshes: &meshes
    };
    let main_render_ctx = client.graphics_resource_manager.create_render_context(&render_pipeline_handle, &diffuse_bind_group);
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
                            client.camera.resize(size);
                        }
                        client.wgpu_context.update();
                        if let Err(e) = client.renderer.render(
                            &client.wgpu_context,
                            &main_render_ctx,
                            &scene,
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
