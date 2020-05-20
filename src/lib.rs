use futures::executor;
use winit::window::Window;

/// Converts a bgr8 buffer to a bgra8 buffer
pub fn bgr2bgra(data: &[u8]) -> Vec<u8> {
    let n_pix = data.len() / 3;
    let mut bgra = Vec::with_capacity(4 * n_pix);
    for i in 0..n_pix {
        bgra.push(data[i * 3 + 0]);
        bgra.push(data[i * 3 + 1]);
        bgra.push(data[i * 3 + 2]);
        bgra.push(1);
    }
    return bgra;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unable to access adapter")]
    NoAdapter,
    #[error("Swap chain timed out while acquiring next texture")]
    SwapChainTimeout(wgpu::TimeOut),
    #[error("Unable to load shader")]
    ShaderLoad(#[source] std::io::Error),
}

/// Stores the state of the wgpu renderer
pub struct Renderer {
    img_w: usize,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_format: wgpu::TextureFormat,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    texture: wgpu::Texture,
    texture_extent: wgpu::Extent3d,
}

impl Renderer {
    /// Creates new renderer for an image of size img_w, img_h on the given window
    pub fn new(win: &Window, img_w: usize, img_h: usize) -> Result<Self, Error> {
        let size = win.inner_size();
        let surface = wgpu::Surface::create(win);

        // The first step in using wgpu is to have an adapter, which describes
        // the hardware and implementation capabilities. The device and the queue
        // are instead operational, and they allow to perform render and computer.
        let (adapter, device, queue) = Self::setup_adapter_device_queue(&surface)?;
        // This format is supported by every platform
        let sc_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        // The swap chain is a set of framebuffers used as (screen) render target.
        let (sc_desc, swap_chain) = Self::setup_swap_chain(&device, size, &surface, sc_format);
        // Then a pipeline is set up: it is a description of a set of steps that will
        // transform data into an image; to do so, a description of how data are layed out
        // is necessary: that's the bind group layout.
        let (bind_group_layout, render_pipeline) = Self::setup_pipeline(&device, sc_format)?;
        // Then we allocate the actual data: a texture (memory region) and a sampler (a
        // resource that allows to retrieve values from a texture).
        let (texture_extent, texture, sampler) = Self::setup_texture(&device, img_w, img_h);
        let texture_view = texture.create_default_view();
        // The bind group is then created: this is an actual set of resources (texture and
        // sampler), that are addressable according to their bind group layout. The
        // driver will take care of optimizing the placement of these resources according
        // to the usage described by the layout. The pipeline will use the bind group
        // layout to access the bind group (resources) efficiently.
        let bind_group =
            Self::setup_bind_group(&device, &bind_group_layout, &texture_view, &sampler);

        Ok(Renderer {
            img_w,
            surface,
            adapter,
            device,
            queue,
            sc_format,
            sc_desc,
            swap_chain,
            render_pipeline,
            texture,
            texture_extent,
            bind_group,
        })
    }

    /// Creates the swap chain, that is framebuffers required to show things on screen
    fn setup_swap_chain(
        device: &wgpu::Device,
        size: winit::dpi::PhysicalSize<u32>,
        surface: &wgpu::Surface,
        sc_format: wgpu::TextureFormat,
    ) -> (wgpu::SwapChainDescriptor, wgpu::SwapChain) {
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: sc_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox, // low latency v-synced display
        };

        let swap_chain = device.create_swap_chain(surface, &sc_desc);

        (sc_desc, swap_chain)
    }

    /// Sync function to retrieve adater, device and default device queue
    fn setup_adapter_device_queue(
        surface: &wgpu::Surface,
    ) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), Error> {
        let adapter = executor::block_on(async {
            wgpu::Adapter::request(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                },
                wgpu::BackendBit::PRIMARY,
            )
            .await
            .ok_or(Error::NoAdapter)
        })?;

        // Request device and default queue
        let (device, queue) = executor::block_on(async {
            adapter
                .request_device( // Panics if no devices are found
                    &wgpu::DeviceDescriptor {
                        extensions: wgpu::Extensions {
                            anisotropic_filtering: false,
                        },
                        limits: wgpu::Limits::default(),
                    },
                )
                .await
        });
        Ok((adapter, device, queue))
    }

    /// Creates texture buffer on the GPU and a sampler to read it from shaders
    fn setup_texture(
        device: &wgpu::Device,
        width: usize,
        height: usize,
    ) -> (wgpu::Extent3d, wgpu::Texture, wgpu::Sampler) {
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
        (texture_extent, texture, sampler)
    }

    /// Creates a bind group for texture access
    fn setup_bind_group(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        // Create bind group to access texture
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: None,
        })
    }

    /// Creates a pipeline that displays the image on a quad texture
    fn setup_pipeline(
        device: &wgpu::Device,
        sc_format: wgpu::TextureFormat,
    ) -> Result<(wgpu::BindGroupLayout, wgpu::RenderPipeline), Error> {
        // Shader modules are just sampling textures and drawing to a fixed quad
        let vs = include_bytes!(concat!(env!("OUT_DIR"), "/shader.vert.spv"));
        let vs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).map_err(Error::ShaderLoad)?);

        let fs = include_bytes!(concat!(env!("OUT_DIR"), "/shader.frag.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).map_err(Error::ShaderLoad)?);

        // Bind 0 is used for texture and 1 is used for sampler
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    // Accessing texture in frag
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    // Sampling texture in frag
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

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_format,
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
        Ok((bind_group_layout, pipeline))
    }

    /// Handles resize by recreating the swap chain (framebuffers)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.sc_desc.width = width;
        self.sc_desc.height = height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    /// Presents BGRA8 bgra8_data sending it to texture buffer and rendering the frame
    pub fn render(&mut self, bgra8_data: &[u8]) -> Result<(), Error> {
        let temp_buff = self
            .device
            .create_buffer_with_data(bgra8_data, wgpu::BufferUsage::COPY_SRC);

        let frame = self
            .swap_chain
            .get_next_texture()
            .map_err(Error::SwapChainTimeout)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buff,
                offset: 0,
                bytes_per_row: 4 * self.img_w as u32,
                rows_per_image: 0,
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            self.texture_extent,
        );
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::BLACK,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }
        self.queue.submit(&[encoder.finish()]);
        Ok(())
    }
}
