
use crate::{
    pipelines::{
        clear::ClearPipeline,
        solid::{
            SolidPipeline,
            DrawCallSolid,
            prep_draw_solid_call,
        },
    },
    std140::{
        Std140,
        std140_struct,
        pad,
    },
    vertex::{
        VertexStruct,
        vertex_struct,
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
use glyph_brush::{
    self as gb,
    ab_glyph::FontArc,
    GlyphBrush,
    GlyphBrushBuilder,
    GlyphPositioner,
};

mod pipelines;
mod std140;
mod shader;
mod vertex;

/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    uniform_buffer_state: Option<UniformBufferState>,
    canvas2d_uniform_bind_group_layout: BindGroupLayout,
    clear_pipeline: ClearPipeline,
    solid_pipeline: SolidPipeline,

    image_pipeline: RenderPipeline,
    image_texture_bind_group_layout: BindGroupLayout,
    image_sampler: Sampler,

    text_pipeline: RenderPipeline,    
    glyph_brush: GlyphBrush<TextQuad, GlyphExtra>,
    fonts: Vec<FontArc>,
    glyph_cache_sampler: Sampler,
    glyph_cache_texture: Texture,
    glyph_cache_bind_group_layout: BindGroupLayout,
    glyph_cache_bind_group: BindGroup,
    text_vertex_state: Option<TextVertexState>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct GlyphExtra {
    color: Rgba<u8>,
    draw_text_call_index: usize,
}


#[derive(Debug, Clone)]
struct TextQuad {
    src: (Vec2<f32>, Extent2<f32>),
    dst: (Vec2<f32>, Extent2<f32>), // TODO are either of these extraneous?
    color: Rgba<u8>,
    draw_text_call_index: usize,
}

#[derive(Debug, Copy, Clone)]
struct TextVertex {
    pos: Vec2<f32>,
    tex: Vec2<f32>,
    color: Rgba<u8>,
}

vertex_struct! {
    TextVertex {
        (pos:   Vec2<f32>) (layout(location=0) in vec2),
        (tex:   Vec2<f32>) (layout(location=1) in vec2),
        (color: Rgba<u8> ) (layout(location=2) in vec4),
    }
}

const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    canvas2d_uniform_bind_group: BindGroup,
}

struct TextVertexState {
    text_vertex_buffer: Buffer,
    text_vertex_buffer_len: usize,
    num_text_vertices: usize,
    draw_text_call_ranges: Vec<Option<(usize, usize)>>,
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

/// Create a glyph cache texture with the given size, then create a glyph cache
/// bind group using that texture and a pre-created sampler and layout.
///
/// This logic is shared between the initial construction of the glyph cache,
/// and the resizing of the glyph cache.
fn finish_glyph_cache_creation(
    device: &Device,
    glyph_cache_sampler: &Sampler,
    glyph_cache_bind_group_layout: &BindGroupLayout,
    size: Extent2<u32>,
) -> (Texture, BindGroup) {
    let glyph_cache_texture = device
        .create_texture(&TextureDescriptor {
            label: Some("glyph cache texture"),
            size: Extent3d {
                width: size.w,
                height: size.h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        });
    let glyph_cache_texture_view = glyph_cache_texture
        .create_view(&TextureViewDescriptor {
            label: Some("glyph cache texture view"),
            ..Default::default()
        });
    let glyph_cache_bind_group = device
        .create_bind_group(&BindGroupDescriptor {
            label: Some("glyph cache bind group"),
            layout: glyph_cache_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&glyph_cache_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(glyph_cache_sampler),
                },
            ],
        });
    (glyph_cache_texture, glyph_cache_bind_group)
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

        // create the layout for the standard bind group all canvas2d shaders
        // use for canvas2d transformations
        let canvas2d_uniform_bind_group_layout = device
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

        // create the clear pipeline
        trace!("creating clear pipeline");
        let clear_pipeline = ClearPipeline::new(&device).await?;

        // create the solid pipeline
        trace!("creating solid pipeline");
        let solid_pipeline = SolidPipeline::new(
            &device,
            &canvas2d_uniform_bind_group_layout,
        ).await?;

        // create the image pipeline
        trace!("creating image pipeline");
        let image_vs_module = device
            .create_shader_module(&load_shader("image.vert").await?);
        let image_fs_module = device
            .create_shader_module(&load_shader("image.frag").await?);
        let image_texture_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("image texture bind group layout"),
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
                    &canvas2d_uniform_bind_group_layout,
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
                            format: SWAPCHAIN_FORMAT,
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

        // create the text pipeline
        trace!("creating text pipeline");
        let text_vs_module = device
            .create_shader_module(&load_shader("text.vert").await?);
        let text_fs_module = device
            .create_shader_module(&load_shader("text.frag").await?);
        let glyph_brush = GlyphBrushBuilder::using_fonts::<FontArc>(Vec::new())
            .build();
        let glyph_cache_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("glyph cache sampler"),
                ..Default::default()
            });
        let glyph_cache_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("text texture bind group layout"),
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
        let (glyph_cache_texture, glyph_cache_bind_group) = finish_glyph_cache_creation(
            &device,
            &glyph_cache_sampler,
            &glyph_cache_bind_group_layout,
            glyph_brush.texture_dimensions().into(),
        );
        let text_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("text pipeline layout"),
                bind_group_layouts: &[
                    &canvas2d_uniform_bind_group_layout, // TODO rename this
                    &glyph_cache_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let text_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("text pipeline"),
                layout: Some(&text_pipeline_layout),
                vertex: VertexState {
                    module: &text_vs_module,
                    entry_point: "main",
                    buffers: &[
                        VertexBufferLayout {
                            array_stride: TextVertex::SIZE as u64,
                            step_mode: VertexStepMode::Vertex,
                            attributes: TextVertex::ATTRIBUTES,
                        },
                    ],
                },
                fragment: Some(FragmentState {
                    module: &text_fs_module,
                    entry_point: "main",
                    targets: &[
                        ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
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

        // set up the swapchain
        trace!("configuring swapchain");
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
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
            canvas2d_uniform_bind_group_layout,
            clear_pipeline,
            solid_pipeline,

            image_pipeline,
            image_texture_bind_group_layout,
            image_sampler,
            
            text_pipeline,
            glyph_brush,
            fonts: Vec::new(),
            glyph_cache_sampler,
            glyph_cache_texture,
            glyph_cache_bind_group_layout,
            glyph_cache_bind_group,
            text_vertex_state: None,
        })
    }

    /// Get the current surface physical size.
    pub fn size(&self) -> Extent2<u32> {
        Extent2::new(self.config.width, self.config.height)
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
        let mut canvas_out_vars = Canvas2dOutVars::default();
        let uniform_offset_align = self.device
            .limits()
            .min_uniform_buffer_offset_alignment as usize;
        f(Canvas2d {
            renderer: self,
            out_vars: &mut canvas_out_vars,

            uniform_offset_align,

            transform: Canvas2dTransform::identity(),
        });

        // process the queued glyph brush data
        // update the glyph cache texture as necessary
        // update the text vertex buffer as necessary

        /// Convert an `gb_glyph::Rect` to a (start, extent) tuple.
        fn rect_to_src_extent(rect: gb::ab_glyph::Rect) -> (Vec2<f32>, Extent2<f32>) {
            (
                Vec2::new(rect.min.x, rect.min.y),
                Extent2::new(rect.max.x, rect.max.y) - Extent2::new(rect.min.x, rect.min.y),
            )
        }

        // loop until glyph cache texture is large enough
        for attempt in 0.. {
            assert!(attempt < 100, "glyph cache update loop not breaking");
            let result = self.glyph_brush
                .process_queued(
                    |rect, unpadded_data| {
                        // callback for updating the glyph cache texture

                        // pad data
                        let unpadded_bytes_per_row = rect.width();
                        let padded_bytes_per_row =
                            if unpadded_bytes_per_row % COPY_BYTES_PER_ROW_ALIGNMENT == 0 {
                                unpadded_bytes_per_row
                            } else {
                                unpadded_bytes_per_row - (unpadded_bytes_per_row % COPY_BYTES_PER_ROW_ALIGNMENT) + COPY_BYTES_PER_ROW_ALIGNMENT
                            };

                        let num_rows = rect.height();

                        let mut padded_data = Vec::new();
                        for row in 0..num_rows {
                            let start = row * unpadded_bytes_per_row;
                            let end = row * unpadded_bytes_per_row + unpadded_bytes_per_row;
                            padded_data.extend(unpadded_data[start as usize..end as usize].iter().copied());
                            for _ in 0..padded_bytes_per_row - unpadded_bytes_per_row {
                                padded_data.push(0);
                            }
                        }

                        // write the data to the texture
                        self.queue
                            .write_texture(
                                self.glyph_cache_texture.as_image_copy(),
                                &padded_data,
                                ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(padded_bytes_per_row.try_into().unwrap()),
                                    rows_per_image: Some(num_rows.try_into().unwrap()),
                                },
                                Extent3d {
                                    width: rect.width(),
                                    height: rect.height(),
                                    depth_or_array_layers: 1,
                                },
                            );

                        padded_data.clear();
                    },
                    // callback to convert glyph_brush vertex to text quad
                    |glyph_vertex| TextQuad {
                        src: rect_to_src_extent(glyph_vertex.tex_coords),
                        dst: rect_to_src_extent(glyph_vertex.pixel_coords),
                        color: glyph_vertex.extra.color,
                        draw_text_call_index: glyph_vertex.extra.draw_text_call_index,
                    },
                );
            match result {
                Ok(gb::BrushAction::Draw(mut quads)) => {
                    // successfully produced a list of text quads to draw
                    // so update the text vertex buffer accordingly

                    // sort the quads by draw_text_call_index
                    quads.sort_by_key(|vert| vert.draw_text_call_index);

                    // convert each quad into 6 vertices
                    let vertex_vec = quads
                        .iter()
                        .flat_map(|quad| (0..6)
                            .map(|i| {
                                let corner = [0, 2, 1, 0, 3, 2][i];
                                let (a, b) = [
                                    (0, 0),
                                    (1, 0),
                                    (1, 1),
                                    (0, 1),
                                ][corner];
                                TextVertex {
                                    pos: (quad.dst.0 + Vec2::new(
                                        [0.0, quad.dst.1.w][a],
                                        [0.0, quad.dst.1.h][b],
                                    )),
                                    tex: quad.src.0 + Vec2::new(
                                        [0.0, quad.src.1.w][a],
                                        [0.0, quad.src.1.h][b],
                                    ),
                                    color: quad.color,
                                }
                            }))
                        .collect::<Vec<_>>();

                    // compute the array that maps from draw_text call index to
                    // range of vertices
                    let mut ranges = Vec::new();
                    let mut quad_idx = 0;
                    for draw_idx in 0..canvas_out_vars.next_draw_text_call_index {
                        // simply multiply start and end by 6 to convert from
                        // quad index to vertex index
                        let start = quad_idx * 6;
                        while 
                            quad_idx < quads.len()
                            && quads[quad_idx].draw_text_call_index == draw_idx
                        {
                            quad_idx += 1;
                        }
                        debug_assert!(
                            quad_idx == quads.len()
                            || quads[quad_idx].draw_text_call_index > draw_idx,
                        );
                        let end = quad_idx * 6;

                        if end > start {
                            ranges.push(Some((start, end)));
                        } else {
                            ranges.push(None);
                        }
                    }

                    // convert the vertices into bytes
                    let mut vertex_bytes = Vec::new();
                    for vertex in &vertex_vec {
                        vertex.write(&mut vertex_bytes);
                    }

                    // write to the vertex buffer, allocating/reallocating if necessary
                    if !vertex_bytes.is_empty() {
                        let dst = self
                            .text_vertex_state
                            .as_mut()
                            .filter(|state| state.text_vertex_buffer_len >= vertex_bytes.len());
                        if let Some(dst) = dst {
                            // vertex buffer exists and is big enough
                            // write to buffer
                            self.queue.write_buffer(&dst.text_vertex_buffer, 0, &vertex_bytes);
                            // update other data
                            dst.text_vertex_buffer_len = vertex_bytes.len();
                            dst.num_text_vertices = vertex_vec.len();
                            dst.draw_text_call_ranges = ranges;
                        } else {
                            // cannot reuse existing vertex buffer
                            // create new one
                            let text_vertex_buffer = self.device
                                .create_buffer_init(&BufferInitDescriptor {
                                    label: Some("text vertex buffer"),
                                    contents: &vertex_bytes,
                                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                                });
                            self.text_vertex_state = Some(TextVertexState {
                                text_vertex_buffer,
                                text_vertex_buffer_len: vertex_bytes.len(),
                                num_text_vertices: vertex_vec.len(),
                                draw_text_call_ranges: ranges,
                            });
                        }
                    }

                    // break the loop
                    break;
                },
                Ok(gb::BrushAction::ReDraw) => break, // reuse existing vertex buffer
                Err(gb::BrushError::TextureTooSmall {
                    suggested: (w, h),
                }) => {
                    // increase glyph cache texture size and try again
                    trace!("increasing glyph cache texture size");
                    self.glyph_brush.resize_texture(w, h);
                    let (glyph_cache_texture, glyph_cache_bind_group) = finish_glyph_cache_creation(
                        &self.device,
                        &self.glyph_cache_sampler,
                        &self.glyph_cache_bind_group_layout,
                        Extent2::new(w, h),
                    );
                    self.glyph_cache_texture = glyph_cache_texture;
                    self.glyph_cache_bind_group = glyph_cache_bind_group;
                },
            };
        }

        // write uniform data to uniform buffer
        trace!("writing uniform data");
        if !canvas_out_vars.uniform_data_buf.is_empty() {
            let dst = self
                .uniform_buffer_state
                .as_ref()
                .filter(|state| state.uniform_buffer_len >= canvas_out_vars.uniform_data_buf.len());
            if let Some(dst) = dst {
                // buffer already exists and is big enough to hold data
                trace!("re-using uniform buffer");
                self.queue.write_buffer(&dst.uniform_buffer, 0, &canvas_out_vars.uniform_data_buf);
            } else {
                // buffer doesn't exist or isn't big enough
                trace!("creating new uniform buffer");
                let uniform_buffer = self.device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("uniform buffer"),
                        contents: &canvas_out_vars.uniform_data_buf,
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });
                let canvas2d_uniform_bind_group = self.device
                    .create_bind_group(&BindGroupDescriptor {
                        label: Some("solid uniform bind group"),
                        layout: &self.canvas2d_uniform_bind_group_layout,
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
                    uniform_buffer_len: canvas_out_vars.uniform_data_buf.len(),
                    canvas2d_uniform_bind_group
                });
            }
        }

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
            self.clear_pipeline.clear_screen(&mut pass);

            // make draw calls
            trace!("making draw calls");
            for draw_call in canvas_out_vars.draw_calls {
                match draw_call {
                    Canvas2dDrawCall::Solid(call) => self.solid_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                        ),
                    Canvas2dDrawCall::Image {
                        uniform_offset,
                        image_index,
                    } => {
                        let uniform_buffer_state = self
                            .uniform_buffer_state
                            .as_ref()
                            .unwrap();

                        let image = &canvas_out_vars.image_array[image_index];
                        pass.set_pipeline(&self.image_pipeline);
                        pass.set_bind_group(
                            0,
                            &uniform_buffer_state.canvas2d_uniform_bind_group,
                            &[uniform_offset as u32],
                        );
                        pass.set_bind_group(
                            1,
                            &image.0.texture_bind_group,
                            &[],
                        );
                        pass.draw(0..6, 0..1);
                    },
                    Canvas2dDrawCall::Text {
                        uniform_offset,
                        draw_text_call_index,
                    } => {
                        let text_vertex_state = self
                            .text_vertex_state
                            .as_ref()
                            .unwrap();
                        let uniform_buffer_state = self
                            .uniform_buffer_state
                            .as_ref()
                            .unwrap();

                        let vertex_range = text_vertex_state
                            .draw_text_call_ranges[draw_text_call_index];
                        if let Some((start, end)) = vertex_range {
                            pass.set_pipeline(&self.text_pipeline);
                            pass.set_vertex_buffer(
                                0,
                                text_vertex_state.text_vertex_buffer.slice(..),
                            );
                            pass.set_bind_group(
                                0,
                                &uniform_buffer_state.canvas2d_uniform_bind_group,
                                &[uniform_offset as u32],
                            );
                            pass.set_bind_group(
                                1,
                                &self.glyph_cache_bind_group,
                                &[],
                            );
                            pass.draw(
                                start as u32..end as u32,
                                0..1,
                            );
                        }
                    },
                };
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
    ///
    /// Just reads the file with tokio then passes it to `self.load_image`.
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

    /// Read an OTF / TTF / etc font from a file and load it onto the renderer.
    ///
    /// Just reads the file with tokio then passes it to `self.load_font`.
    pub async fn load_font_file(&mut self, path: impl AsRef<Path>) -> Result<FontId> {
        let file_data = fs::read(path).await?;
        self.load_font(&file_data)
    }

    /// Load a font onto the renderer from OTF / TTF / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font(&mut self, file_data: &[u8]) -> Result<FontId> {
        // load font
        let font = FontArc::try_from_vec(file_data.into())?;

        // load into glyph brush
        let font_idx = self.glyph_brush.add_font(font.clone()).0;

        // add to vec
        self.fonts.push(font);
        
        // done
        Ok(FontId(font_idx))
    }

    /// Pre-compute the layout for a text block.
    pub fn lay_out_text(&self, text_block: &TextBlock) -> LayedOutTextBlock {
        // convert to glyph_brush types
        let layout = text_block.to_layout();
        let section_geometry = text_block.to_section_geometry();

        // have glyph_brush lay it out
        let section_glyphs = layout
            .calculate_glyphs(
                &self.fonts,
                &section_geometry,
                text_block.as_sections(),
            );
        let bounds = layout.bounds_rect(&section_geometry);

        // re-associate the color data
        let glyphs = section_glyphs
            .into_iter()
            .map(|section_glyph| LayedOutGlyph {
                color: text_block
                    .spans[section_glyph.section_index]
                    .color,
                section_glyph,
            })
            .collect();

        // done
        LayedOutTextBlock {
            glyphs,
            bounds,
        }
    }
}


