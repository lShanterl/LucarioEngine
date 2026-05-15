use std::sync::Arc;
use cgmath::{Rotation3, InnerSpace, One, Vector3};
use noise::{NoiseFn, Perlin};
use wgpu::util::DeviceExt;
use crate::renderer::renderer::{Instance, InstanceRaw, BASE_LEVEL};
use rayon::prelude::*;

pub(crate) const CHUNK_SIZE:  usize = 32;
pub const MAX_HEIGHT:  usize = 128;
pub const WATER_LEVEL: i32   = 36;
pub(crate) const BLOCK_WIDTH: f32   = 8.0;

pub struct TerrainNoise {
    continent:   Perlin,
    erosion:     Perlin,
    peaks_vals:  Perlin,
    temperature: Perlin,
    humidity:    Perlin,
    hills:       Perlin,
    detail:      Perlin,
    warp_x:      Perlin,
    warp_z:      Perlin,
}

impl TerrainNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            continent:   Perlin::new(seed),
            erosion:     Perlin::new(seed.wrapping_add(1)),
            peaks_vals:  Perlin::new(seed.wrapping_add(2)),
            temperature: Perlin::new(seed.wrapping_add(3)),
            humidity:    Perlin::new(seed.wrapping_add(4)),
            hills:       Perlin::new(seed.wrapping_add(5)),
            detail:      Perlin::new(seed.wrapping_add(6)),
            warp_x:      Perlin::new(seed.wrapping_add(7)),
            warp_z:      Perlin::new(seed.wrapping_add(8)),
        }
    }

    pub fn sample_clump(&self, wx: f64, wz: f64) -> f64 {
        Self::octaves(&self.erosion, wx * 0.05, wz * 0.05, 2, 0.5, 2.0)
    }

    fn octaves(noise: &Perlin, x: f64, z: f64, octs: u32, persistence: f64, lacunarity: f64) -> f64 {
        let (mut val, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for _ in 0..octs {
            val  += noise.get([x * freq, z * freq]) * amp;
            norm += amp;
            amp  *= persistence;
            freq *= lacunarity;
        }
        val / norm
    }

    pub fn sample_height(&self, wx: f64, wz: f64) -> i32 {
        let warp_strength = 40.0;
        let qx = Self::octaves(&self.warp_x, wx * 0.005, wz * 0.005, 2, 0.5, 2.0) * warp_strength;
        let qz = Self::octaves(&self.warp_z, (wx + 5.2) * 0.005, (wz + 1.3) * 0.005, 2, 0.5, 2.0) * warp_strength;

        let nx = wx + qx;
        let nz = wz + qz;

        let cont = Self::octaves(&self.continent, nx * 0.002, nz * 0.002, 4, 0.5, 2.0);
        let base_height = 40.0 + (cont * 30.0);

        let erosion = Self::octaves(&self.erosion, nx * 0.004, nz * 0.004, 3, 0.5, 2.0);
        let mountain_mask = smoothstep(0.1, 0.44, cont);
        let ridged = 1.0 - Self::octaves(&self.peaks_vals, nx * 0.008, nz * 0.008, 5, 0.5, 2.0).abs();
        let detail = Self::octaves(&self.detail, nx * 0.02, nz * 0.02, 4, 0.5, 2.0) * 4.0;

        let final_height = base_height + (ridged * 50.0 * mountain_mask * (1.0 - erosion.abs())) + detail;

        (final_height as i32).clamp(1, MAX_HEIGHT as i32 - 1)
    }

    pub fn sample_biome(&self, wx: f64, wz: f64) -> Biome {
        let jitter_x = Self::octaves(&self.detail, wx * 0.1, wz * 0.1, 1, 0.5, 2.0) * 4.0;
        let jitter_z = Self::octaves(&self.detail, wz * 0.1, wx * 0.1, 1, 0.5, 2.0) * 4.0;

        let nx = wx + jitter_x;
        let nz = wz + jitter_z;

        // continent noise to detect high-elevation regions
        let cont = Self::octaves(&self.continent, nx * 0.002, nz * 0.002, 4, 0.5, 2.0);

        // if the continent noise is very high, it's a mountain biome regardless of temp
        if cont > 0.8 {
            return Biome::Mountain;
        }

        let temp = Self::octaves(&self.temperature, nx * 0.001, nz * 0.001, 3, 0.5, 2.0);
        let humid = Self::octaves(&self.humidity, nx * 0.001, nz * 0.001, 3, 0.5, 2.0);

        let t = (temp * 0.5 + 0.5).clamp(0.0, 1.0);
        let h = (humid * 0.5 + 0.5).clamp(0.0, 1.0);

        // biome Table Logic
        if t < 0.3 {
            if h < 0.4 { Biome::IceSpikes } else { Biome::Tundra }
        } else if t < 0.6 {
            if h < 0.2 { Biome::Plains }
            else if h < 0.5 { Biome::Meadow }
            else if h < 0.7 { Biome::Forest }
            else { Biome::Taiga }
        } else {
            if h < 0.2 { Biome::Desert }
            else if h < 0.5 { Biome::Savanna }
            else { Biome::Jungle }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    Plains,
    Meadow,
    Mountain,
    Forest,
    Jungle,
    Desert,
    Savanna,
    Taiga,
    Tundra,
    IceSpikes
}

impl Biome {
    pub fn surface_block(self, y: i32, slope: f32) -> BlockType {
        match self {
            Biome::Mountain => {
                if y > 110 { return BlockType::Snow; }
                // mountains are stone if they are steep OR moderately high
                if slope > 1.3 || y > 85 { BlockType::Stone } // need to find a better solution for that though
                else { BlockType::Grass }
            },
            Biome::Meadow => {
                // meadows stay grassy even when steep, perfect for rolling hills
                if y > 115 { BlockType::Snow }
                else { BlockType::Grass }
            },
            Biome::Desert => BlockType::Sand,
            Biome::Tundra | Biome::IceSpikes => BlockType::Snow,
            _ => {
                // default behavior for plains, forest, etc.
                if slope > 1.8 && y > WATER_LEVEL { BlockType::Stone }
                else if y <= WATER_LEVEL + 1 { BlockType::Sand }
                else { BlockType::Grass }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType { Air, Grass, Dirt, Stone, Sand, Snow, Water }

impl BlockType {
    pub fn is_transparent(self) -> bool { matches!(self, BlockType::Air | BlockType::Water) }
    pub fn texture_index(self) -> u32 {
        match self {
            BlockType::Stone => 0, BlockType::Dirt => 1, BlockType::Grass => 2,
            BlockType::Sand => 3, BlockType::Water => 4, BlockType::Snow => 5,
            BlockType::Air => 0,
        }
    }
    pub fn uv_range(self) -> [f32; 4] {
        let index = self.texture_index() as f32;
        let tile_size = 16.0;
        let gutter = 16.0;
        let num_tiles = 6.0;
        let total_width = num_tiles * (tile_size + gutter);

        let x_px = index * (tile_size + gutter);

        // use a 0.5 pixel inset to stay safely away from the neighbor's edge
        let eps = 0.5;
        let u_start = (x_px + eps) / total_width;
        let u_end = (x_px + tile_size - eps) / total_width;

        // a small inset on V should prevent vertical bleeding
        let v_start = eps / tile_size;
        let v_end = 1.0 - (eps / tile_size);

        [u_start, v_start, u_end, v_end]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkCoordinates {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}


impl ChunkCoordinates {
    pub fn new(x: i32, y: i32, z: i32) -> Self { Self { x, y, z } }
    pub fn to_world_coordinates(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3 {
            x: self.x as f32 * BLOCK_WIDTH,

            y: self.y as f32 * BLOCK_WIDTH,

            z: self.z as f32 * BLOCK_WIDTH,

        }
    }
    pub fn distance_sq(&self, other: &ChunkCoordinates) -> i64 {

        let dx = (self.x - other.x) as i64;

        let dz = (self.z - other.z) as i64;

        dx * dx + dz * dz

    }
}
#[derive(Debug)]
pub struct Chunk {
    pub position: ChunkCoordinates,
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,

    pub height_map: [[i32; CHUNK_SIZE]; CHUNK_SIZE],
}
impl Chunk {
    pub fn new(device: &wgpu::Device, position: ChunkCoordinates, noise: &Arc<TerrainNoise>) -> Self {
        // Start with empty shell
        let mut chunk = Self {
            position,
            instances: Vec::new(),
            instance_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Placeholder"),
                size: 64,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }),
            height_map: [[0; CHUNK_SIZE]; CHUNK_SIZE],
        };

        chunk.generate(device, noise);
        chunk
    }

    pub fn generate(&mut self, device: &wgpu::Device, noise: &Arc<TerrainNoise>) {
        let mut instances = Vec::new();
        let mut block_grid = vec![[[BlockType::Air; CHUNK_SIZE]; CHUNK_SIZE]; MAX_HEIGHT];

        let start_x = self.position.x;
        let start_z = self.position.z;

        let mut local_heights = [[0i32; CHUNK_SIZE + 1]; CHUNK_SIZE + 1];
        for z in 0..=CHUNK_SIZE {
            for x in 0..=CHUNK_SIZE {
                let wx = (start_x + x as i32) as f64;
                let wz = (start_z + z as i32) as f64;
                let h = noise.sample_height(wx, wz);
                local_heights[x][z] = h;

                if x < CHUNK_SIZE && z < CHUNK_SIZE {
                    self.height_map[x][z] = h;
                }
            }
        }

        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let wx = (start_x + x as i32) as f64;
                let wz = (start_z + z as i32) as f64;

                let h = local_heights[x][z];
                let biome = noise.sample_biome(wx, wz);

                let dx = (local_heights[x + 1][z] - h) as f32;
                let dz = (local_heights[x][z + 1] - h) as f32;
                let slope = (dx * dx + dz * dz).sqrt();

                for y in 0..MAX_HEIGHT {
                    let yi = y as i32;
                    block_grid[y][x][z] = if yi > h {
                        if yi <= WATER_LEVEL { BlockType::Water } else { BlockType::Air }
                    } else if yi == h {
                        biome.surface_block(yi, slope)
                    } else if yi > h - 4 {
                        if biome == Biome::Desert { BlockType::Sand } else { BlockType::Dirt }
                    } else {
                        BlockType::Stone
                    };
                }
            }
        }

        for y in 0..MAX_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let block = block_grid[y][x][z];
                    if block == BlockType::Air { continue; }

                    let exposed = (x == 0 || block_grid[y][x-1][z].is_transparent()) ||
                        (x == CHUNK_SIZE-1 || block_grid[y][x+1][z].is_transparent()) ||
                        (y == 0 || block_grid[y-1][x][z].is_transparent()) ||
                        (y == MAX_HEIGHT-1 || block_grid[y+1][x][z].is_transparent()) ||
                        (z == 0 || block_grid[y][x][z-1].is_transparent()) ||
                        (z == CHUNK_SIZE-1 || block_grid[y][x][z+1].is_transparent());

                    if exposed {
                        let pos = cgmath::Vector3 {
                            x: (start_x + x as i32) as f32 * BLOCK_WIDTH,
                            y: (BASE_LEVEL + y as f32) * BLOCK_WIDTH,
                            z: (start_z + z as i32) as f32 * BLOCK_WIDTH,
                        };

                        instances.push(Instance::new(
                            pos,
                            cgmath::Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
                            block.uv_range()
                        ));
                    }
                }
            }
        }

        self.instances = instances;
        let raw_data: Vec<InstanceRaw> = self.instances.par_iter().map(|i| i.to_raw()).collect();

        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Instance Buffer"),
            contents: bytemuck::cast_slice(&raw_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }
}
fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }


