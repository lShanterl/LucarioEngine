mod core;
mod object;
mod renderer;
mod texture;
mod utils;

use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
use crate::core::app::App;

async fn run() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(800u32, 600u32))
            .with_title("LucarioProject - Voxel engine")
            .build(&event_loop)
            .expect("Failed to build window"),
    );

    let mut app = App::new(window.clone()).await;
    let mut last_frame = instant::Instant::now();

    event_loop
        .run(move |event, control_flow| match event {
            Event::DeviceEvent { ref event, .. } => {
                app.on_device_event(event);
            }

            Event::WindowEvent { ref event, window_id }
            if window_id == app.window_id() =>
                {
                    if !app.on_window_event(event) {
                        match event {
                            WindowEvent::Resized(size) => {
                                app.handle_resize(*size);
                            }

                            WindowEvent::RedrawRequested => {
                                let now = instant::Instant::now();
                                let dt = now - last_frame;
                                last_frame = now;

                                app.update(dt);

                                match app.render() {
                                    Ok(()) => {}
                                    Err(wgpu::SurfaceError::Lost) => {
                                        app.handle_resize(app.surface_size());
                                    }
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        control_flow.exit();
                                    }
                                    Err(e) => eprintln!("render error: {:?}", e),
                                }

                                app.end_frame();
                            }

                            WindowEvent::CloseRequested => control_flow.exit(),
                            _ => {}
                        }
                    }
                }

            Event::AboutToWait => {
                app.request_redraw();
            }

            _ => {}
        })
        .expect("Event loop failed");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    run().await;
}