mod renderer;
mod object;
mod texture;
mod core;

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
use winit::keyboard::PhysicalKey;
use crate::core::graphics_resource_manager::{BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager};
use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;
use crate::renderer::renderer::Renderer;

pub struct InputController{
    left_pressed: bool,
    right_pressed: bool,

    last_mouse_pos: PhysicalPosition<f64>,
}

impl InputController{
    pub fn update(&self, camera: &mut Camera, wgpu_context: &mut WgpuContext, camera_uniform: &mut CameraUniform, camera_buffer: &mut wgpu::Buffer, dt: Duration) {
        camera.update_camera(dt);
        camera_uniform.update_view_proj(camera);
        wgpu_context.queue.write_buffer(camera_buffer, 0, bytemuck::cast_slice(&[*camera_uniform]));
    }
    pub fn input(&mut self, event: &WindowEvent, camera: &mut Camera) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                },
                ..
            } => camera.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                camera.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.left_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::MouseInput { button: MouseButton::Right, state, .. } => {self.right_pressed = *state == ElementState::Pressed; true}
            _ => false,
        }
    }
}

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

    input_controller: InputController,
}

impl Client {
    pub fn new(window: Arc<Window>) -> Self {
        let wgpu_context = WgpuContext::new(window.clone());
        let mut graphics_resource_manager = GraphicsResourceManager::new();

        let renderer = Renderer::new();
        let camera = Camera::new(
            wgpu_context.get_surface_config().width as f32,
            wgpu_context.get_surface_config().height as f32,
            0.2,
            1.1,
            (0.0, 0.0, 0.0),
            cgmath::Deg(-90.0),
            cgmath::Deg(-20.0)
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

        let input_controller = InputController{right_pressed: false, left_pressed: false, last_mouse_pos: PhysicalPosition::new(0.0, 0.0)};
        
        Self {
            wgpu_context,
            graphics_resource_manager,
            renderer,
            camera,

            camera_buffer,
            camera_uniform,
            camera_bind_group_handle,
            camera_bind_group_layout_handle,
            input_controller,
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
        &client.wgpu_context.surface_config
    );

    let meshes = [
        &Mesh::new(&client.wgpu_context.device, object::VERTICES, object::INDICES)
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
    let (mut dx, mut dy) = (0.0,0.0);

    event_loop.run(move |event, control_flow| {
        let mut new_size: Option<winit::dpi::PhysicalSize<u32>> = None;

        match event {
            // Handle device events like mouse motion
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if client.input_controller.left_pressed {
                    //(dx, dy) = delta;
                    client.camera.process_mouse(delta.0, delta.1);
                    window.set_cursor_position(PhysicalPosition::new(client.wgpu_context.window.inner_size().width as f64 / 2.0, client.wgpu_context.window.inner_size().height as f64 / 2.0)).unwrap();
                }
                else if client.input_controller.right_pressed{
                    client.wgpu_context.is_cursor_visible = !client.wgpu_context.is_cursor_visible;
                    client.wgpu_context.window.set_cursor_visible(client.wgpu_context.is_cursor_visible);
                }
            }

            // Handle window events
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == client.wgpu_context.get_window().id() => {
                if !client.input_controller.input(event, &mut client.camera) {
                    match event {
                        WindowEvent::Resized(size) => {
                            new_size = Some(*size);
                        }
                        WindowEvent::RedrawRequested => {
                            if let Some(size) = new_size.take() {
                                client.wgpu_context.resize(size);
                                client.camera.resize(size);
                            }

                            let now = instant::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;

                            println!("{:?}",dt);

                            //client.camera.process_mouse(dx,dy);
                            //(dx,dy) = (0.0, 0.0);

                            client.input_controller.update(
                                &mut client.camera,
                                &mut client.wgpu_context,
                                &mut client.camera_uniform,
                                &mut client.camera_buffer,
                                dt,
                            );

                            // Attempt to render
                            if let Err(e) = client.renderer.render(
                                &client.wgpu_context,
                                &main_render_ctx,
                                &scene,
                            ) {
                                match e {
                                    wgpu::SurfaceError::Lost => {
                                        client.wgpu_context.resize(client.wgpu_context.size);
                                    }
                                    wgpu::SurfaceError::OutOfMemory => control_flow.exit(),
                                    _ => eprintln!("{:?}", e),
                                }
                            }
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
