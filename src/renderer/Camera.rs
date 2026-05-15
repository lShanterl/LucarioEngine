use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::keyboard::KeyCode;
use crate::core::chunk::ChunkCoordinates;
use crate::core::graphics_resource_manager::{
    BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager,
};
use crate::core::input::Input;

const GRAVITY: f32 = -256.0;

const JUMP_VY: f32 = 128.0;

pub const PLAYER_HEIGHT: f32 = 16.0;

const TERMINAL_VELOCITY: f32 = -400.0;

pub struct Camera {
    pub eye:    cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up:     cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy:   f32,
    pub znear:  f32,
    pub zfar:   f32,

    /// eye position in render-units feet = position.y - PLAYER_HEIGHT
    pub position: Point3<f32>,
    yaw:          f32,
    pitch:        f32,

    #[allow(dead_code)] amount_left:      f32,
    #[allow(dead_code)] amount_right:     f32,
    #[allow(dead_code)] amount_forward:   f32,
    #[allow(dead_code)] amount_backward:  f32,
    #[allow(dead_code)] amount_up:        f32,
    #[allow(dead_code)] amount_down:      f32,
    #[allow(dead_code)] rotate_horizontal: f32,
    #[allow(dead_code)] rotate_vertical:   f32,
    #[allow(dead_code)] scroll:           f32,

    speed:       f32,
    sensitivity: f32,

    pub projection: Projection,

    velocity_y:   f32,
    is_on_ground: bool,
}
pub struct Projection {
    aspect: f32,
    fovy:   Rad<f32>,
    znear:  f32,
    zfar:   f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self { aspect: width as f32 / height as f32, fovy: fovy.into(), znear, zfar }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Plane { a: f32, b: f32, c: f32, d: f32 }

impl Plane {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self { Self { a, b, c, d } }
    fn normalize(self) -> Self {
        let l = (self.a*self.a + self.b*self.b + self.c*self.c).sqrt();
        Self { a: self.a/l, b: self.b/l, c: self.c/l, d: self.d/l }
    }
}

pub fn frustum_contains(frustum: &[Plane; 6], position: &ChunkCoordinates) -> bool {
    const XZ: f32 = 64.0;    // CHUNK_SIZE * BLOCK_WIDTH
    const Y:  f32 = 1024.0;  // MAX_HEIGHT  * BLOCK_WIDTH

    let p = position.to_world_coordinates();
    let corners = [
        p,                                          p + Vector3::new(XZ, 0.0, 0.0),
        p + Vector3::new(0.0, 0.0,  XZ),            p + Vector3::new(XZ, 0.0,  XZ),
        p + Vector3::new(0.0,  Y,  0.0),            p + Vector3::new(XZ,  Y,  0.0),
        p + Vector3::new(0.0,  Y,   XZ),            p + Vector3::new(XZ,  Y,   XZ),
    ];

    'corner: for c in &corners {
        for plane in frustum {
            if plane.a * c.x + plane.b * c.y + plane.c * c.z + plane.d < 0.0 {
                continue 'corner;
            }
        }
        return true;
    }
    false
}

