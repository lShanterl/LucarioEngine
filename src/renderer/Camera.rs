pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
/*

|right_x, up_x, forward_x, position_x|
|right_y, up_y, forward_y, position_y|
|right_z, up_z, forward_z, position_z|
|  0.0  ,  0.0,    0.0   ,    0.0    |

 */



impl Camera {
    pub fn new() -> Self {
        let eye = cgmath::Point3::new(0.0, 0.0, -5.0);
        let target = cgmath::Point3::new(0.0, 0.0, 0.0);
        let up = cgmath::Vector3::new(0.0, 1.0, 0.0);
        let aspect = 16.0 / 9.0;
        let fovy = 45.0;
        let znear = 0.1;
        let zfar = 100.0;
        

        Self{
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {

        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}