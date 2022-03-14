
use crate::{
    std140::{
        Std140,
        std140_struct,
        pad,
    },
    shader::load_shader,
};
use std::{
    path::Path,
    sync::Arc,
};
use anyhow::Result;
use tracing::*;
use winit_main::reexports::{
    window::Window,
    dpi::PhysicalSize,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,
    },
};
use vek::*;
use tokio::fs;


mod std140;
mod shader;


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    uniform_buffer_state: Option<UniformBufferState>,

    clear_pipeline: RenderPipeline,
    
    solid_pipeline: RenderPipeline,
    solid_uniform_bind_group_layout: BindGroupLayout,

    image_pipeline: RenderPipeline,
    image_texture_bind_group_layout: BindGroupLayout,
    image_sampler: Sampler,
}

struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    solid_uniform_bind_group: BindGroup,
}

/// 2D RGBA image loaded into a GPU texture.
///
/// Internally reference-counted.
#[derive(Clone)]
pub struct GpuImage(Arc<GpuImageInner>);

struct GpuImageInner {
    size: Extent2<u32>,
    texture_bind_group: BindGroup,
}

impl GpuImage {
    /// Get image size in pixels.
    pub fn size(&self) -> Extent2<u32> {
        self.0.size
    }
}

impl Renderer {
    /// Create a new renderer on a given window.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        trace!("beginning initializing renderer");

        // create the instance, surface, and adapter
        trace!("creating instance");
        let size = window.inner_size();
        let instance = Instance::new(Backends::PRIMARY);
        trace!("creating surface");
        let surface = unsafe { instance.create_surface(&*window) };

