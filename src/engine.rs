use winit::window::Window;
use crate::renderer::graphics::Graphics;

pub struct Engine {
    tick_ms : u32,
    //player : Player,
    //camera : Camera
    //renderer : Renderer
}

impl Engine {
    pub fn new(graphics: &Graphics) -> Self{
        let tick_ms = 16;
        Self{
            tick_ms
        }
    }
}