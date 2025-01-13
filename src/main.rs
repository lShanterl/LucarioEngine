mod renderer;
mod object;
mod texture;
mod core;
mod utils;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use cgmath::{Rotation3, Vector3};
use image::{GrayImage, Luma};
use log::debug;
use renderer::{camera::{self, Camera, CameraUniform}, renderer::{RenderContext}};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::CursorGrabMode;
use crate::core::chunk_manager::ChunkGenerator;
use crate::core::debug_manager::DebugManager;
use crate::core::graphics_resource_manager::{BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager};
use crate::core::input::Input;
use crate::core::scene_manager::SceneManager;
use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;
use crate::renderer::renderer::{Renderer, State};
use crate::texture::Texture;
use crate::core::light::LightUniform;
use crate::renderer::camera::extract_frustum_planes;

pub struct Client {
    wgpu_context: WgpuContext,
    graphics_resource_manager: GraphicsResourceManager,
    renderer: Renderer,
    camera: Camera,
    debug_manager: DebugManager,
    scene_manager: SceneManager,

    //temporary I guess

    camera_buffer: wgpu::Buffer,
    camera_uniform: CameraUniform,
    camera_bind_group_handle: BindGroupHandle,
    camera_bind_group_layout_handle: BindGroupLayoutHandle,

    light_bind_group_layout_handle: BindGroupLayoutHandle,
    light_bind_group_handle: BindGroupHandle,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,

    pool: uvth::ThreadPool,
    chunk_generator: ChunkGenerator,

    input: Input,
    is_mouse_focused: bool,

    depth_texture: Texture
}

//split into modules
pub struct CameraState {
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group_handle: BindGroupHandle,
}

pub struct LightState {
    pub light_uniform: LightUniform,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group_handle: BindGroupHandle,
}

impl Client {
    pub async fn new(window: Arc<Window>) -> Self {
        let mut debug_manager = DebugManager::new().await;

        debug_manager.start_timer("client_init");

        let wgpu_context = WgpuContext::new(window.clone());
        let mut graphics_resource_manager = GraphicsResourceManager::new();

        let renderer = Renderer::new();
        let camera = Camera::new(
            wgpu_context.get_surface_config().width as f32,
            wgpu_context.get_surface_config().height as f32,
            150.1,
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

        println!("Client init: {:?}",debug_manager.stop_timer("client_init").unwrap().duration());

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0f32,
            color: [1.0, 1.0, 1.0],
            _padding2: 0f32,
        };

        let light_buffer = wgpu_context.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        
        let light_bind_group_layout_handle = graphics_resource_manager.create_bind_group_layout(
            &wgpu_context.device,
            &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ]
        );
        let light_bind_group_handle = graphics_resource_manager.create_bind_group(
            light_bind_group_layout_handle,
            &wgpu_context.device,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                }
            ]
        );

        let pool = uvth::ThreadPoolBuilder::new().name("Chunk thread pool".parse().unwrap()).build();
        let chunk_generator = ChunkGenerator::new();


        Self {
            wgpu_context,
            graphics_resource_manager,
            renderer,
            camera,
            debug_manager,
            scene_manager: SceneManager::new(),

            camera_buffer,
            camera_uniform,
            camera_bind_group_handle,
            camera_bind_group_layout_handle,

            light_bind_group_layout_handle,
            light_bind_group_handle,
            light_uniform,
            light_buffer,

            pool,
            chunk_generator,

            input: Input::new(),
            is_mouse_focused: false,
            depth_texture,


        }
    }
}

