
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub(crate) position: [f32; 3],
    // due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding: f32,
    pub(crate) color: [f32; 3],
    pub(crate) _padding2: f32,
}

use cgmath::{InnerSpace, Rotation3, Vector3};
use wgpu::util::DeviceExt;

use crate::core::graphics_resource_manager::{
    BindGroupHandle, BindGroupLayoutHandle, GraphicsResourceManager,
};

pub struct LightGpu {
    pub uniform:           LightUniform,
    pub buffer:            wgpu::Buffer,
    pub bind_group:        BindGroupHandle,
    pub bind_group_layout: BindGroupLayoutHandle,
}

impl LightGpu {
    pub fn new(device: &wgpu::Device, grm: &mut GraphicsResourceManager) -> Self {
        let uniform = LightUniform {
            position:  [2.0, 2.0, 2.0],
            _padding:  0.0,
            color:     [1.0, 1.0, 1.0],
            _padding2: 0.0,
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Light Buffer"),
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

    pub fn tick(&mut self, queue: &wgpu::Queue) {
        let old: Vector3<f32> = self.uniform.position.into();
        let new_pos = cgmath::Quaternion::from_axis_angle(
            Vector3::unit_y(),
            cgmath::Deg(1.0),
        ) * old;
        self.uniform.position = new_pos.into();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}