/// Target for drawing 2 dimensionally onto. Each successive draw call is
/// blended over the previously drawn data.
pub struct Canvas2d<'a> {
    renderer: &'a mut Renderer,
    out_vars: &'a mut Canvas2dOutVars,
    
    // alignment for all offsets into uniform_data
    uniform_offset_align: usize,

    transform: Canvas2dTransform,
}

#[derive(Default)]
struct Canvas2dOutVars {
    uniform_data_buf: Vec<u8>,
    image_array: Vec<GpuImage>,
    draw_calls: Vec<Canvas2dDrawCall>,
    next_draw_text_call_index: usize,
}

enum Canvas2dDrawCall {
    Solid(DrawCallSolid),
    Image {
        uniform_offset: usize,
        image_index: usize,
    },
    Text {
        uniform_offset: usize,
        draw_text_call_index: usize,
    },
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
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
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
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_scale(s),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain x value before drawing to self.
    pub fn with_clip_min_x<'b>(&'b mut self, min_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_min_x(min_x),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain x value before drawing to self.
    pub fn with_clip_max_x<'b>(&'b mut self, max_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_max_x(max_x),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain y value before drawing to self.
    pub fn with_clip_min_y<'b>(&'b mut self, min_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_min_y(min_y),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain y value before drawing to self.
    pub fn with_clip_max_y<'b>(&'b mut self, max_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_max_y(max_y),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, multiplies all colors by the
    /// given color value before drawing to self.
    pub fn with_color<'b>(&'b mut self, c: impl Into<Rgba<u8>>) -> Canvas2d<'b> {
        let c = c.into().map(|b| b as f32 / 0xFF as f32);
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_color(c),
            ..*self
        }
    }

    /// Push canvas2d uniform data onto `self.uniform_data_buf` based on
    /// current transform, and return the offset.
    fn push_uniform_data(&mut self) -> usize {
        let uniform_data = DrawSolidUniformData {
            transform: self.transform.affine,
            color: self.transform.color,
            clip_min_x: self.transform.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.transform.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.transform.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.transform.clip_max_y.unwrap_or(f32::INFINITY),
        };
        pad(&mut self.out_vars.uniform_data_buf, self.uniform_offset_align);
        // TODO make padding logic less jankily connected
        uniform_data.pad_write(&mut self.out_vars.uniform_data_buf)
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        prep_draw_solid_call(self);
    }

    /// Draw the given image from <0, 0> to <1, 1>.
    pub fn draw_image(&mut self, image: &GpuImage) {
        // push uniform data
        let uniform_offset = self.push_uniform_data();

        // push image
        let image_index = self.out_vars.image_array.len();
        self.out_vars.image_array.push(image.clone());

        // push draw call
        self.out_vars.draw_calls.push(Canvas2dDrawCall::Image {
            uniform_offset,
            image_index,
        });
    }

    /// Draw the given text block with <0, 0> as the top-left corner.
    pub fn draw_text(&mut self, text_block: &LayedOutTextBlock) {
        // push uniform data
        let uniform_offset = self.push_uniform_data();

        // get and increment the draw text call index to identify this batch of
        // glyphs
        let draw_text_call_index = self.out_vars.next_draw_text_call_index;
        self.out_vars.next_draw_text_call_index += 1;

        // extract the section glyphs from the text block
        let section_glyphs = text_block
            .glyphs
            .iter()
            .map(|glyph| glyph.section_glyph.clone())
            .collect();

        // produce the matching extra data for the text block
        let extra = text_block
            .glyphs
            .iter()
            .map(|glyph| GlyphExtra {
                draw_text_call_index,
                color: glyph.color,
            })
            .collect();

        // queue to the glyph brush
        self.renderer.glyph_brush
            .queue_pre_positioned(
                section_glyphs,
                extra,
                text_block.bounds,
            );

        // push draw call
        self.out_vars.draw_calls.push(Canvas2dDrawCall::Text {
            uniform_offset,
            draw_text_call_index,
        });
    }
}


