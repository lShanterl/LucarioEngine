use std::sync::Arc;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::engine::Engine;
use crate::renderer::graphics::Graphics;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::Device;
use winit::dpi::PhysicalSize;
use winit::event_loop::ControlFlow;

mod engine;
mod renderer;

pub struct Client {
    graphics: Graphics,
    engine: Engine,
}

impl Client{
    pub fn new(window: Arc<Window>) -> Client {
        let graphics = unsafe {Graphics::new(window)};
        let engine = Engine::new(&graphics);

        Self { graphics, engine }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    window.set_title("LucarioProject - Voxel engine");


    let mut client = Client::new(window);

    #[cfg(target_arch = "wasm32")]
    {
        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
    let mut new_size: Option<PhysicalSize<u32>> = None;
    event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {ref event, window_id }
        if window_id == client.graphics.window().id() => match event {
            WindowEvent::Resized(size) => {
                new_size = Some(*size);
            }
            WindowEvent::RedrawRequested => {
                if let Some(size) = new_size.take() {
                    client.graphics.resize(size);
                }

                client.graphics.update();
                match client.graphics.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => client.graphics.resize(client.graphics.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
                client.graphics.window().request_redraw();
            }
            WindowEvent::CloseRequested => control_flow.exit(),

            _ => {}
        },

        _ => {}
    }).expect("Window failed to open");

}

fn main() {
    run();
}
