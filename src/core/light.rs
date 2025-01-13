
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub(crate) position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding: f32,
    pub(crate) color: [f32; 3],
    pub(crate) _padding2: f32,
}