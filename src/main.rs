use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use std::time::{Duration, Instant};

// Structure for a frame
struct BgrFrame {
    height: usize,
    width: usize,
    data: Vec<u8>,
}

struct Producer {
    height: usize,
    width: usize,
    frame: usize,
}

impl Producer {
    fn new_with_size(height: usize, width: usize) -> Self {
        Producer{height, width, frame: 0}
    }
    fn next_frame(&mut self) -> BgrFrame {
        let data = (0 .. self.height * self.width)
            .flat_map(|i| {
                let x = i % self.width;
                let y = i / self.width;
                let col = (255.0 * (y as f32 / self.height as f32)) as u8 * (self.frame % 2) as u8;
                if ((x / 10) % 2) ^ ((y / 10) % 2) == 0 {
                    vec!(col as u8, 0x00 as u8, 0x00 as u8) // Blue
                } else {
                    vec!(0x00 as u8, 0x00 as u8, col as u8)
                }
            }).collect();
        let frame = BgrFrame {
            height: self.height,
            width: self.width,
            data,
        };
        self.frame += 1;
        return frame;
    }
}

fn bgr2bgra(data: Vec<u8>) -> Vec<u8> {
    let n_pix = data.len() / 3;
    let mut bgra = Vec::with_capacity(4 * n_pix);
    for i in 0 .. n_pix {
        bgra.push(data[i * 3 + 0]);
        bgra.push(data[i * 3 + 1]);
        bgra.push(data[i * 3 + 2]);
        bgra.push(1);
    }
    return bgra;
}

// This is painfully slow!! Why?
/*
fn bgr2bgra(data: Vec<u8>) -> Vec<u8> {
    data.chunks(3).flat_map(|chunk| {
        chunk.iter().map(u8::clone).chain(std::iter::once(1))
    }).collect()
}
*/

async fn run<F>(evl: EventLoop<()>, win: Window, mut next_frame: F)
    // Why 'static lifetime was necessary?
    where F: FnMut() -> BgrFrame + 'static {
    let size = win.inner_size();
    /* from master branch
    let instance = wgpu::Instance::new();
    let surface = unsafe { instance.create_surface(&win) };
    */
    let surface = wgpu::Surface::create(&win);

    //let adapter = instance.request_adapter(
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    ).await.unwrap();

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default()
        },
        // None,
    ).await; //.unwrap();

    let vs = include_bytes!("shader.vert.spv");
    let vs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());

    let fs = include_bytes!("shader.frag.spv");
    let fs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[
            wgpu::BindGroupLayoutEntry { // Accessing texture in frag
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: false,
                    component_type: wgpu::TextureComponentType::Float,
                    dimension: wgpu::TextureViewDimension::D2,
                },
            },
            wgpu::BindGroupLayoutEntry { // Sampling texture in frag
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: false },
            },
        ],
        label: None,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    // Create the pattern
    let frame = next_frame();
    let width = frame.width;
    let height = frame.height;
    let image_data = bgr2bgra(frame.data);
    // Create texture of required size
    let texture_extent = wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    let texture_view = texture.create_default_view();
    // Copy texture 
    let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let temp_buff = device.create_buffer_with_data(image_data.as_slice(), wgpu::BufferUsage::COPY_SRC);
    init_encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
            buffer: &temp_buff,
            offset: 0,
            bytes_per_row: 4 * width as u32,
            rows_per_image: 0,
        },
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        texture_extent,
    );
    queue.submit(&[init_encoder.finish()]);

    // Create a sampler to access texture data
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: -100.0,
        lod_max_clamp: 100.0,
        compare: wgpu::CompareFunction::Undefined,
    });

    // Create bind group to access texture
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });

    let swapchain_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor { module: &vs_module, entry_point: "main", },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor { module: &fs_module, entry_point: "main" }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
        color_states: &[wgpu::ColorStateDescriptor {
            format: swapchain_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let mut count = 0;

    let mut t_produce = Duration::new(0, 0);
    let mut t_encode = Duration::new(0, 0);
    let mut t_exec = Duration::new(0, 0);
    let mut t_present = Duration::new(0, 0);
    let mut now_present = Instant::now();

    evl.run(move |event, _, control_flow| {
        // Force ownership
        let _ = (
            //&instance,
            &adapter,
            &vs_module,
            &fs_module,
            &pipeline_layout,
        );
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => win.request_redraw(),
            // Resize events must rebuild the swap chain
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }
            Event::RedrawRequested(_) => {
                // Time since last now_present (end of frame encode)
                t_present += now_present.elapsed();

                // Get next frame
                let now = Instant::now();
                let image_data = bgr2bgra(next_frame().data);
                t_produce += now.elapsed();

                let now = Instant::now();
                let temp_buff = device.create_buffer_with_data(image_data.as_slice(), wgpu::BufferUsage::COPY_SRC);

                let frame = swap_chain.get_next_texture().expect("Timeout when acquiring next swap chain texture");
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label: None});
                encoder.copy_buffer_to_texture(
                    wgpu::BufferCopyView {
                        buffer: &temp_buff,
                        offset: 0,
                        bytes_per_row: 4 * width as u32,
                        rows_per_image: 0,
                    },
                    wgpu::TextureCopyView {
                        texture: &texture,
                        mip_level: 0,
                        array_layer: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    texture_extent,
                );
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color::GREEN,
                        }],
                        depth_stencil_attachment: None,
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.draw(0..4, 0..1);
                }
                t_encode += now.elapsed();
                let now = Instant::now();
                queue.submit(&[encoder.finish()]);
                t_exec += now.elapsed();
                
                // Measure time for next present
                now_present = Instant::now();

                count += 1;
                if count % 50 == 0 {
                    println!("Frame count {}", count);
                    println!("  t_produce: {}s", t_produce.as_secs());
                    println!("  t_encode: {}s", t_encode.as_secs());
                    println!("  t_exec: {}s", t_exec.as_secs());
                    println!("  t_present: {}s", t_exec.as_secs());
                    t_produce = Duration::new(0, 0);
                    t_encode = Duration::new(0, 0);
                    t_exec = Duration::new(0, 0);
                    t_present = Duration::new(0, 0);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}


fn main() {
    let evl = EventLoop::new();
    let win = Window::new(&evl).unwrap();
    env_logger::init();
    let mut producer = Producer::new_with_size(300, 400);

    futures::executor::block_on(run(evl, win, move || { producer.next_frame() }));
}