/// Block of text with specification of how to display it.
#[derive(Debug, Copy, Clone)]
pub struct TextBlock<'a> {
    /// The spans of text to flow together.
    pub spans: &'a [TextSpan<'a>],
    /// Specification of horizontal align/wrap behavior.
    pub horizontal_align: HorizontalAlign,
    /// Specification of vertical align/wrap behavior.
    pub vertical_align: VerticalAlign,
}

/// Specification of text horizontal align/wrap behavior.
#[derive(Debug, Copy, Clone)]
pub enum HorizontalAlign {
    /// Left-justify the text.
    Left {
        /// The block width to wrap text at. If `None`, text will just continue
        /// rightwards forever.
        width: Option<f32>,
    },
    /// Center-justify the text.
    Center {
        /// The width of the block. Text will be centered between 0 and `width`
        /// and wrap within that range.
        width: f32,
    },
    /// Right-justify the text.
    Right {
        /// The width of the block. Text will be pressed up against `width`
        /// and wrap between 0 and `width`.
        width: f32,
    },
}

/// Specification of text vertical align/wrap behavior.
#[derive(Debug, Copy, Clone)]
pub enum VerticalAlign {
    /// Press the text up against the top of the block (0).
    Top,
    /// Vertically center the text between 0 and `height`.
    Center { height: f32 },
    /// Press the text down against the bottom of the block (`height`).
    Bottom { height: f32 },
}