        trace!("creating adapter");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::Error::msg("failed to find an appropriate adapter"))?;

        // create the device and queue
        trace!("creating device and queue");
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits::default(),
                },
                None,
            )
            .await?;

        let swapchain_format = TextureFormat::Bgra8Unorm;

        // create the clear pipeline
        trace!("creating clear pipeline");
        let clear_vs_module = device
            .create_shader_module(&load_shader("clear.vert").await?);
        let clear_fs_module = device
            .create_shader_module(&load_shader("clear.frag").await?);
        let clear_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("clear pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let clear_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("clear pipeline"),
                layout: Some(&clear_pipeline_layout),
                vertex: VertexState {
                    module: &clear_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &clear_fs_module,
                    entry_point: "main",
                    targets: &[swapchain_format.into()],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        // create the solid pipeline
        trace!("creating solid pipeline");
        let solid_vs_module = device
            .create_shader_module(&load_shader("solid.vert").await?);
        let solid_fs_module = device
            .create_shader_module(&load_shader("solid.frag").await?);
        let solid_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("solid uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some((DrawSolidUniformData::SIZE as u64).try_into().unwrap()),
                        },
                        count: None,
                    },
                ],
            });
        let solid_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("solid pipeline layout"),
                bind_group_layouts: &[
                    &solid_uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let solid_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("solid pipeline"),
                layout: Some(&solid_pipeline_layout),
                vertex: VertexState {
                    module: &solid_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &solid_fs_module,
                    entry_point: "main",
                    targets: &[
                        ColorTargetState {
                            format: swapchain_format,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        },
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        // create the image pipeline
        trace!("creating image pipeline");
        let image_vs_module = device
            .create_shader_module(&load_shader("image.vert").await?);
        let image_fs_module = device
            .create_shader_module(&load_shader("image.frag").await?);
        // the image pipeline's uniform bind group layout is exactly the same
        // as the solid uniform bind group layout, so literally just use that
        let image_texture_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("image sampler bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: false,
                            },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });
        let image_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("image pipeline layout"),
                bind_group_layouts: &[
                    &solid_uniform_bind_group_layout,
                    &image_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let image_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("image pipeline"),
                layout: Some(&image_pipeline_layout),
                vertex: VertexState {
                    module: &image_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &image_fs_module,
                    entry_point: "main",
                    targets: &[
                        ColorTargetState {
                            format: swapchain_format,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        },
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
        let image_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("image sampler"),
                ..Default::default()
            });

        // set up the swapchain
        trace!("configuring swapchain");
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Mailbox,
        };
        surface.configure(&device, &config);

        // done
        trace!("done initializing renderer");
        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            uniform_buffer_state: None,

            clear_pipeline,

            solid_pipeline,
            solid_uniform_bind_group_layout,

            image_pipeline,
            image_texture_bind_group_layout,
            image_sampler,
        })
    }

    /// Resize the surface, in reponse to a change in window size.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draw a frame. The callback can draw onto the Canvas2d. Then it will be
    /// displayed on the window from <0,0> (top left corner) to <1,1> (bottom
    /// right corner).
    pub fn draw_frame(&mut self, f: impl FnOnce(Canvas2d)) -> Result<()> {
        // acquire frame to draw onto
        trace!("acquiring frame");
        let mut attempts = 0;
        let frame = loop {
            match self.surface.get_current_texture() {
                Ok(frame) => break frame,
                Err(e) => {
                    if attempts < 10 {
                        trace!(error=%e, "get_current_texture error, retrying");
                        attempts += 1;
                        self.surface.configure(&self.device, &self.config);
                    } else {
                        return Err(e.into());
                    }
                }
            }
        };
        if attempts > 0 {
            trace!("successfully recreated swap chain surface");
        }
        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());

        // accumulate draw data from the callback
        trace!("accumulating draw data");
        let mut uniform_data = Vec::new();
        let mut images = Vec::new();
        let mut draw_calls = Vec::new();
        f(Canvas2d {
            uniform_data: &mut uniform_data,
            images: &mut images,
            draw_calls: &mut draw_calls,

            uniform_offset_align: self.device.limits().min_uniform_buffer_offset_alignment as usize,

            transform: Canvas2dTransform::identity(),
        });

        // begin encoder and pass
        trace!("creating encoder and pass");
        let mut encoder = self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: None,
            });
        {
            let mut pass = encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::WHITE),
                                store: true,
                            }
                        }
                    ],
                    depth_stencil_attachment: None,
                });

            // clear the screen
            trace!("clearing screen");
            pass.set_pipeline(&self.clear_pipeline);
            pass.draw(0..1, 0..1);
            

            // write uniform data to uniform buffer
            trace!("writing uniform data");
            if !uniform_data.is_empty() {
                let dst = self
                    .uniform_buffer_state
                    .as_ref()
                    .filter(|state| state.uniform_buffer_len >= uniform_data.len());
                if let Some(dst) = dst {
                    // buffer already exists and is big enough to hold data
                    trace!("re-using uniform buffer");
                    self.queue.write_buffer(&dst.uniform_buffer, 0, &uniform_data);
                } else {
                    // buffer doesn't exist or isn't big enough
                    trace!("creating new uniform buffer");
                    let uniform_buffer = self.device
                        .create_buffer_init(&BufferInitDescriptor {
                            label: Some("uniform buffer"),
                            contents: &uniform_data,
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        });
                    let solid_uniform_bind_group = self.device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("solid uniform bind group"),
                            layout: &self.solid_uniform_bind_group_layout,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::Buffer(BufferBinding {
                                        buffer: &uniform_buffer,
                                        offset: 0,
                                        size: Some((DrawSolidUniformData::SIZE as u64).try_into().unwrap()),
                                    }),
                                },
                            ],
                        });
                    self.uniform_buffer_state = Some(UniformBufferState {
                        uniform_buffer,
                        uniform_buffer_len: uniform_data.len(),
                        solid_uniform_bind_group
                    });
                }
            }

            // make draw calls
            trace!("making draw calls");
            if !draw_calls.is_empty() {
                let uniform_buffer_state = self
                    .uniform_buffer_state
                    .as_ref()
                    .unwrap();

                for draw_call in draw_calls {
                    match draw_call {
                        Canvas2dDrawCall::Solid { uniform_offset } => {
                            pass.set_pipeline(&self.solid_pipeline);
                            pass.set_bind_group(
                                0,
                                &uniform_buffer_state.solid_uniform_bind_group,
                                &[uniform_offset as u32],
                            );
                            pass.draw(0..6, 0..1);
                        },
                        Canvas2dDrawCall::Image {
                            uniform_offset,
                            image_index,
                        } => {
                            let image = &images[image_index];
                            pass.set_pipeline(&self.image_pipeline);
                            pass.set_bind_group(
                                0,
                                &uniform_buffer_state.solid_uniform_bind_group,
                                &[uniform_offset as u32],
                            );
                            pass.set_bind_group(
                                1,
                                &image.0.texture_bind_group,
                                &[],
                            );
                            pass.draw(0..6, 0..1);
                        },
                    };
                }
            }
            
            // finish
            trace!("finishing frame");

            // end scope to drop
        }
        self.queue.submit(Some(encoder.finish()));        
        frame.present();
        Ok(())
    }

    /// Read a PNG / JPG / etc image from a file and load it onto the GPU.
    pub async fn load_image_file(&self, path: impl AsRef<Path>) -> Result<GpuImage> {
        let file_data = fs::read(path).await?;
        self.load_image(&file_data)
    }

    /// Load an image onto the GPU from PNG / JPG / etc file data.
    pub fn load_image(&self, file_data: &[u8]) -> Result<GpuImage> {
        let texture_format = TextureFormat::Rgba8Unorm;

        // load image
        let image = image::load_from_memory(file_data)?
            .into_rgba8();

        // create texture
        let texture = self.device
            .create_texture_with_data(
                &self.queue,
                &TextureDescriptor {
                    label: Some("image texture"),
                    size: Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: texture_format,
                    usage: TextureUsages::TEXTURE_BINDING,
                },
                image.as_raw(),
            );

        // create texture view
        let texture_view = texture
            .create_view(&TextureViewDescriptor {
                label: Some("image texture view"),
                ..Default::default()
            });

        // create bind group
        let texture_bind_group = self.device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("image texture bind group"),
                layout: &self.image_texture_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.image_sampler),
                    },
                ],
            });

        // done
        Ok(GpuImage(Arc::new(GpuImageInner {
            texture_bind_group,
            size: Extent2::new(image.width(), image.height()),
        })))
    }
}


