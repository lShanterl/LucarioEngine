use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wgpu::naga::Block;
use crate::core::chunk::{Chunk, ChunkCoordinates};
use crate::object::{BlockTypes, Material, Mesh};

pub(crate) const RENDER_DISTANCE : i32 = 32;

#[derive(Debug)]
pub struct SceneManager {
    scene: Scene,
    materials: HashMap<u32, Material>,
    next_material_id: u32,
}


impl SceneManager {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            materials: HashMap::new(),
            next_material_id: 0,
        }
    }

    pub fn add_chunk(&mut self, chunk: Arc<Chunk>) {
        self.scene.chunks.insert(chunk.position, chunk);
    }
    pub fn get_chunk(&self, chunk_coordinates: &ChunkCoordinates) -> &Arc<Chunk> {
        &self.scene.chunks[chunk_coordinates]
    }
    pub fn get_chunk_array(&self) -> &HashMap<ChunkCoordinates,Arc<Chunk>> {
        &self.scene.chunks
    }
    pub fn get_chunk_mut_array(&mut self) -> &mut HashMap<ChunkCoordinates,Arc<Chunk>> {
        &mut self.scene.chunks
    }

    pub fn add_material(&mut self, block_type: u32, material: Material){
        let material_id = self.next_material_id;
        self.next_material_id += 1;

        self.materials.insert(block_type, material);
    }

    pub fn get_material(&self, block_type: u32) -> &Material {
        self.materials.get(&block_type).unwrap()
    }

    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    pub fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> u32 {
        let mesh_id = self.scene.meshes.len() as u32;
        self.scene.meshes.insert(mesh_id, mesh);
        mesh_id
    }

    pub fn get_mesh(&self, mesh_id: u32) -> &Mesh {
        self.scene.meshes.get(&mesh_id).unwrap()
    }

    pub fn get_mesh_mut(&mut self, mesh_id: u32) -> &mut Mesh {
        self.scene.meshes.get_mut(&mesh_id).unwrap()
    }

    pub fn remove_mesh(&mut self, mesh_id: u32) {
        self.scene.meshes.remove(&mesh_id);
    }

    pub fn clear_scene(&mut self) {
        self.scene.meshes.clear();
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<u32, Mesh> {
        self.scene.meshes.iter()
    }
}

#[derive(Debug)]
pub struct Scene {
    pub(crate) meshes: HashMap<u32, Mesh>,
    
    pub(crate) chunks: HashMap<ChunkCoordinates, Arc<Chunk>>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            meshes: HashMap::new(),
            chunks: HashMap::new(),
        }
    }
}