/// Index for a font loaded into a `Renderer`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FontId(pub usize);

/// Span of text with specification of how to display it.
#[derive(Debug, Copy, Clone)]
pub struct TextSpan<'a> {
    /// The actual string of text.
    pub text: &'a str,
    /// Which font to use.
    pub font_id: FontId,
    /// Text height units.
    pub font_size: f32,
    /// Text color.
    pub color: Rgba<u8>,
}

/// Convert from font points (1/72 in) to logical pixels (1/92 in).
pub fn pt_to_px(pt: f32) -> f32 {
    pt * 4.0 / 3.0
}

impl<'a> TextBlock<'a> {
    /// Produce a corresponding glyph_brush `Layout`.
    fn to_layout(&self) -> gb::Layout<gb::BuiltInLineBreaker> {
        let gb_h_align = match self.horizontal_align {
            HorizontalAlign::Left { .. } => gb::HorizontalAlign::Left,
            HorizontalAlign::Center { .. } => gb::HorizontalAlign::Center,
            HorizontalAlign::Right { .. } => gb::HorizontalAlign::Right,
        };
        let gb_v_align = match self.vertical_align {
            VerticalAlign::Top => gb::VerticalAlign::Top,
            VerticalAlign::Center { .. } => gb::VerticalAlign::Center,
            VerticalAlign::Bottom { .. } => gb::VerticalAlign::Bottom,
        };
        let single_line = matches!(
            self.horizontal_align,
            HorizontalAlign::Left { width: None },
        );
        if single_line {
            gb::Layout::SingleLine {
                line_breaker: gb::BuiltInLineBreaker::UnicodeLineBreaker,
                h_align: gb_h_align,
                v_align: gb_v_align,
            }
        } else {
            gb::Layout::Wrap {
                line_breaker: gb::BuiltInLineBreaker::UnicodeLineBreaker,
                h_align: gb_h_align,
                v_align: gb_v_align,
            }
        }
    }

