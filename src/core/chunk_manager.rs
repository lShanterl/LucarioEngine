use std::sync::Arc;
use cgmath::Vector3;
use wgpu::Device;
use crate::core::chunk::{Chunk, ChunkCoordinates, MeshData, TerrainNoise, CHUNK_SIZE};
use crate::core::scene_manager::{self, SceneManager};
use crate::renderer::camera::{frustum_contains, Plane};


const MAX_LOADS_PER_FRAME:   usize = 2;
const MAX_REBUILDS_PER_FRAME: usize = 2;
const PRIMARY_RADIUS:        i32   = 6;   // chunks (not blocks) around player
const FRUSTUM_RADIUS:        i32   = 16;


pub struct ChunkGenerator {
    data_sender:         flume::Sender<(ChunkCoordinates, MeshData)>,
    data_receiver:       flume::Receiver<(ChunkCoordinates, MeshData)>,
    chunk_load_queue:    Vec<ChunkCoordinates>,
    chunk_rebuild_queue: Vec<ChunkCoordinates>,
    data_in_progress:    Vec<ChunkCoordinates>,
    terrain_noise:       Arc<TerrainNoise>,
}

impl ChunkGenerator {
    pub fn new() -> Self {
        let (data_sender, data_receiver) = flume::unbounded::<(ChunkCoordinates, MeshData)>();
        let seed: u32 = rand::random();
        let terrain_noise = Arc::new(TerrainNoise::new(seed));

        Self {
            data_sender,
            data_receiver,
            chunk_load_queue: Vec::new(),
            chunk_rebuild_queue: Vec::new(),
            data_in_progress: Vec::new(),
            terrain_noise,
        }
    }

    pub fn build_chunks(
        &mut self,
        device:        Arc<Device>,
        scene_manager: &mut SceneManager,
        player_pos:    Vector3<f32>,
        pool:          &uvth::ThreadPool,
        frustum:       &[Plane; 6],
    ) {
        self.receive_finished(&device.clone(), scene_manager);
        self.enqueue_primary(device.clone(), scene_manager, player_pos, pool);
        self.enqueue_frustum(player_pos, scene_manager, frustum);
        self.dispatch_loads(device.clone(), pool, player_pos);
        self.dispatch_rebuilds(device.clone(), scene_manager, pool, player_pos);
        self.unload_distant(player_pos, scene_manager);
    }


    fn receive_finished(&mut self, device: &wgpu::Device, scene_manager: &mut SceneManager) {
        let mut uploads_this_frame = 0;

        while let Ok((pos, mesh_data)) = self.data_receiver.try_recv() {
            let chunk = Chunk::from_data(device, pos, mesh_data);

            scene_manager.add_chunk(Arc::new(chunk));

            self.data_in_progress.retain(|p| *p != pos);

            uploads_this_frame += 1;
            if uploads_this_frame >= MAX_LOADS_PER_FRAME { break; }
        }
    }

    fn enqueue_primary(
        &mut self,
        device:        Arc<Device>,
        scene_manager: &SceneManager,
        player_pos:    Vector3<f32>,
        pool:          &uvth::ThreadPool,
    ) {
        let pc = self.player_chunk(player_pos);
        let cs = CHUNK_SIZE as i32;
        for z in -PRIMARY_RADIUS..=PRIMARY_RADIUS {
            for x in -PRIMARY_RADIUS..=PRIMARY_RADIUS {
                let pos = ChunkCoordinates::new(pc.x + x * cs, 0, pc.z + z * cs);
                self.request_now(pos, device.clone(), scene_manager, pool);
            }
        }
    }

    fn enqueue_frustum(
        &mut self,
        player_pos:    Vector3<f32>,
        scene_manager: &SceneManager,
        frustum:       &[Plane; 6],
    ) {
        let pc = self.player_chunk(player_pos);
        let cs = CHUNK_SIZE as i32;
        for radius in (PRIMARY_RADIUS + 1)..FRUSTUM_RADIUS {
            for z in -radius..=radius {
                self.try_queue_frustum(ChunkCoordinates::new(pc.x + radius * cs, 0, pc.z + z * cs), scene_manager, frustum);
                self.try_queue_frustum(ChunkCoordinates::new(pc.x - radius * cs, 0, pc.z + z * cs), scene_manager, frustum);
            }
            for x in (-radius + 1)..radius {
                self.try_queue_frustum(ChunkCoordinates::new(pc.x + x * cs, 0, pc.z + radius * cs), scene_manager, frustum);
                self.try_queue_frustum(ChunkCoordinates::new(pc.x + x * cs, 0, pc.z - radius * cs), scene_manager, frustum);
            }
        }
    }

    fn try_queue_frustum(&mut self, pos: ChunkCoordinates, scene_manager: &SceneManager, frustum: &[Plane; 6]) {
        let a = self.align(pos);
        if !self.is_tracked(scene_manager, a) && frustum_contains(frustum, &a) {
            self.chunk_load_queue.push(a);
        }
    }

