use vello::{
    kurbo::{Affine, PathEl},
    peniko::{Color, Fill},
    util::{RenderContext, RenderSurface},
    Renderer, RendererOptions, Scene, SceneBuilder,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct RenderState {
    vello_context: RenderContext,
    vello_surface: RenderSurface,
    vello_renderer: Renderer,
}

fn on_resumed(window: &Window) -> Result<RenderState, Box<dyn std::error::Error>> {
    let mut context = RenderContext::new()?;
    let size = window.inner_size();
    let surface = pollster::block_on(context.create_surface(window, size.width, size.height))?;
    let wgpu_device = &context.devices[surface.dev_id].device;
    let wgpu_queue = &context.devices[surface.dev_id].queue;
    let renderer = Renderer::new(
        wgpu_device,
        &RendererOptions {
            surface_format: Some(surface.format),
            timestamp_period: wgpu_queue.get_timestamp_period(),
        },
    )?;

    Ok(RenderState {
        vello_context: context,
        vello_surface: surface,
        vello_renderer: renderer,
    })
}

fn on_redraw(renderer_state: &mut RenderState) -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    let mut builder = SceneBuilder::for_scene(&mut scene);
    builder.fill(
        Fill::EvenOdd,
        Affine::translate((100.5, 100.0)),
        Color::rgb8(255, 0, 0),
        None,
        &[
            // left rect
            PathEl::MoveTo((0.0, 0.0).into()),
            PathEl::LineTo((0.0, 100.0).into()),
            PathEl::LineTo((100.0, 100.0).into()),
            PathEl::LineTo((100.0, 0.0).into()),
            // right rect
            PathEl::MoveTo((100.0, 0.0).into()),
            PathEl::LineTo((200.0, 0.0).into()),
            PathEl::LineTo((200.0, 100.0).into()),
            PathEl::LineTo((100.0, 100.0).into()),
        ],
    );

    let render_params = vello::RenderParams {
        base_color: Color::BLACK,
        width: WIDTH,
        height: HEIGHT,
    };

    let render_texture = renderer_state.vello_surface.surface.get_current_texture()?;
    let wgpu_device =
        &renderer_state.vello_context.devices[renderer_state.vello_surface.dev_id].device;
    let wgpu_queue =
        &renderer_state.vello_context.devices[renderer_state.vello_surface.dev_id].queue;

    renderer_state.vello_renderer.render_to_surface(
        wgpu_device,
        wgpu_queue,
        &scene,
        &render_texture,
        &render_params,
    )?;

    render_texture.present();
    wgpu_device.poll(wgpu::Maintain::Poll);

    Ok(())
}

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("native winit!")
        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .expect("can not build window");
    let mut render_state = None::<RenderState>;

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait();
        match event {
            Event::Suspended => {
                println!("Suspending");
                *control_flow = ControlFlow::Wait;
            }
            Event::Resumed => {
                println!("Resumed");
                render_state = Some(on_resumed(&window).expect("can not resume"));
                *control_flow = ControlFlow::Poll;
            }
            Event::RedrawRequested(_) => {
                if let Some(render_state) = &mut render_state {
                    on_redraw(render_state).expect("can not redraw");
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                if window.id() != window_id {
                    return;
                }
                let Some(render_state) = &mut render_state else { return; };
                match event {
                    WindowEvent::CloseRequested => {
                        println!("Close requested");
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        println!("Resized: {:?}", size);
                        render_state.vello_context.resize_surface(
                            &mut render_state.vello_surface,
                            size.width,
                            size.height,
                        );
                        window.request_redraw();
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    });
}
