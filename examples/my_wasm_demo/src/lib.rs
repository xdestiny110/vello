use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    window::{Fullscreen, WindowBuilder},
};
use wasm_bindgen::prelude::*;
use console_log;
use vello::peniko::Color;
use vello::util::RenderSurface;
use vello::{
    kurbo::{Affine, Vec2},
    util::RenderContext,
    Renderer, Scene, SceneBuilder,
};
use vello::{BumpAllocators, RendererOptions, SceneFragment};


#[wasm_bindgen(start)]
pub fn main() {
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
    log::info!("Hello, world!");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&event_loop)
        .unwrap();

    let mut render_cx = RenderContext::new().unwrap();

    use winit::platform::web::WindowExtWebSys;
    let canvas = window.canvas();
    let size = window.inner_size();
    canvas.set_width(size.width);
    canvas.set_height(size.height);
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| body.append_child(canvas.as_ref()).ok())
        .expect("couldn't append canvas to document body");
    _ = web_sys::HtmlElement::from(canvas).focus();

    let size = window.inner_size();
    log::info!("window size: {:?}", size);
    wasm_bindgen_futures::spawn_local(async move {
        let size = window.inner_size();
        let surface = render_cx
            .create_surface(&window, size.width, size.height)
            .await;
        if let Ok(surface) = surface {
            // No error handling here; if the event loop has finished, we don't need to send them the surface
            event_loop.run(move |event, _, control_flow| {
                control_flow.set_wait();
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        window_id,
                    } if window_id == window.id() => control_flow.set_exit(),
                    Event::MainEventsCleared => {
                        log::info!("redraw");
                        window.request_redraw();
                    }
                    _ => (),
                }
            });
        }
    });

}