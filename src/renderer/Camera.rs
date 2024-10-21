use cgmath::*;
use winit::event::*;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;
use winit::keyboard::KeyCode;
use crate::core::input::Input;
use crate::core::wgpu_context::WgpuContext;

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,


    pub position: Point3<f32>,
    yaw: f32,
    pitch: f32,


    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,

    amount_up: f32,
    amount_down: f32,

    rotate_horizontal: f32,
    rotate_vertical: f32,

    scroll: f32,
    speed: f32,
    sensitivity: f32,

    projection: Projection,
}

impl Camera {
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;


/*

|right_x, up_x, forward_x, position_x|
|right_y, up_y, forward_y, position_y|
|right_z, up_z, forward_z, position_z|
|  0.0  ,  0.0,    0.0   ,    0.0    |

 */

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
    view_position: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            view_position: [0.0; 4],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (camera.projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>>,
    >
    (width: f32, height: f32, speed: f32, sensitivity: f32, pos: V, yaw: f32, pitch: f32) -> Self {
        let eye = cgmath::Point3::new(0.0, 1.0, 2.0);
        let target = cgmath::Point3::new(0.0, 0.0, 0.0);
        let up = cgmath::Vector3::unit_y();
        let aspect = width / height;
        let fovy = 45.0;
        let znear = 0.1;
        let zfar = 100.0;


        Self{
            eye,
            target, // it is kinda a forward vector
            up,
            aspect,
            fovy,
            znear,
            zfar,

            position: pos.into(),
            yaw,
            pitch,

            
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            
            amount_up: 0.0,
            amount_down: 0.0,
            
            rotate_vertical: 0.0,
            rotate_horizontal: 0.0,
            
            scroll: 0.0,
            speed,
            sensitivity,
            
            projection: Projection{
                aspect,
                fovy: Rad::from(Deg(fovy)),
                znear,
                zfar,
            },

        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.projection.resize(size.width, size.height);
    }
    
    pub(crate) fn update_camera(&mut self, input: &Input, dt: Duration, is_mouse_focused: bool) {
        let dt = dt.as_secs_f32();


        let input_forward = Self::get_input_dir(&input, KeyCode::KeyW, KeyCode::KeyS);
        let input_right = Self::get_input_dir(&input, KeyCode::KeyD, KeyCode::KeyA);
        let input_up = Self::get_input_dir(&input, KeyCode::Space, KeyCode::ControlLeft);

        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();


        let dir_forward = Vector3::new(cos_yaw, 0.0, sin_yaw);
        let dir_right = Vector3::new(-sin_yaw, 0.0, cos_yaw);
        const DIR_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

        let speed = self.speed * dt;

        self.position += dir_forward * input_forward * speed;
        self.position += dir_right * input_right * speed;
        self.position += DIR_UP * input_up * speed * 2.0;

        let temporary_lr = Self::get_input_dir(input, KeyCode::ArrowRight, KeyCode::ArrowLeft);
        let temporary_td = Self::get_input_dir(input, KeyCode::ArrowDown, KeyCode::ArrowUp);

        // rotation
        if is_mouse_focused{

            //let rotate_amount = input.mouse_delta_f32();

            let rotate_amount = Vector2::new(temporary_lr * 20.0, temporary_td * 20.0);

            self.yaw -= self.sensitivity * -rotate_amount.x;
            self.pitch -= self.sensitivity * rotate_amount.y;
    
            self.pitch = self
                .pitch
                .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
        }
    }

    fn get_input_dir(input: &Input, key_positive: KeyCode, key_negative: KeyCode) -> f32 {
        (input.is_key_down(key_positive) as i32 - input.is_key_down(key_negative) as i32) as f32
    }
}