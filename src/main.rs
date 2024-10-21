mod renderer;
mod object;
mod texture;
mod core;
mod utils;

use std::sync::Arc;
use std::time::Duration;
use renderer::{camera::{self, Camera, CameraUniform}, renderer::{RenderContext, Scene}};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::CursorGrabMode;
use crate::core::graphics_resource_manager::{BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager};
use crate::core::input::Input;
use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;
use crate::renderer::renderer::Renderer;
use crate::texture::Texture;

pub struct Client {
    wgpu_context: WgpuContext,
    graphics_resource_manager: GraphicsResourceManager,
    renderer: Renderer,
    camera: Camera,

    //temporary I guess

    camera_buffer: wgpu::Buffer,
    camera_uniform: CameraUniform,
    camera_bind_group_handle: BindGroupHandle,
    camera_bind_group_layout_handle: BindGroupLayoutHandle,

    input: Input,
    is_mouse_focused: bool,

    depth_texture: Texture
}

impl Client {
    pub fn new(window: Arc<Window>) -> Self {
        let wgpu_context = WgpuContext::new(window.clone());
        let mut graphics_resource_manager = GraphicsResourceManager::new();

        let renderer = Renderer::new();
        let camera = Camera::new(
            wgpu_context.get_surface_config().width as f32,
            wgpu_context.get_surface_config().height as f32,
            1.2,
            0.001,
            (0.0, 0.0, 0.0),
            0.0,
            0.0
        );

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = wgpu_context.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout_handle = graphics_resource_manager.create_bind_group_layout(&wgpu_context.device, &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ]);

        let camera_bind_group_handle = graphics_resource_manager.create_bind_group(camera_bind_group_layout_handle, &wgpu_context.device, &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }
        ]);

        let depth_texture = texture::Texture::create_depth_texture(&wgpu_context.device, &wgpu_context.surface_config, "depth_texture");


        Self {
            wgpu_context,
            graphics_resource_manager,
            renderer,
            camera,

            camera_buffer,
            camera_uniform,
            camera_bind_group_handle,
            camera_bind_group_layout_handle,
            //input_controller,
            input: Input::new(),
            is_mouse_focused: false,
            depth_texture
        }
    }

}

pub fn run() {

    std::env::set_var("RUST_BACKTRACE", "1");

    let event_loop = EventLoop::new().unwrap_or_else(|e| panic!("Failed to initialize event loop: {}", e));
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    window.set_title("LucarioProject - Voxel engine");

    let mut client = Client::new(window.clone());

    let shader = client.wgpu_context.device.create_shader_module(wgpu::include_wgsl!("./shaders/test_shader.wgsl"));
    
    let diff_texture = texture::Texture::from_bytes(&client.wgpu_context.device, &client.wgpu_context.queue, include_bytes!("./assets/textures/grid_02.png"), "temporary.png").unwrap();
    let texture_bind_group_layout = client.graphics_resource_manager.create_bind_group_layout(
        &client.wgpu_context.device, 
        &[
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
        ]
    ); // TODO: make some presets for the layout entries

    let diffuse_bind_group_handle = client.graphics_resource_manager.create_bind_group(
        texture_bind_group_layout,
        &client.wgpu_context.device, 
        &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&diff_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&diff_texture.sampler),
            },
        ]
    );
    
    // TODO: use the Arc more frequently
    
    let render_pipeline_layout_handle = client.graphics_resource_manager.create_pipeline_layout(
        &client.wgpu_context.device,
        &[
            &texture_bind_group_layout,
            &client.camera_bind_group_layout_handle
        ]
    );

    let render_pipeline_handle = client.graphics_resource_manager.create_pipeline(
        &client.wgpu_context.device,
        render_pipeline_layout_handle,
        &shader,
        &client.wgpu_context.surface_config,
        &client.depth_texture
    );

    let meshes = [
        //&Mesh::new(&client.wgpu_context.device, object::VERTICES, object::INDICES),
        &Mesh::new(&client.wgpu_context.device, object::CUBE_VERTICES, object::CUBE_INDICES),
        &Mesh::new(&client.wgpu_context.device, object::CONE_VERTICES, object::CONE_INDICES),
    ];
    
    let scene = Scene{
        meshes: &meshes
    };

    let main_render_ctx = client.graphics_resource_manager.create_render_context(
        &render_pipeline_handle,
        &[
            &diffuse_bind_group_handle,
            &client.camera_bind_group_handle
        ]
    );
    

    let mut new_size: Option<PhysicalSize<u32>> = None;
    let mut last_render_time = instant::Instant::now();
    let mut print_framerate = false;
    

    event_loop.run(move |event, control_flow| {
        let mut new_size: Option<winit::dpi::PhysicalSize<u32>> = None;

        match event {
            // Handle device events like mouse motion

            Event::DeviceEvent {ref event, .. } => {
                client.input.handle_device_event(event);
            }

            // Handle window events

            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == client.wgpu_context.get_window().id() => {
                if !client.input.handle_window_event(event) {
                    match event {
                        WindowEvent::Resized(size) => {
                            new_size = Some(*size);
                            client.camera.resize(*size);
                            client.camera_uniform.update_view_proj(&client.camera);
                        }
                        WindowEvent::RedrawRequested => {
                            if let Some(size) = new_size.take() {
                                client.wgpu_context.resize(size);


                            }

                            let now = instant::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;

                            client.input.handle_window_event(event);

                            client.camera.update_camera(&client.input,dt, client.is_mouse_focused);
                            client.camera_uniform.update_view_proj(&client.camera);
                            client.wgpu_context.queue.write_buffer(&client.camera_buffer, 0, bytemuck::cast_slice(&[client.camera_uniform]));

                            if client.input.is_key_just_pressed(winit::keyboard::KeyCode::KeyJ) {
                                println!("{:?}", client.camera.position);
                            }
                            if client.input.is_key_just_pressed(winit::keyboard::KeyCode::KeyF) {
                                print_framerate = !print_framerate;
                            }
                            if client.input.is_mouse_button_just_pressed(MouseButton::Left) {
                                client.is_mouse_focused = !client.is_mouse_focused;

                                if client.is_mouse_focused {
                                    client.wgpu_context.window.set_cursor_grab(CursorGrabMode::Confined)
                                        .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
                                        .unwrap();
                                    client.wgpu_context.window.set_cursor_visible(false);
                                } else {
                                    client.wgpu_context.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                                    client.wgpu_context.window.set_cursor_visible(true);
                                }
                            }
                            
                            
                            if print_framerate {
                                println!("{:?}fps", 1000.0f32 / dt.as_millis() as f32);
                            }


                            // Attempt to render
                            if let Err(e) = client.renderer.render(
                                &client.wgpu_context,
                                &main_render_ctx,
                                &scene,
                                &client.depth_texture,
                            ) {
                                match e {
                                    wgpu::SurfaceError::Lost => {
                                        client.wgpu_context.resize(client.wgpu_context.size);
                                    }
                                    wgpu::SurfaceError::OutOfMemory => control_flow.exit(),
                                    _ => eprintln!("{:?}", e),
                                }
                            }

                            client.input.reset();
                        }
                        WindowEvent::CloseRequested => {
                            control_flow.exit();
                        }
                        _ => {}
                    }
                }
            }

            // Force redraw when about to wait
            Event::AboutToWait => {
                client.wgpu_context.get_window().request_redraw();
            }

            _ => {}
        }
    }).expect("Failed to create a window");

}
fn main() {
    run();
}