pub fn extract_frustum_planes(view_proj: Matrix4<f32>) -> [Plane; 6] {
    let m = view_proj;
    [
        Plane::new(m.x.w+m.x.x, m.y.w+m.y.x, m.z.w+m.z.x, m.w.w+m.w.x), // left
        Plane::new(m.x.w-m.x.x, m.y.w-m.y.x, m.z.w-m.z.x, m.w.w-m.w.x), // right
        Plane::new(m.x.w+m.x.y, m.y.w+m.y.y, m.z.w+m.z.y, m.w.w+m.w.y), // bottom
        Plane::new(m.x.w-m.x.y, m.y.w-m.y.y, m.z.w-m.z.y, m.w.w-m.w.y), // top
        Plane::new(m.x.w+m.x.z, m.y.w+m.y.z, m.z.w+m.z.z, m.w.w+m.w.z), // near
        Plane::new(m.x.w-m.x.z, m.y.w-m.y.z, m.z.w-m.z.z, m.w.w-m.w.z), // far
    ].map(|p| p.normalize())
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj:     [[f32; 4]; 4],
    view_position: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self { view_proj: Matrix4::identity().into(), view_position: [0.0; 4] }
    }
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (camera.projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>>(
        width: f32, height: f32,
        speed: f32, sensitivity: f32,
        pos: V, yaw: f32, pitch: f32,
    ) -> Self {
        let aspect = width / height;
        let fovy   = 45.0_f32;
        let znear  = 1.0;
        let zfar   = 8000.0;
        let pos    = pos.into();
        Self {
            eye:    Point3::new(0.0, 1.0, 2.0),
            target: Point3::new(0.0, 0.0, 0.0),
            up:     Vector3::unit_y(),
            aspect, fovy, znear, zfar,
            position: pos,
            yaw, pitch,
            amount_left: 0.0, amount_right: 0.0,
            amount_forward: 0.0, amount_backward: 0.0,
            amount_up: 0.0, amount_down: 0.0,
            rotate_horizontal: 0.0, rotate_vertical: 0.0,
            scroll: 0.0, speed, sensitivity,
            projection: Projection {
                aspect,
                fovy:  Rad::from(Deg(fovy)),
                znear, zfar,
            },
            velocity_y:   0.0,
            is_on_ground: false,
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cp * cy, sp, cp * sy).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.projection.resize(size.width, size.height);
    }

    #[inline]
    fn input_axis(input: &Input, pos: KeyCode, neg: KeyCode) -> f32 {
        (input.is_key_down(pos) as i32 - input.is_key_down(neg) as i32) as f32
    }

    pub fn update_camera(
        &mut self,
        input:           &Input,
        dt:              Duration,
        ground_y:        f32,
        is_mouse_focused: bool,
    ) {
        let dt = dt.as_secs_f32();


        let fwd   = Self::input_axis(input, KeyCode::KeyW, KeyCode::KeyS);
        let right = Self::input_axis(input, KeyCode::KeyD, KeyCode::KeyA);

        let (sy, cy) = self.yaw.sin_cos();
        let dir_fwd   = Vector3::new(cy, 0.0, sy);
        let dir_right = Vector3::new(-sy, 0.0, cy);
        let horiz_speed = self.speed * dt;

        self.position += dir_fwd   * fwd   * horiz_speed;
        self.position += dir_right * right * horiz_speed;


        if self.is_on_ground && input.is_key_just_pressed(KeyCode::Space) {
            self.velocity_y = JUMP_VY;
            self.is_on_ground = false;
        }

        // gravity
        self.velocity_y = (self.velocity_y + GRAVITY * dt).max(TERMINAL_VELOCITY);

        //  move vertically
        self.position.y += self.velocity_y * dt;

        let feet_y = self.position.y - PLAYER_HEIGHT;
        if feet_y <= ground_y {
            // push feet up to the surface, kill downward velocity
            self.position.y = ground_y + PLAYER_HEIGHT;
            if self.velocity_y < 0.0 { self.velocity_y = 0.0; }
            self.is_on_ground = true;
        } else {
            self.is_on_ground = false;
        }

        if is_mouse_focused {
            let delta = input.mouse_delta_f32();
            self.yaw   -= self.sensitivity * -delta.x;
            self.pitch  -= self.sensitivity *  delta.y;
            self.pitch   = self.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
        }
    }

}


pub struct CameraGpu {
    pub uniform:            CameraUniform,
    pub buffer:             wgpu::Buffer,
    pub bind_group:         BindGroupHandle,
    pub bind_group_layout:  BindGroupLayoutHandle,
}

impl CameraGpu {
    pub fn new(
        device: &wgpu::Device,
        grm:    &mut GraphicsResourceManager,
        camera: &Camera,
    ) -> Self {
        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = grm.create_bind_group_layout(
            device,
            &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty:                 wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        );

        let bind_group = grm.create_bind_group(
            bind_group_layout,
            device,
            &[wgpu::BindGroupEntry {
                binding:  0,
                resource: buffer.as_entire_binding(),
            }],
        );

        Self { uniform, buffer, bind_group, bind_group_layout }
    }

    pub fn sync(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        self.uniform.update_view_proj(camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}