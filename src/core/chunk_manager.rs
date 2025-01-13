use std::cmp::Ordering;
use std::sync::Arc;
use cgmath::Vector3;
use noise::core::perlin::perlin_2d;
use noise::{Fbm, MultiFractal, Perlin, Seedable};
use wgpu::Device;
use crate::core::chunk::Chunk;
use crate::core::scene_manager::SceneManager;
use crate::core::chunk::ChunkCoordinates;
use crate::core::scene_manager;
use crate::renderer::camera::{frustum_contains, Plane};
use crate::renderer::renderer::Instance;

// code derived from blatko's implementation
pub struct ChunkGenerator {
    data_sender: flume::Sender<Arc<Chunk>>,
    data_receiver: flume::Receiver<Arc<Chunk>>,

    chunk_load_queue: Vec<ChunkCoordinates>,

    pub chunk_rebuild_queue: Vec<ChunkCoordinates>,

    data_in_progress: Vec<ChunkCoordinates>,
    perlin_noise: Arc<Fbm::<Perlin>>,
}

impl ChunkGenerator {
    pub fn new() -> Self {
        let (data_sender, data_receiver) = flume::unbounded();
        let seed: u32 = rand::random();
        let perlin_noise = Arc::new(Fbm::<Perlin>::new(0)
            .set_seed(seed + 10)
            .set_frequency(0.1)
            .set_octaves(4)
            .set_lacunarity(2.0)
            .set_persistence(0.5)
        );

        ChunkGenerator {
            data_sender,
            data_receiver,
            chunk_load_queue: Vec::new(),
            chunk_rebuild_queue: Vec::new(),
            data_in_progress: Vec::new(),
            perlin_noise,
        }
    }

    pub fn build_chunks(
        &mut self,
        device: Arc<Device>,
        scene_manager: &mut SceneManager,
        player_position: Vector3<f32>,
        pool: &uvth::ThreadPool,
        frustum: &[Plane; 6],
    ){
        self.load_primary_chunks(device.clone(), scene_manager, player_position, pool);

        self.enqueue_chunks_in_frustum(player_position, scene_manager, frustum);
        self.process_chunk_loading_queue(device.clone(), scene_manager, pool);
        self.process_rebuild_queue(device.clone(), scene_manager, pool);

        self.update_scene(scene_manager);

        self.clean_up_queues();
        self.filter_unseen_chunks(player_position, scene_manager);


    }

    fn enqueue_chunks_in_frustum(&mut self, player_position: Vector3<f32>, scene_manager: &SceneManager, frustum: &[Plane; 6]) {
        let player_chunk_position = ChunkCoordinates::new(
            (player_position.x / 16.0).floor()  as i32,
            0,
            (player_position.z / 16.0) .floor() as i32,
        );

        for radius in 2..4 {
            for z in -radius..radius {
                self.enqueue_data(scene_manager, ChunkCoordinates::new(
                    player_chunk_position.x + radius  * 16,
                    0,
                    player_chunk_position.z + z  * 16
                ), &frustum);
                self.enqueue_data(scene_manager, ChunkCoordinates::new(
                    player_chunk_position.x - radius  * 16,
                    0,
                    player_chunk_position.z + z  * 16
                ), &frustum);
            }
            for x in (-radius + 1)..radius {
                self.enqueue_data(scene_manager, ChunkCoordinates::new(
                    player_chunk_position.x + x  * 16,
                    0,
                    player_chunk_position.z + radius  * 16
                ), &frustum);
                self.enqueue_data(scene_manager, ChunkCoordinates::new(
                    player_chunk_position.x + x  * 16,
                    0,
                    player_chunk_position.z - radius * 16
                ), &frustum);
            }
        }

        for pos in self.chunk_load_queue.clone() {
            self.chunk_rebuild_queue.push(pos);
        }
        self.chunk_load_queue.clear();
    }

