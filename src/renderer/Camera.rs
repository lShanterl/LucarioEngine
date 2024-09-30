use cgmath::*;
use winit::event::*;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use winit::keyboard::KeyCode;

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,

    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,


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
        use cgmath::SquareMatrix;
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
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>
    >
    (width: f32, height: f32, speed: f32, sensitivity: f32, pos: V, yaw: Y, pitch: P ) -> Self {
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
            yaw: yaw.into(),
            pitch: pitch.into(),
            
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
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            Vector3::unit_y(),
        )
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {

        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.build_view_projection_matrix();

    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub(crate) fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };

        match key {
            winit::keyboard::KeyCode::KeyW => {
                self.amount_forward = amount;
                true
            }
            winit::keyboard::KeyCode::KeyS => {
                self.amount_backward = amount;
                true
            }
            winit::keyboard::KeyCode::KeyA => {
                self.amount_left = amount;
                true
            }
            winit::keyboard::KeyCode::KeyD => {
                self.amount_right = amount;
                true
            }
            winit::keyboard::KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            winit::keyboard::KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }


    pub(crate) fn update_camera(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        self.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        self.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = self.pitch.0.sin_cos();
        let scrollward = Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        self.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        self.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        self.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non-cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if self.pitch < -Rad(SAFE_FRAC_PI_2) {
            self.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if self.pitch > Rad(SAFE_FRAC_PI_2) {
            self.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}