    /// Produce a corresponding glyph_brush `SectionGeometry`.
    fn to_section_geometry(&self) -> gb::SectionGeometry {
        let width = match self.horizontal_align {
            HorizontalAlign::Left { width } => width,
            HorizontalAlign::Center { width } => Some(width),
            HorizontalAlign::Right { width } => Some(width),
        };
        let height = match self.vertical_align {
            VerticalAlign::Top => None,
            VerticalAlign::Center { height } => Some(height),
            VerticalAlign::Bottom { height } => Some(height),
        };
        gb::SectionGeometry {
            screen_position: (0.0, 0.0),
            bounds: (
                width.unwrap_or(f32::INFINITY),
                height.unwrap_or(f32::INFINITY),
            ),
        }
    }

    /// View as a slice of glyph_brush `ToSectionText` impls.
    fn as_sections(&self) -> &[impl gb::ToSectionText + 'a] {
        &self.spans
    }
}

impl<'a> gb::ToSectionText for TextSpan<'a> {
    fn to_section_text(&self) -> gb::SectionText {
        gb::SectionText {
            text: self.text,
            scale: gb::ab_glyph::PxScale {
                x: self.font_size,
                y: self.font_size,
            },
            font_id: gb::FontId(self.font_id.0),
        }
    }
}

/// Block of text with a pre-computed plan for how to lay it out.
pub struct LayedOutTextBlock {
    glyphs: Vec<LayedOutGlyph>,
    bounds: gb::ab_glyph::Rect,
}

struct LayedOutGlyph {
    section_glyph: gb::SectionGlyph,
    color: Rgba<u8>,
}

impl LayedOutTextBlock {
    /// Size taken by this text block in pixels.
    pub fn size(&self) -> Extent2<f32> {
        Extent2::new(self.bounds.width(), self.bounds.height())
    }
}
