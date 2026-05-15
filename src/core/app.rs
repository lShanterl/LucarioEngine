use std::sync::Arc;

use cgmath::Vector3;
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, MouseButton, WindowEvent},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::core::{
    chunk_manager::ChunkGenerator,
    graphics_resource_manager::GraphicsResourceManager,
    input::Input,
    scene_manager::SceneManager,
    wgpu_context::WgpuContext,
};
use crate::core::fog::FogGpu;
use crate::core::light::LightGpu;
use crate::renderer::{
    camera::{Camera, extract_frustum_planes},
    renderer::{RenderContext, Renderer},
};
use crate::renderer::camera::CameraGpu;
use crate::texture::Texture;

/// top-level application state. Owns all GPU resources, game state, and the
/// render/update cycle
pub struct App {
    // GPU infrastructure
    wgpu:     WgpuContext,
    grm:      GraphicsResourceManager,
    renderer: Renderer,

    // Per-frame GPU uniforms
    camera_gpu: CameraGpu,
    light_gpu:  LightGpu,
    fog_gpu:    FogGpu,

    _atlas: Texture,

    // Game / world state
    camera: Camera,
    scene:  SceneManager,
    chunks: ChunkGenerator,
    pool:   uvth::ThreadPool,

    // Render resources
    main_render_ctx: RenderContext,
    depth_texture:   Texture,

    // Runtime flags
    input:            Input,
    is_mouse_focused: bool,
    print_framerate:  bool,
}

impl App {
    pub async fn new(window: Arc<Window>) -> Self {
        let mut wgpu = WgpuContext::new(window.clone());
        let mut grm  = GraphicsResourceManager::new();

        // camera must come first so its layout is created before the pipeline
        let cfg    = wgpu.get_surface_config();
        let camera = Camera::new(
            cfg.width as f32, cfg.height as f32,
            /*speed*/ 150.1, /*sensitivity*/ 0.001,
            (0.0, 0.0, 0.0), /*yaw*/ 0.0, /*pitch*/ 0.0,
        );
        let camera_gpu = CameraGpu::new(&wgpu.device, &mut grm, &camera);

        // lighting & fog uniforms
        let light_gpu = LightGpu::new(&wgpu.device, &mut grm);
        let fog_gpu   = FogGpu::new(&wgpu.device, &mut grm);

        // depth buffer
        let depth_texture = Texture::create_depth_texture(
            &wgpu.device, &wgpu.surface_config, "depth_texture",
        );

        // texture atlas (stone=0, dirt=1, grass=2, sand=3, water=4, snow=5)
        let (atlas, _atlas_coords) = Texture::create_texture_atlas(
            &wgpu.device,
            &wgpu.queue,
            &[
                include_bytes!("../assets/textures/stone.png"),
                include_bytes!("../assets/textures/dirt.png"),
                include_bytes!("../assets/textures/grass.png"),
                include_bytes!("../assets/textures/sand.png"),
                include_bytes!("../assets/textures/water.png"),
                include_bytes!("../assets/textures/snow.png"),
            ],
            "atlas",
        ).expect("Failed to create texture atlas");

        println!("{:?}",atlas.texture.size());


        let shader = wgpu.device.create_shader_module(
            wgpu::include_wgsl!("../shaders/test_shader.wgsl"),
        );

        let texture_bgl = grm.create_bind_group_layout(
            &wgpu.device,
            &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled:   false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );

        let diffuse_bg = grm.create_bind_group(
            texture_bgl,
            &wgpu.device,
            &[
                wgpu::BindGroupEntry {
                    binding:  0,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding:  1,
                    resource: wgpu::BindingResource::Sampler(&atlas.sampler),
                },
            ],
        );

        let pipeline_layout = grm.create_pipeline_layout(
            &wgpu.device,
            &[
                &texture_bgl,
                &camera_gpu.bind_group_layout,
                &fog_gpu.bind_group_layout,
            ],
        );

        let pipeline = grm.create_pipeline(
            &wgpu.device,
            pipeline_layout,
            &shader,
            &wgpu.surface_config,
            Some(&depth_texture),
            true,
        );

        let main_render_ctx = grm.create_render_context(
            &pipeline,
            &[&diffuse_bg, &camera_gpu.bind_group, &fog_gpu.bind_group],
        );

        // initial scene geometry
        let mut scene = SceneManager::new();

        let pool = uvth::ThreadPoolBuilder::new()
            .name("chunk_pool".parse().unwrap())
            .build();

        Self {
            wgpu,
            grm,
            renderer: Renderer::new(),
            camera_gpu,
            light_gpu,
            fog_gpu,
            _atlas: atlas,
            camera,
            scene,
            chunks: ChunkGenerator::new(),
            pool,
            main_render_ctx,
            depth_texture,
            input: Input::new(),
            is_mouse_focused: false,
            print_framerate: false,
        }
    }

