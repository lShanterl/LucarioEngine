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

        // Create a 3D grid to store block presence
        let mut block_grid = vec![vec![vec![false; chunk_size as usize]; chunk_size as usize]; 256];

        // First pass: populate the block grid
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let noise_value = perlin_noise.get([((start_x + x as i32) as f64) * 0.9, ((start_z + z as i32) as f64) * 0.9]);
                let noise_value = (noise_value + 1.0) * 127.5;
                let height_value = noise_value as i32;
                let height_value = State::map_to_closest_multiple_of(height_value as u32, 8) as i32;

                // Populate the block grid up to the height value
                for y in 0..=height_value {
                    block_grid[y as usize][x as usize][z as usize] = true;
                }
            }
        }

        // Second pass: generate instances only for visible blocks
        let mut instances = Vec::new();  // We will collect instances manually here

        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let noise_value = perlin_noise.get([((start_x + x as i32) as f64) * 0.9, ((start_z + z as i32) as f64) * 0.9]);
                let noise_value = (noise_value + 1.0) * 127.5;
                let height_value = noise_value as i32;
                let height_value = State::map_to_closest_multiple_of(height_value as u32, 8) as i32;

                // Ensure all y positions up to the height_value are being checked
                for y in 0..=height_value {
                    // Determine visibility logic
                    let is_surface = if y == height_value {
                        true  // Top of the column is always visible
                    } else {
                        // If the block is not at the top, check if it is exposed

                        // Block is exposed if it has no neighbor below (it's at the bottom) or if it has any empty neighboring side
                        let is_above_empty = y == 0 || !block_grid[(y - 1) as usize][x as usize][z as usize];  // Check if the block below is empty

                        // Check sides for exposure (left, right, front, back)
                        let is_side_exposed =
                            (x > 0 && !block_grid[y as usize][(x - 1) as usize][z as usize]) ||  // Left
                                (x < chunk_size - 1 && !block_grid[y as usize][(x + 1) as usize][z as usize]) ||  // Right
                                (z > 0 && !block_grid[y as usize][x as usize][(z - 1) as usize]) ||  // Front
                                (z < chunk_size - 1 && !block_grid[y as usize][x as usize][(z + 1) as usize]);  // Back

                        // Expose the block if it's not covered by a block below and if it's exposed on any side
                        is_above_empty || is_side_exposed
                    };

                    // If the block is considered visible (either on top or exposed on sides)
                    if is_surface {
                        let position = cgmath::Vector3 {
                            x: (start_x + x as i32) as f32 * 8.0,
                            y: BASE_LEVEL + State::map_to_closest_multiple_of(y as u32, 8) as f32,
                            z: (start_z + z as i32) as f32 * 8.0,
                        };

                        let rotation = cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_x(),
                            cgmath::Deg(180.0),
                        );

                        // Assign a texture based on the height
                        let texture_index = match y {
                            0..=64 => 0,
                            65..=80 => 1,
                            81..=128 => 2,
                            129..=192 => 3,
                            193..=240 => 4,
                            241..=255 => 5,
                            _ => 0,
                        };

                        // Push the instance to the collection
                        instances.push(Instance::new(position, rotation, texture_index));
                    }
                }
            }
        }

        // Assign the instances to the struct
        self.instances = instances;

        // Create the instance buffer
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