    fn update_scene(&mut self, scene_manager: &mut SceneManager) {
        match self.data_receiver.try_recv() {
            Ok(chunk) => {
                let position = chunk.position;
                //println!(
                //    "Loaded chunk at: x: {}, y: {}, z: {}",
                //    position.x, position.y, position.z
                //);
                scene_manager.add_chunk(chunk);
                self.data_in_progress.retain(|pos| *pos != position);

                self.rebuild_adjacent_chunks(scene_manager, position);
            }
            Err(_) => {}
        }

    }
    fn rebuild_adjacent_chunks(&mut self, scene_manager: &mut SceneManager, pos: ChunkCoordinates) {
        if !scene_manager.get_chunk_array().is_empty() {
            if scene_manager
                .get_chunk_array()
                .contains_key(&ChunkCoordinates::new(pos.x + 16, 0, pos.z))
            {
                self.chunk_rebuild_queue
                    .push(ChunkCoordinates::new(pos.x + 16, 0, pos.z));
            }
            if scene_manager
                .get_chunk_array()
                .contains_key(&ChunkCoordinates::new(pos.x - 16, 0, pos.z))
            {
                self.chunk_rebuild_queue
                    .push(ChunkCoordinates::new(pos.x - 16, 0, pos.z));
            }
            if scene_manager
                .get_chunk_array()
                .contains_key(&ChunkCoordinates::new(pos.x, 0, pos.z + 16))
            {
                self.chunk_rebuild_queue
                    .push(ChunkCoordinates::new(pos.x, 0, pos.z + 16));
            }
            if scene_manager
                .get_chunk_array()
                .contains_key(&ChunkCoordinates::new(pos.x, 0, pos.z - 16))
            {
                self.chunk_rebuild_queue
                    .push(ChunkCoordinates::new(pos.x, 0, pos.z - 16));
            }
        }
    }

    fn process_rebuild_queue(
        &mut self,
        device: Arc<Device>,
        scene_manager: &mut SceneManager,
        pool: &uvth::ThreadPool
    ) {
        if !self.chunk_rebuild_queue.is_empty() {
            let pos = self.chunk_rebuild_queue.remove(0);
            let sender = self.data_sender.clone();
            let device = device.clone();
            let perlin_noise = self.perlin_noise.clone();

            pool.execute(move || {
                let chunk = Chunk::new(&device, pos, &perlin_noise);
                sender.send(Arc::new(chunk)).unwrap();
            });
            self.data_in_progress.push(pos);
        }
    }

    fn process_chunk_loading_queue(
        &mut self,
        device: Arc<Device>,
        scene_manager: &mut SceneManager,
        pool: &uvth::ThreadPool
    ) {
        if !self.chunk_load_queue.is_empty() {
            let position = self.chunk_load_queue.remove(0);
            let adjacent_chunks = self.adjacent_chunks(&position, scene_manager);
            let sender = self.data_sender.clone();
            let device = device.clone();
            let perlin_noise = self.perlin_noise.clone();

            pool.execute(move || {
                let chunk = Chunk::new(&device, position, &perlin_noise);
                sender.send(Arc::new(chunk)).unwrap();
            });
            self.data_in_progress.push(position);
        }
    }

    fn load_primary_chunks(
        &mut self,
        device: Arc<Device>,
        scene_manager: &SceneManager,
        player_position: Vector3<f32>,
        pool: &uvth::ThreadPool
    ) {
        let player_chunk_position = ChunkCoordinates::new(
            (player_position.x / 16.0 ).floor() as i32,
            0,
            (player_position.z / 16.0 ).floor() as i32,
        );
        //println!(
        //    "Player chunk position: x: {}, y: {}, z: {}",
        //    player_chunk_position.x, player_chunk_position.y, player_chunk_position.z
        //);

        self.load_chunk_directly(device.clone(), player_chunk_position, scene_manager, pool);

        let radius = 1;

        for z in -radius..radius {
            self.load_chunk_directly(
                device.clone(),
                ChunkCoordinates::new(
                    player_chunk_position.x + radius  * 16,
                    0,
                    player_chunk_position.z + z * 16
                ),
                scene_manager,
                pool
            );
            self.load_chunk_directly(
                device.clone(),
                ChunkCoordinates::new(
                    player_chunk_position.x - radius  * 16,
                    0,
                    player_chunk_position.z + z  * 16
                ),
                scene_manager,
                pool
            );
        }
        for x in (-radius + 1)..radius {
            self.load_chunk_directly(
                device.clone(),
                ChunkCoordinates::new(
                    player_chunk_position.x + x  * 16,
                    0,
                    player_chunk_position.z + radius  * 16
                ),
                scene_manager,
                pool
            );
            self.load_chunk_directly(
                device.clone(),
                ChunkCoordinates::new(
                    player_chunk_position.x + x  * 16,
                    0,
                    player_chunk_position.z - radius  * 16
                ),
                scene_manager,
                pool
            );
        }

    }