    pub fn update(&mut self, dt: instant::Duration) {
        // debug toggles
        if self.input.is_key_just_pressed(winit::keyboard::KeyCode::KeyJ) {
            println!("camera pos: {:?}", self.camera.position);
        }
        if self.input.is_key_just_pressed(winit::keyboard::KeyCode::KeyF) {
            self.print_framerate = !self.print_framerate;
        }
        if self.print_framerate {
            println!("{:.0} fps", 1.0 / dt.as_secs_f32());
        }

        // rotate directional light
        //self.light_gpu.tick(&self.wgpu.queue);

        // camera movement + physics
        let ground_y = self
            .scene
            .surface_render_height_at(self.camera.position.x, self.camera.position.z)
            .unwrap_or(f32::MIN);
        self.camera
            .update_camera(&self.input, dt, ground_y, self.is_mouse_focused);
        self.camera_gpu.sync(&self.wgpu.queue, &self.camera);

        // chunk streaming
        let player_pos = Vector3::new(
            self.camera.position.x,
            self.camera.position.y,
            self.camera.position.z,
        );
        let view_proj = self.camera.projection.calc_matrix() * self.camera.calc_matrix();
        let frustum   = extract_frustum_planes(view_proj);
        self.chunks.build_chunks(
            self.wgpu.device.clone(),
            &mut self.scene,
            player_pos,
            &self.pool,
            &frustum,
        );
    }

    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.render_chunks(
            &self.wgpu,
            &self.main_render_ctx,
            &self.grm,
            &self.depth_texture,
            &self.scene,
        )
    }

    pub fn end_frame(&mut self) {
        self.input.reset();
    }


    pub fn on_device_event(&mut self, event: &DeviceEvent) {
        self.input.handle_device_event(event);
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        if self.input.handle_window_event(event) {
            if self.input.is_mouse_button_just_pressed(MouseButton::Left) {
                self.toggle_mouse_focus();
            }
            return true;
        }
        false
    }


    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        self.wgpu.resize(new_size);
        self.camera.resize(new_size);
        self.camera_gpu.sync(&self.wgpu.queue, &self.camera);
        self.depth_texture = Texture::create_depth_texture(
            &self.wgpu.device,
            &self.wgpu.surface_config,
            "depth_texture",
        );
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.wgpu.size
    }

    pub fn window_id(&self) -> WindowId {
        self.wgpu.get_window().id()
    }

    pub fn request_redraw(&self) {
        self.wgpu.get_window().request_redraw();
    }


    fn toggle_mouse_focus(&mut self) {
        self.is_mouse_focused = !self.is_mouse_focused;
        let window = self.wgpu.get_window();
        if self.is_mouse_focused {
            window
                .set_cursor_grab(CursorGrabMode::Confined)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                .unwrap();
            window.set_cursor_visible(false);
        } else {
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
            window.set_cursor_visible(true);
        }
    }
}