use std::collections::HashMap;
use std::sync::Arc;
use crate::core::chunk::{Chunk, ChunkCoordinates, CHUNK_SIZE, BLOCK_WIDTH};
use crate::object::{Material, Mesh};
use crate::renderer::renderer::BASE_LEVEL;

pub const RENDER_DISTANCE: i32 = 16; // in chunk units
#[derive(Debug)]
pub struct SceneManager {
    scene:            Scene,
    materials:        HashMap<u32, Material>,
    next_material_id: u32,
}

impl SceneManager {
    pub fn new() -> Self {
        Self { scene: Scene::new(), materials: HashMap::new(), next_material_id: 0 }
    }

    pub fn add_chunk(&mut self, chunk: Arc<Chunk>) {
        self.scene.chunks.insert(chunk.position, chunk);
    }
    pub fn get_chunk(&self, pos: &ChunkCoordinates) -> &Arc<Chunk> {
        &self.scene.chunks[pos]
    }
    pub fn get_chunk_array(&self) -> &HashMap<ChunkCoordinates, Arc<Chunk>> {
        &self.scene.chunks
    }
    pub fn get_chunk_mut_array(&mut self) -> &mut HashMap<ChunkCoordinates, Arc<Chunk>> {
        &mut self.scene.chunks
    }

    pub fn surface_render_height_at(&self, render_x: f32, render_z: f32) -> Option<f32> {
        let bw = BLOCK_WIDTH;
        let cs = CHUNK_SIZE as i32;

        let block_x = (render_x / bw).floor() as i32;
        let block_z = (render_z / bw).floor() as i32;

        let chunk_x = (block_x as f32 / cs as f32).floor() as i32 * cs;
        let chunk_z = (block_z as f32 / cs as f32).floor() as i32 * cs;

        let local_x = (block_x - chunk_x) as usize;
        let local_z = (block_z - chunk_z) as usize;

        let chunk = self.scene.chunks.get(&ChunkCoordinates::new(chunk_x, 0, chunk_z))?;
        let surface_block_y = chunk.height_map[local_x][local_z];

        // top face of surface block = BASE_LEVEL + (surface_block_y + 1) * BLOCK_WIDTH
        Some(BASE_LEVEL + (surface_block_y + 1) as f32 * bw)
    }

    pub fn add_material(&mut self, block_type: u32, material: Material) {
        self.next_material_id += 1;
        self.materials.insert(block_type, material);
    }
    pub fn get_material(&self, block_type: u32) -> &Material {
        self.materials.get(&block_type).unwrap()
    }
    pub fn get_scene(&self)         -> &Scene      { &self.scene }
    pub fn get_scene_mut(&mut self) -> &mut Scene  { &mut self.scene }

    pub fn add_mesh(&mut self, mesh: Mesh) -> u32 {
        let id = self.scene.meshes.len() as u32;
        self.scene.meshes.insert(id, mesh);
        id
    }
    pub fn get_mesh(&self, id: u32)         -> &Mesh      { self.scene.meshes.get(&id).unwrap() }
    pub fn get_mesh_mut(&mut self, id: u32) -> &mut Mesh  { self.scene.meshes.get_mut(&id).unwrap() }
    pub fn remove_mesh(&mut self, id: u32)               { self.scene.meshes.remove(&id); }
    pub fn clear_scene(&mut self)                         { self.scene.meshes.clear(); }
    pub fn iter(&self) -> std::collections::hash_map::Iter<u32, Mesh> { self.scene.meshes.iter() }
}

#[derive(Debug)]
pub struct Scene {
    pub meshes: HashMap<u32, Mesh>,
    pub chunks: HashMap<ChunkCoordinates, Arc<Chunk>>,
}

impl Scene {
    pub fn new() -> Self {
        Self { meshes: HashMap::new(), chunks: HashMap::new() }
    }
}
