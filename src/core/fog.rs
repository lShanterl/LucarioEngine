use wgpu::util::DeviceExt;
use crate::core::chunk::{BLOCK_WIDTH, CHUNK_SIZE};
use crate::core::graphics_resource_manager::{
    BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FogUniform {
    pub color: [f32; 4],
    pub start: f32,
    pub end:   f32,
    pub _pad:  [f32; 2],
}

impl FogUniform {
    pub fn new() -> Self {
        Self {
            color: [0.53, 0.81, 0.98, 1.0],
            start: BLOCK_WIDTH * CHUNK_SIZE as f32 * 12.0,
            end:   BLOCK_WIDTH * CHUNK_SIZE as f32 * 16.0,
            _pad:  [0.0; 2],
        }
    }
}
pub struct FogGpu {
    pub bind_group:        BindGroupHandle,
    pub bind_group_layout: BindGroupLayoutHandle,
}

impl FogGpu {
    pub fn new(device: &wgpu::Device, grm: &mut GraphicsResourceManager) -> Self {
        let uniform = FogUniform::new();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Fog Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = grm.create_bind_group_layout(
            device,
            &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::FRAGMENT,
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

        Self { bind_group, bind_group_layout }
    }
}