    fn load_chunk_directly(
        &mut self,
        device: Arc<Device>,
        position: ChunkCoordinates,
        scene_manager: &SceneManager,
        pool: &uvth::ThreadPool
    ){
        if !self.is_chunk_loaded(&scene_manager, position){
            let sender = self.data_sender.clone();
            let device = device.clone();
            let perlin_noise = self.perlin_noise.clone();

            pool.execute(move || {
                let chunk = Chunk::new(&device, position, &perlin_noise);

                sender.send(Arc::new(chunk)).unwrap();
            });
            self.data_in_progress.push(position);
        }
    }

    fn clean_up_queues(&mut self) {
        self.chunk_load_queue.clear();
        self.chunk_rebuild_queue.sort();
        self.chunk_rebuild_queue.dedup();

    }

    fn enqueue_data(&mut self, scene_manager: &SceneManager, position: ChunkCoordinates, frustum: &[Plane; 6]) {

        if !self.is_chunk_loaded(scene_manager, position) {
            if frustum_contains(frustum, &position){
                self.chunk_load_queue.push(position);
            }
        }
    }

    fn is_chunk_loaded(&self, scene_manager: &SceneManager, position: ChunkCoordinates) -> bool {
        if !scene_manager.get_chunk_array().contains_key(&position) {
            if !self.chunk_load_queue.contains(&position) {
                if !self.data_in_progress.contains(&position) {
                    return false;
                }
            }
        }
        true
    }

    fn filter_unseen_chunks(&mut self, player_position: cgmath::Vector3<f32>, scene_manager: &mut SceneManager) {
        let chunk_size = 16;

        let player_chunk = ChunkCoordinates::new(
            (player_position.x / chunk_size as f32).floor() as i32,
            (player_position.y / chunk_size as f32).floor() as i32,
            (player_position.z / chunk_size as f32).floor() as i32,
        );

        let chunks = scene_manager.get_chunk_mut_array();
        let mut to_remove = Vec::new();

        chunks.retain(|p, _| {
            if p.x <= scene_manager::RENDER_DISTANCE + player_chunk.x
                && p.z <= scene_manager::RENDER_DISTANCE + player_chunk.z
                && p.x >= -scene_manager::RENDER_DISTANCE + player_chunk.x
                && p.z >= -scene_manager::RENDER_DISTANCE + player_chunk.z
                && p.y <= scene_manager::RENDER_DISTANCE + player_chunk.y
                && p.y >= -scene_manager::RENDER_DISTANCE + player_chunk.y
            {
                return true;
            }
            to_remove.push(*p);

            false
        });

        for p in to_remove {
            chunks.remove(&p);
            if let Some(i) = self.chunk_rebuild_queue.iter().position(|&pos| pos == p) {
                self.chunk_rebuild_queue.remove(i);
            }
        }

    }

    fn adjacent_chunks(&mut self, pos: &ChunkCoordinates, scene_manager: &SceneManager) -> Vec<Option<Arc<Chunk>>> {
        let mut adjacent_chunks = Vec::new();
        if let Some(c) = scene_manager
            .get_chunk_array()
            .get(&ChunkCoordinates::new(pos.x - 16, pos.y, pos.z))
        {
            adjacent_chunks.push(Some(c.clone()));
        } else {
            adjacent_chunks.push(None);
        }
        if let Some(c) = scene_manager
            .get_chunk_array()
            .get(&ChunkCoordinates::new(pos.x + 16, pos.y, pos.z))
        {
            adjacent_chunks.push(Some(c.clone()));
        } else {
            adjacent_chunks.push(None);
        }
        if let Some(c) = scene_manager
            .get_chunk_array()
            .get(&ChunkCoordinates::new(pos.x, pos.y, pos.z - 16))
        {
            adjacent_chunks.push(Some(c.clone()));
        } else {
            adjacent_chunks.push(None);
        }
        if let Some(c) = scene_manager
            .get_chunk_array()
            .get(&ChunkCoordinates::new(pos.x, pos.y, pos.z + 16))
        {
            adjacent_chunks.push(Some(c.clone()));
        } else {
            adjacent_chunks.push(None);
        }
        adjacent_chunks
    }

}