    fn dispatch_loads(&mut self, device: Arc<Device>, pool: &uvth::ThreadPool, player_pos: Vector3<f32>) {
        if self.chunk_load_queue.is_empty() { return; }
        let pc = self.player_chunk(player_pos);
        self.chunk_load_queue.sort_unstable_by_key(|p| p.distance_sq(&pc));
        self.chunk_load_queue.dedup();
        let n = MAX_LOADS_PER_FRAME.min(self.chunk_load_queue.len());
        let batch: Vec<_> = self.chunk_load_queue.drain(..n).collect();
        for pos in batch { self.spawn(pos, device.clone(), pool); }
    }

    fn dispatch_rebuilds(
        &mut self,
        device:        Arc<Device>,
        scene_manager: &SceneManager,
        pool:          &uvth::ThreadPool,
        player_pos:    Vector3<f32>,
    ) {
        if self.chunk_rebuild_queue.is_empty() { return; }
        let pc = self.player_chunk(player_pos);
        self.chunk_rebuild_queue.sort_unstable_by_key(|p| p.distance_sq(&pc));
        self.chunk_rebuild_queue.dedup();
        self.chunk_rebuild_queue.retain(|p| scene_manager.get_chunk_array().contains_key(p));
        let n = MAX_REBUILDS_PER_FRAME.min(self.chunk_rebuild_queue.len());
        let batch: Vec<_> = self.chunk_rebuild_queue.drain(..n).collect();
        for pos in batch {
            if !self.data_in_progress.contains(&pos) { self.spawn(pos, device.clone(), pool); }
        }
    }

    fn unload_distant(&mut self, player_pos: Vector3<f32>, scene_manager: &mut SceneManager) {
        let cs = CHUNK_SIZE as i32;
        let pc = self.player_chunk(player_pos);
        let rd = scene_manager::RENDER_DISTANCE + 2;
        let keep = |pos: &ChunkCoordinates| -> bool {
            (pos.x - pc.x).abs() / cs <= rd && (pos.z - pc.z).abs() / cs <= rd
        };
        scene_manager.get_chunk_mut_array().retain(|p, _| keep(p));
        self.data_in_progress.retain(|p| keep(p));
        self.chunk_load_queue.retain(|p| keep(p));
    }

    pub fn rebuild_chunk(&mut self, pos: ChunkCoordinates) {
        let cs = CHUNK_SIZE as i32;
        for n in [pos,
            ChunkCoordinates::new(pos.x + cs, 0, pos.z),
            ChunkCoordinates::new(pos.x - cs, 0, pos.z),
            ChunkCoordinates::new(pos.x, 0, pos.z + cs),
            ChunkCoordinates::new(pos.x, 0, pos.z - cs),
        ] {
            if !self.chunk_rebuild_queue.contains(&n) { self.chunk_rebuild_queue.push(n); }
        }
    }

    fn spawn(&mut self, pos: ChunkCoordinates, _device: Arc<Device>, pool: &uvth::ThreadPool) {
        if self.data_in_progress.contains(&pos) { return; }

        let sender = self.data_sender.clone();
        let noise  = self.terrain_noise.clone();

        // not passing the device, no gpu only cpu
        pool.execute(move || {
            let mesh_data = Chunk::build_mesh_data(&noise, pos);
            let _ = sender.send((pos, mesh_data));
        });
        self.data_in_progress.push(pos);
    }

    fn request_now(&mut self, pos: ChunkCoordinates, device: Arc<Device>, scene_manager: &SceneManager, pool: &uvth::ThreadPool) {
        let a = self.align(pos);
        if !self.is_tracked(scene_manager, a) { self.spawn(a, device, pool); }
    }

    #[inline]
    fn align(&self, pos: ChunkCoordinates) -> ChunkCoordinates {
        let cs = CHUNK_SIZE as i32;
        ChunkCoordinates::new(
            (pos.x as f32 / cs as f32).floor() as i32 * cs,
            0,
            (pos.z as f32 / cs as f32).floor() as i32 * cs,
        )
    }

    fn is_tracked(&self, scene_manager: &SceneManager, pos: ChunkCoordinates) -> bool {
        scene_manager.get_chunk_array().contains_key(&pos)
            || self.chunk_load_queue.contains(&pos)
            || self.data_in_progress.contains(&pos)
    }

    #[inline]
    fn player_chunk(&self, player_pos: Vector3<f32>) -> ChunkCoordinates {
        let bw = crate::core::chunk::BLOCK_WIDTH as f32;
        let cs = CHUNK_SIZE as f32;
        let block_x = (player_pos.x / bw).floor();
        let block_z = (player_pos.z / bw).floor();
        ChunkCoordinates::new(
            (block_x / cs).floor() as i32 * cs as i32,
            0,
            (block_z / cs).floor() as i32 * cs as i32,
        )
    }
}
