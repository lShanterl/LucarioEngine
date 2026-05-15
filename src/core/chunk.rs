use std::sync::Arc;
use cgmath::{Vector3};
use noise::{NoiseFn, Perlin};
use wgpu::util::DeviceExt;
use crate::renderer::renderer::BASE_LEVEL;
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

        let cont = Self::octaves(&self.continent, nx * 0.002, nz * 0.002, 4, 0.5, 2.0);

        if cont > 0.8 {
            return Biome::Mountain;
        }

        let temp = Self::octaves(&self.temperature, nx * 0.001, nz * 0.001, 3, 0.5, 2.0);
        let humid = Self::octaves(&self.humidity, nx * 0.001, nz * 0.001, 3, 0.5, 2.0);

        let t = (temp * 0.5 + 0.5).clamp(0.0, 1.0);
        let h = (humid * 0.5 + 0.5).clamp(0.0, 1.0);

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
    Plains, Meadow, Mountain, Forest, Jungle, Desert, Savanna, Taiga, Tundra, IceSpikes
}

impl Biome {
    pub fn surface_block(self, y: i32, slope: f32) -> BlockType {
        match self {
            Biome::Mountain => {
                if y > 110 { return BlockType::Snow; }
                if slope > 1.3 || y > 85 { BlockType::Stone }
                else { BlockType::Grass }
            },
            Biome::Meadow => {
                if y > 115 { BlockType::Snow }
                else { BlockType::Grass }
            },
            Biome::Desert => BlockType::Sand,
            Biome::Tundra | Biome::IceSpikes => BlockType::Snow,
            _ => {
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
    pub fn is_empty(self) -> bool {
        matches!(self, BlockType::Air)
    }

    pub fn is_transparent(self) -> bool {
        matches!(self, BlockType::Air | BlockType::Water)
    }

    pub fn texture_index(self) -> u32 {
        match self {
            BlockType::Stone => 0, BlockType::Dirt => 1, BlockType::Grass => 2,
            BlockType::Sand => 3, BlockType::Water => 4, BlockType::Snow => 5,
            BlockType::Air => 0,
        }
    }

    pub fn uv_range(self) -> [f32; 4] {
        let index = self.texture_index() as f32;
        let total_width = 192.0;
        let tile_size = 16.0;
        let slot_size = 32.0;

        let x_start = index * slot_size;
        let x_end = x_start + tile_size;

        let eps = 0.5;
        let u_start = (x_start + eps) / total_width;
        let u_end = (x_end - eps) / total_width;

        let v_start = eps / 16.0;
        let v_end = (16.0 - eps) / 16.0;

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

    pub fn to_world_coordinates(&self) -> Vector3<f32> {
        Vector3 {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub uv_rect: [f32; 4],
    pub tex_index: u32,
    pub padding: [u32; 3],
}

impl TerrainVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 20, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Uint32 },
            ],
        }
    }
}

pub struct MeshData {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub height_map: [[i32; CHUNK_SIZE]; CHUNK_SIZE],
}

#[derive(Debug)]
pub struct Chunk {
    pub position: ChunkCoordinates,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub height_map: [[i32; CHUNK_SIZE]; CHUNK_SIZE],
}

impl Chunk {
    pub fn from_data(device: &wgpu::Device, position: ChunkCoordinates, data: MeshData) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk VBO"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk IBO"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            position,
            vertex_buffer,
            index_buffer,
            index_count: data.indices.len() as u32,
            height_map: data.height_map,
        }
    }

    pub fn upload_to_gpu(&mut self, device: &wgpu::Device, data: MeshData) {
        self.index_count = data.indices.len() as u32;
        self.height_map = data.height_map;

        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk VBO"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk IBO"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    }

    pub fn build_mesh_data(noise: &Arc<TerrainNoise>, position: ChunkCoordinates) -> MeshData {
        let start_x = position.x;
        let start_z = position.z;

        // voxel gen
        let mut local_heights = [[0i32; CHUNK_SIZE + 1]; CHUNK_SIZE + 1];
        let mut final_height_map = [[0i32; CHUNK_SIZE]; CHUNK_SIZE];

        for z in 0..=CHUNK_SIZE {
            for x in 0..=CHUNK_SIZE {
                let h = noise.sample_height((start_x + x as i32) as f64, (start_z + z as i32) as f64);
                local_heights[x][z] = h;
                if x < CHUNK_SIZE && z < CHUNK_SIZE {
                    final_height_map[x][z] = h;
                }
            }
        }

        let block_grid: Vec<[[BlockType; CHUNK_SIZE]; CHUNK_SIZE]> = (0..MAX_HEIGHT)
            .into_par_iter()
            .map(|y| {
                let mut layer = [[BlockType::Air; CHUNK_SIZE]; CHUNK_SIZE];
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let h = local_heights[x][z];
                        let dx = (local_heights[x + 1][z] - h) as f32;
                        let dz = (local_heights[x][z + 1] - h) as f32;
                        let slope = (dx * dx + dz * dz).sqrt();
                        let yi = y as i32;

                        layer[x][z] = if yi > h {
                            if yi <= WATER_LEVEL { BlockType::Water } else { BlockType::Air }
                        } else if yi == h {
                            noise.sample_biome((start_x + x as i32) as f64, (start_z + z as i32) as f64).surface_block(yi, slope)
                        } else if yi > h - 4 {
                            BlockType::Dirt
                        } else {
                            BlockType::Stone
                        };
                    }
                }
                layer
            })
            .collect();

        // greedy meshing
        let mut vertices = Vec::with_capacity(1024);
        let mut indices = Vec::with_capacity(2048);
        let mut v_idx = 0;
        let dims = [CHUNK_SIZE as i32, MAX_HEIGHT as i32, CHUNK_SIZE as i32];

        for axis in 0..3 {
            let u = (axis + 1) % 3;
            let v = (axis + 2) % 3;
            let mut x = [0; 3];
            let mut q = [0; 3]; q[axis] = 1;

            for i in -1..dims[axis] {
                let mut mask = vec![BlockType::Air; (dims[u] * dims[v]) as usize];
                for j in 0..dims[v] {
                    for k in 0..dims[u] {
                        x[axis] = i; x[u] = k; x[v] = j;
                        let b0 = Self::get_block_static(&block_grid, x[0], x[1], x[2]);
                        let b1 = Self::get_block_static(&block_grid, x[0] + q[0], x[1] + q[1], x[2] + q[2]);

                        if b0.is_transparent() != b1.is_transparent() {
                            mask[(k + j * dims[u]) as usize] = if b0.is_transparent() { b1 } else { b0 };
                        } else if b0 == BlockType::Water && b1 == BlockType::Air {
                            mask[(k + j * dims[u]) as usize] = b0;
                        } else if b0 == BlockType::Air && b1 == BlockType::Water {
                            mask[(k + j * dims[u]) as usize] = b1;
                        }
                    }
                }

                let mut n = 0;
                for j in 0..dims[v] {
                    let mut k = 0;
                    while k < dims[u] {
                        if mask[n] != BlockType::Air {
                            let block = mask[n];
                            let mut w = 1;
                            while k + w < dims[u] && mask[n + w as usize] == block { w += 1; }
                            let mut h = 1;
                            'h_loop: while j + h < dims[v] {
                                for dw in 0..w {
                                    if mask[(n as i32 + dw + h * dims[u]) as usize] != block { break 'h_loop; }
                                }
                                h += 1;
                            }

                            let mut pos = [0; 3];
                            pos[axis] = i + q[axis]; pos[u] = k; pos[v] = j;

                            let left_block = Self::get_block_static(&block_grid, x[0], x[1], x[2]);
                            let back_face = if block == BlockType::Water {
                                left_block != BlockType::Water
                            } else {
                                left_block.is_transparent()
                            };

                            Self::add_quad_static(&mut vertices, &mut indices, &mut v_idx, position, pos, [u, v], [w, h], block, back_face);

                            for dh in 0..h {
                                for dw in 0..w { mask[(n as i32 + dw + dh * dims[u]) as usize] = BlockType::Air; }
                            }
                            k += w; n += w as usize;
                        } else {
                            k += 1; n += 1;
                        }
                    }
                }
            }
        }
        MeshData {
            vertices,
            indices,
            height_map: final_height_map
        }
    }

    fn get_block_static(grid: &Vec<[[BlockType; CHUNK_SIZE]; CHUNK_SIZE]>, x: i32, y: i32, z: i32) -> BlockType {
        if x < 0 || x >= CHUNK_SIZE as i32 || y < 0 || y >= MAX_HEIGHT as i32 || z < 0 || z >= CHUNK_SIZE as i32 {
            return BlockType::Air;
        }
        grid[y as usize][x as usize][z as usize]
    }

    fn add_quad_static(
        verts: &mut Vec<TerrainVertex>, idxs: &mut Vec<u32>, v_idx: &mut u32,
        chunk_pos: ChunkCoordinates, pos: [i32; 3], axes: [usize; 2], size: [i32; 2],
        block: BlockType, is_back_face: bool
    ) {
        let u = axes[0]; let v = axes[1];
        let w = size[0] as f32; let h = size[1] as f32;
        let uv_rect = block.uv_range();
        let tex_index = block.texture_index();

        let mut du = [0.0f32; 3]; du[u] = w;
        let mut dv = [0.0f32; 3]; dv[v] = h;
        let p_f = [pos[0] as f32, pos[1] as f32, pos[2] as f32];

        let to_world = |p: [f32; 3]| [
            (chunk_pos.x as f32 + p[0]) * BLOCK_WIDTH,
            (BASE_LEVEL + p[1]) * BLOCK_WIDTH,
            (chunk_pos.z as f32 + p[2]) * BLOCK_WIDTH,
        ];

        verts.push(TerrainVertex { position: to_world(p_f), uv: [0.0, h], uv_rect, tex_index, padding: [0; 3] });
        verts.push(TerrainVertex { position: to_world([p_f[0]+du[0], p_f[1]+du[1], p_f[2]+du[2]]), uv: [w, h], uv_rect, tex_index, padding: [0; 3] });
        verts.push(TerrainVertex { position: to_world([p_f[0]+du[0]+dv[0], p_f[1]+du[1]+dv[1], p_f[2]+du[2]+dv[2]]), uv: [w, 0.0], uv_rect, tex_index, padding: [0; 3] });
        verts.push(TerrainVertex { position: to_world([p_f[0]+dv[0], p_f[1]+dv[1], p_f[2]+dv[2]]), uv: [0.0, 0.0], uv_rect, tex_index, padding: [0; 3] });

        let b = *v_idx;
        if is_back_face {
            idxs.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        } else {
            idxs.extend_from_slice(&[b, b + 2, b + 1, b, b + 3, b + 2]);
        }
        *v_idx += 4;
    }
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}