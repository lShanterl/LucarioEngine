struct Client{
    graphics: Graphics,
    engine: Engine,
    pool: uvth::ThreadPool,
}

impl Client{
    fn new(window: &winit::window::Window) -> Self{
        let graphics = Graphics::new(&window);
        let engine = Engine::new(&graphics);
        let pool = uvth::ThreadPoolBuilder::new()
            .name("MainThreadPool".parse().unwrap())
            .build();
        Self {
            graphics,
            engine,
            pool,
        }
    }
    fn render() -> Self{
        todo!()
    }
}

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    window.set_title("wgpu voxel engine");

    let mut client = Client::new(&window);

    
}
