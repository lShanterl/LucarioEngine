use std::collections::HashSet;
use std::sync::Arc;
use cgmath::Rotation3;
use noise::core::perlin::perlin_2d;
use noise::{Fbm, NoiseFn, Perlin};
use tokio::runtime::Runtime;
use tokio::task;
use wgpu::util::DeviceExt;
use crate::renderer::renderer::{Instance, State, BASE_LEVEL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkCoordinates {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
impl ChunkCoordinates {
    pub(crate) fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkCoordinates { x, y, z }
    }
    
    pub(crate) fn to_world_coordinates(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3 {
            x: self.x as f32 * 8.0,
            y: self.y as f32 * 8.0,
            z: self.z as f32 * 8.0,
        }
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub(crate) position: ChunkCoordinates,
    pub(crate) instances: Vec<Instance>,
    pub(crate) instance_buffer: wgpu::Buffer,
}

impl Chunk {
    pub fn new(device: &wgpu::Device, position: ChunkCoordinates, perlin_noise: &Arc<Fbm<Perlin>>) -> Chunk {
        let instances: Vec<Instance> = Vec::new();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        

        let mut chunk = Chunk {
            position,
            instances,
            instance_buffer,
        };

        chunk.generate_chunk(device, position.x, position.z, 8, perlin_noise);

        chunk
    }

    pub fn generate_chunk(&mut self, device: &wgpu::Device, start_x: i32, start_z: i32, chunk_size: u32, perlin_noise: &Arc<Fbm<Perlin>>) {
        let half_chunk_size = chunk_size / 2;

        let instances = (0..chunk_size).flat_map(|z| {
            (0..chunk_size).flat_map(move |x| {

                let noise_value = perlin_noise.get([((start_x + x as i32) as f64) * 0.9, ((start_z + z as i32) as f64) * 0.9]);
                // the noise value is between -1 and 1, we want it between 0 and 255
                let noise_value = (noise_value + 1.0) * 127.5;
                let height_value = noise_value as i32;
                let height_value = State::map_to_closest_multiple_of(height_value as u32, 8) as i32;

                (0..=height_value).map(move |y| {
                    let position = cgmath::Vector3 {
                        x: (start_x + x as i32) as f32 * 8.0,
                        y: BASE_LEVEL +  State::map_to_closest_multiple_of(y as u32, 8) as f32,
                        z: (start_z + z as i32) as f32 * 8.0,
                    };

                    let rotation = cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_x(),
                        cgmath::Deg(180.0),
                    );
                    
                    let texture_index = match y {
                        0..=64 => 0,
                        65..=80 => 1,
                        81..=128 => 2,
                        129..=192 => 3,
                        193..=240 => 4,
                        241..=255 => 5,
                        _ => 0,
                    };
                    Instance::new(position, rotation, texture_index)
                })
            })
        }).collect::<Vec<_>>();

        self.instances = instances;

        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        self.instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
    }
}