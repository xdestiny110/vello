use std::fs::File;
use vello::kurbo::{PathEl, QuadBezIter};
use vello::peniko::{kurbo::BezPath, kurbo::Rect, Color, Fill};
use vello::{block_on_wgpu, RendererOptions};
use vello::{
    kurbo::{Affine, Point},
    util::RenderContext,
    Renderer, Scene, SceneBuilder,
};
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, ImageCopyBuffer,
    TextureUsages,
};

const WIDTH: u32 = 500;
const HEIGHT: u32 = 500;

fn read_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: Extent3d,
    label: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let padded_byte_width = {
        let w = size.width * 4;
        match w % 256 {
            0 => w,
            r => w + (256 - r),
        }
    };
    let buffer_size = padded_byte_width as u64 * size.height as u64;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: buffer_size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Copy out buffer"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_byte_width),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);

    let buf_slice = buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buf_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    if let Some(recv_result) = block_on_wgpu(device, receiver.receive()) {
        recv_result?;
    }
    let data = buf_slice.get_mapped_range();
    let mut result_unpadded = Vec::<u8>::with_capacity((size.width * size.height * 4).try_into()?);
    for row in 0..size.height {
        let start = (row * padded_byte_width).try_into()?;
        result_unpadded.extend(&data[start..start + (size.width * 4) as usize]);
    }
    Ok(result_unpadded)
}

fn write_image(
    data: &Vec<u8>,
    width: u32,
    height: u32,
    out_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(&out_path)?;
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&data)?;
    writer.finish()?;
    Ok(())
}

fn main() {
    let mut context = RenderContext::new().expect("can not create context");
    let device_id = pollster::block_on(context.device(None)).expect("can not create device");

    let device_handle = &context.devices[device_id];
    let device = &device_handle.device;
    let queue = &device_handle.queue;

    let size = Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test texture"),
        size: size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = Renderer::new(
        device,
        &RendererOptions {
            surface_format: None,
            timestamp_period: queue.get_timestamp_period(),
        },
    )
    .expect("can not create renderer");

    let mut scene = Scene::new();
    let mut builder = SceneBuilder::for_scene(&mut scene);
    builder.fill(
        Fill::NonZero,
        Affine::translate((100.5, 100.0)),
        Color::rgb8(255, 0, 0),
        None,
        &[
            PathEl::MoveTo((0.0, 0.0).into()),
            PathEl::LineTo((0.0, 100.0).into()),
            PathEl::LineTo((100.0, 100.0).into()),
            PathEl::LineTo((100.0, 0.0).into()),
        ],
    );
    builder.fill(
        Fill::NonZero,
        Affine::translate((100.5, 100.0)),
        Color::rgb8(255, 0, 0),
        None,
        &[
            PathEl::MoveTo((100.0, 0.0).into()),
            PathEl::LineTo((200.0, 0.0).into()),
            PathEl::LineTo((200.0, 100.0).into()),
            PathEl::LineTo((100.0, 100.0).into()),
        ],
    );
    // builder.fill(
    //     Fill::EvenOdd,
    //     Affine::translate((100.5, 100.0)),
    //     Color::rgb8(255, 0, 0),
    //     None,
    //     &[
    //         // left rect
    //         PathEl::MoveTo((0.0, 0.0).into()),
    //         PathEl::LineTo((0.0, 100.0).into()),
    //         PathEl::LineTo((100.0, 100.0).into()),
    //         PathEl::LineTo((100.0, 0.0).into()),
    //         // right rect
    //         PathEl::MoveTo((100.0, 0.0).into()),
    //         PathEl::LineTo((200.0, 0.0).into()),
    //         PathEl::LineTo((200.0, 100.0).into()),
    //         PathEl::LineTo((100.0, 100.0).into()),
    //     ],
    // );

    let render_params = vello::RenderParams {
        base_color: Color::BLACK,
        width: WIDTH,
        height: HEIGHT,
    };
    renderer
        .render_to_texture(device, queue, &scene, &texture_view, &render_params)
        .expect("can not render to texture");

    let data = read_pixel(device, queue, &texture, size, "read pixel").expect("can not read pixel");
    write_image(&data, size.width, size.height, "test.png").expect("can not write image");
}