/// Target for drawing 2 dimensionally onto. Each successive draw call is
/// blended over the previously drawn data.
pub struct Canvas2d<'a> {
    uniform_data: &'a mut Vec<u8>,
    images: &'a mut Vec<GpuImage>,
    draw_calls: &'a mut Vec<Canvas2dDrawCall>,

    // alignment for all offsets into uniform_data
    uniform_offset_align: usize,

    transform: Canvas2dTransform,
}

enum Canvas2dDrawCall {
    Solid { uniform_offset: usize },
    Image {
        uniform_offset: usize,
        image_index: usize,
    }
}

/// Accumulated transforms on a `Canvas2d`.
#[derive(Debug, Copy, Clone)]
struct Canvas2dTransform {
    affine: Mat3<f32>,
    color: Rgba<f32>,

    clip_min_x: Option<f32>,
    clip_max_x: Option<f32>,
    clip_min_y: Option<f32>,
    clip_max_y: Option<f32>,
}

impl Canvas2dTransform {
    /// Identity transform.
    fn identity() -> Self {
        Canvas2dTransform {
            affine: Mat3::identity(),
            color: Rgba::white(),
            clip_min_x: None,
            clip_max_x: None,
            clip_min_y: None,
            clip_max_y: None,
        }
    }

    /// Apply translation.
    fn with_translate(self, t: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::translation_2d(t),
            ..self
        }
    }

    /// Apply scaling.
    ///
    /// Assumes no negative scaling, that would break the clipping logic.
    ///
    /// TODO: Figure out a more mathematically robust clipping logic.
    fn with_scale(self, s: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::scaling_3d([s.x, s.y, 1.0]),
            ..self
        }
    }

    /// Apply color multiplication.
    fn with_color(self, c: Rgba<f32>) -> Self {
        Canvas2dTransform {
            color: self.color * c,
            ..self
        }
    }

    /// Apply min-x clipping.
    fn with_clip_min_x(self, min_x: f32) -> Self {
        let min_x = (self.affine * Vec3::new(min_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_min_x: Some(self.clip_min_x
                .map(|x| f32::max(x, min_x))
                .unwrap_or(min_x)),
            ..self
        }
    }

    /// Apply max-x clipping.
    fn with_clip_max_x(self, max_x: f32) -> Self {
        let max_x = (self.affine * Vec3::new(max_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_max_x: Some(self.clip_max_x
                .map(|x| f32::min(x, max_x))
                .unwrap_or(max_x)),
            ..self
        }
    }

    /// Apply min-y clipping.
    fn with_clip_min_y(self, min_y: f32) -> Self {
        let min_y = (self.affine * Vec3::new(0.0, min_y, 1.0)).y;
        Canvas2dTransform {
            clip_min_y: Some(self.clip_min_y
                .map(|x| f32::max(x, min_y))
                .unwrap_or(min_y)),
            ..self
        }
    }

    /// Apply max-y clipping.
    fn with_clip_max_y(self, max_y: f32) -> Self {
        let max_y = (self.affine * Vec3::new(0.0, max_y, 1.0)).y;
        Canvas2dTransform {
            clip_max_y: Some(self.clip_max_y
                .map(|x| f32::min(x, max_y))
                .unwrap_or(max_y)),
            ..self
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct DrawSolidUniformData {
    transform: Mat3<f32>,
    color: Rgba<f32>,
    clip_min_x: f32,
    clip_max_x: f32,
    clip_min_y: f32,
    clip_max_y: f32,
}

std140_struct! {
    DrawSolidUniformData {
        transform: Mat3<f32>,
        color: Rgba<f32>,
        clip_min_x: f32,
        clip_max_x: f32,
        clip_min_y: f32,
        clip_max_y: f32,
    }
}

impl<'a> Canvas2d<'a> {
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// translation.
    pub fn with_translate<'b>(&'b mut self, t: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_translate(t.into()),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// scaling.
    ///
    /// Panics if either axis is negative.
    pub fn with_scale<'b>(&'b mut self, s: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        let s = s.into();
        assert!(s.x >= 0.0, "negative scaling");
        assert!(s.y >= 0.0, "negative scaling");
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_scale(s),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain x value before drawing to self.
    pub fn with_clip_min_x<'b>(&'b mut self, min_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_clip_min_x(min_x),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain x value before drawing to self.
    pub fn with_clip_max_x<'b>(&'b mut self, max_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_clip_max_x(max_x),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain y value before drawing to self.
    pub fn with_clip_min_y<'b>(&'b mut self, min_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_clip_min_y(min_y),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain y value before drawing to self.
    pub fn with_clip_max_y<'b>(&'b mut self, max_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_clip_max_y(max_y),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, multiplies all colors by the
    /// given color value before drawing to self.
    pub fn with_color<'b>(&'b mut self, c: impl Into<Rgba<u8>>) -> Canvas2d<'b> {
        let c = c.into().map(|b| b as f32 / 0xFF as f32);
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_calls: &mut *self.draw_calls,
            images: &mut *self.images,
            transform: self.transform.with_color(c),
            ..*self
        }
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        // push uniform data
        let uniform_data = DrawSolidUniformData {
            transform: self.transform.affine,
            color: self.transform.color,
            clip_min_x: self.transform.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.transform.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.transform.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.transform.clip_max_y.unwrap_or(f32::INFINITY),
        };
        pad(self.uniform_data, self.uniform_offset_align);
        let uniform_offset = uniform_data.pad_write(self.uniform_data);

        // push draw call
        self.draw_calls.push(Canvas2dDrawCall::Solid { uniform_offset });
    }

    /// Draw the given image from <0, 0> to <1, 1>.
    pub fn draw_image(&mut self, image: &GpuImage) {
        // push uniform data
        // TODO: dedup
        let uniform_data = DrawSolidUniformData {
            transform: self.transform.affine,
            color: self.transform.color,
            clip_min_x: self.transform.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.transform.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.transform.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.transform.clip_max_y.unwrap_or(f32::INFINITY),
        };
        pad(self.uniform_data, self.uniform_offset_align);
        let uniform_offset = uniform_data.pad_write(self.uniform_data);

        // push image
        let image_index = self.images.len();
        self.images.push(image.clone());

        // push draw call
        self.draw_calls.push(Canvas2dDrawCall::Image {
            uniform_offset,
            image_index,
        });
    }
}