pub async fn run() {

    // let width = 255;
    // let height = 255;
    // let scale = 0.1;
    // let seed = 42;
    //
    // let mut img: GrayImage = GrayImage::new(width, height);
    //
    // for y in 0..height {
    //     for x in 0..width {
    //         let noise_value = perlin_noise::perlin(x as f32 * scale, y as f32 * scale);
    //         let color_value = (noise_value * 255.0).clamp(0.0, 255.0) as u8; // Convert to grayscale
    //         img.put_pixel(x, y, Luma([color_value]));
    //     }
    // }

    
    //let output_path = Path::new("perlin_noise.png");
    //img.save(output_path).expect("Failed to save image");
    //println!("Image saved to {:?}", output_path);

    std::env::set_var("RUST_BACKTRACE", "1");

    let event_loop = EventLoop::new().unwrap_or_else(|e| panic!("Failed to initialize event loop: {}", e));
    let window = Arc::new(WindowBuilder::new().with_inner_size(PhysicalSize::new(800, 600)).build(&event_loop).unwrap());
    window.set_title("LucarioProject - Voxel engine");

    let mut client = Client::new(window.clone()).await;

    //let atlas = Texture::create_texture_atlas(&client.wgpu_context.device, &client.wgpu_context.queue, &[include_bytes!("./assets/textures/uv_map.jpg"),include_bytes!("./assets/textures/grid_01.png"), include_bytes!("./assets/textures/grid_02.png"), include_bytes!("./assets/textures/obama.png")], "egg_label").unwrap();

    //println!("{:?}", atlas.1);

    //client.debug_manager.start_timer("texture_atlas");
    //let textures = Texture::create_texture_atlas(&client.wgpu_context.device, &client.wgpu_context.queue, &[include_bytes!("./assets/textures/uv_map.jpg"),include_bytes!("./assets/textures/grid_01.png"), include_bytes!("./assets/textures/grid_02.png"), include_bytes!("./assets/textures/obama.png")], "egg_label");
    //println!("Texture atlas: {:?}", client.debug_manager.stop_timer("texture_atlas").unwrap().duration());

    client.debug_manager.start_timer("binds");
    let shader = client.wgpu_context.device.create_shader_module(wgpu::include_wgsl!("./shaders/test_shader.wgsl"));
    let light_shader = client.wgpu_context.device.create_shader_module(wgpu::include_wgsl!("./shaders/light.wgsl"));
    
    let diff_texture = texture::Texture::from_bytes(&client.wgpu_context.device, &client.wgpu_context.queue, include_bytes!("../atlas.png"), "temporary.png").unwrap();
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
            &client.camera_bind_group_layout_handle,
        ]
    );

    let render_pipeline_handle = client.graphics_resource_manager.create_pipeline(
        &client.wgpu_context.device,
        render_pipeline_layout_handle,
        &shader,
        &client.wgpu_context.surface_config,
        Some(&client.depth_texture),
        true
    );

    println!("Binds: {:?}", client.debug_manager.stop_timer("binds").unwrap().duration());

    // let meshes = [
    //     //&Mesh::new(&client.wgpu_context.device, object::VERTICES, object::INDICES),
    //     &Mesh::new(&client.wgpu_context.device, object::CUBE_VERTICES, object::CUBE_INDICES),
    //     &Mesh::new(&client.wgpu_context.device, object::CONE_VERTICES, object::CONE_INDICES),
    // ];
    
    let world = [
        &Mesh::new_cube_at(&client.wgpu_context.device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        &Mesh::new_cube_at(&client.wgpu_context.device, [1.0, 1.0, 1.0], [1.0, 0.0, 0.0]),
    ];

    let cube = client.scene_manager.add_mesh(Mesh::new_cube_at(&client.wgpu_context.device, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
    client.scene_manager.add_mesh(Mesh::new_cube_at(&client.wgpu_context.device, [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]));

    client.debug_manager.start_timer("state");
    let state = State::new(&client.wgpu_context.device);
    println!("State: {:?}", client.debug_manager.stop_timer("state").unwrap().duration());


    // let scene = Scene{
    //     meshes: &world
    // }; //TODO: optimize it, create a scene manager, add mesh delete mesh etc

    let main_render_ctx = client.graphics_resource_manager.create_render_context(
        &render_pipeline_handle,
        &[
            &diffuse_bind_group_handle,
            &client.camera_bind_group_handle,
        ]
    );
    ;
    let mut new_size: Option<PhysicalSize<u32>> = None;
    let mut last_render_time = instant::Instant::now();
    let mut print_framerate = false;

    let mut iterator = 0;


    event_loop.run(move |event, control_flow| {
        let mut new_size: Option<PhysicalSize<u32>> = None;

        match event {

            Event::DeviceEvent {ref event, .. } => {
                client.input.handle_device_event(event);
            }
            

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
                            client.wgpu_context.resize(*size);

                            client.depth_texture = texture::Texture::create_depth_texture(&client.wgpu_context.device, &client.wgpu_context.surface_config, "depth_texture");
                            //println!("{:?}", &client.wgpu_context.surface_config);

                        }
                        WindowEvent::RedrawRequested => {
                            if let Some(size) = new_size.take() {
                            }

                            let old_position: cgmath::Vector3<_> = client.light_uniform.position.into();
                            client.light_uniform.position =
                                (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                                    * old_position)
                                    .into();
                            client.wgpu_context.queue.write_buffer(&client.light_buffer, 0, bytemuck::cast_slice(&[client.light_uniform]));

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
                            let player_position: cgmath::Vector3<f32> = cgmath::Vector3::new(client.camera.position.x, client.camera.position.y, client.camera.position.z);

                            let view_proj = client.camera.calc_matrix() * client.camera.projection.calc_matrix();

                            let frustum = extract_frustum_planes(view_proj);

                            client.chunk_generator.build_chunks(client.wgpu_context.device.clone(), &mut client.scene_manager, player_position, &client.pool, &frustum);
                            iterator+=1;
                            if(iterator > 1000){
                                iterator = 0;
                                

                                for (p, chunk) in client.scene_manager.get_chunk_array() {
                                    println!("{:?}`", p);
                                }
                            }
                            println!("{:?}", client.scene_manager.get_chunk_array().len());


                            // Attempt to render
                            if let Err(e) = //client.renderer.render(&client.wgpu_context, &main_render_ctx, &scene_manager, &client.depth_texture, )
                                //client.renderer.render_instanced(&client.wgpu_context, &main_render_ctx, &client.graphics_resource_manager, &state, &client.scene_manager.get_mesh(cube), &client.depth_texture)
                                client.renderer.render_chunks(&client.wgpu_context, &main_render_ctx, &client.graphics_resource_manager,  &client.scene_manager.get_mesh(cube), &client.depth_texture, &client.scene_manager)
                                {
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
                            for (pos, mesh) in client.scene_manager.get_chunk_mut_array() {
                                println!("{:?}", pos);
                            }
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
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    run().await;
}
