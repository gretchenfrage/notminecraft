//! 2D pipeline for drawing text.

use crate::{
    SWAPCHAIN_FORMAT,
    Canvas2dTarget,
    Canvas2dDrawCall,
    UniformBufferState,
    shader::load_shader,
    vertex::{
        VertexStruct,
        vertex_struct,
    },
    transform2d::Canvas2dTransform,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,
    },
};
use anyhow::Result;
use glyph_brush::{
    self as gb,
    ab_glyph::FontArc,
    GlyphBrush,
    GlyphBrushBuilder,
    GlyphPositioner,
};
use vek::*;
use tracing::*;


pub struct TextPipeline {
    text_pipeline: RenderPipeline,    
    glyph_brush: GlyphBrush<TextQuad, GlyphExtra>,
    fonts: Vec<FontArc>,
    glyph_cache_sampler: Sampler,
    glyph_cache_texture: Texture,
    glyph_cache_bind_group_layout: BindGroupLayout,
    glyph_cache_bind_group: BindGroup,
    text_vertex_state: Option<TextVertexState>,
}

struct TextVertexState {
    text_vertex_buffer: Buffer,
    text_vertex_buffer_len: usize,
    num_text_vertices: usize,
    draw_text_call_ranges: Vec<Option<(usize, usize)>>,
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

pub struct DrawCallText {
    uniform_offset: usize,
    draw_text_call_index: usize,
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

impl TextPipeline {
    pub(crate) async fn new(
        device: &Device,
        canvas2d_uniform_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> {
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
                    &canvas2d_uniform_bind_group_layout,
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

        Ok(TextPipeline {
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

    pub(crate) fn prep_draw_text_call(
        &mut self,
        canvas_target: &mut Canvas2dTarget,
        canvas_transform: &Canvas2dTransform,
        text_block: &LayedOutTextBlock,
    ) {
        // push uniform data
        let uniform_offset = canvas_target.push_uniform_data(&canvas_transform);

        // get and increment the draw text call index to identify this batch of
        // glyphs
        let draw_text_call_index = canvas_target.next_draw_text_call_index;
        canvas_target.next_draw_text_call_index += 1;

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
        self.glyph_brush
            .queue_pre_positioned(
                section_glyphs,
                extra,
                text_block.bounds,
            );

        // push draw call
        let call = DrawCallText {
            uniform_offset,
            draw_text_call_index,
        };
        canvas_target.draw_calls.push(Canvas2dDrawCall::Text(call));
    }

    pub(crate) fn pre_render(
        &mut self,
        device: &Device,
        queue: &Queue,
        canvas_target: &Canvas2dTarget,
    ) {
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
                        queue
                            .write_texture(
                                ImageCopyTexture {
                                    texture: &self.glyph_cache_texture,
                                    mip_level: 0,
                                    origin: Origin3d {
                                        x: rect.min[0],
                                        y: rect.min[1],
                                        z: 0,
                                    },
                                    aspect: TextureAspect::All
                                },
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
                    for draw_idx in 0..canvas_target.next_draw_text_call_index {
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
                            queue.write_buffer(&dst.text_vertex_buffer, 0, &vertex_bytes);
                            // update other data
                            dst.text_vertex_buffer_len = vertex_bytes.len();
                            dst.num_text_vertices = vertex_vec.len();
                            dst.draw_text_call_ranges = ranges;
                        } else {
                            // cannot reuse existing vertex buffer
                            // create new one
                            let text_vertex_buffer = device
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
                        device,
                        &self.glyph_cache_sampler,
                        &self.glyph_cache_bind_group_layout,
                        Extent2::new(w, h),
                    );
                    self.glyph_cache_texture = glyph_cache_texture;
                    self.glyph_cache_bind_group = glyph_cache_bind_group;
                },
            };
        }
    }

    pub(crate) fn render_call<'a>(
        &'a self,
        call: &'a DrawCallText,
        pass: &mut RenderPass<'a>,
        uniform_buffer_state: &'a Option<UniformBufferState>,
    ) {
        let text_vertex_state = self
            .text_vertex_state
            .as_ref()
            .unwrap();

        let vertex_range = text_vertex_state
            .draw_text_call_ranges[call.draw_text_call_index];
        if let Some((start, end)) = vertex_range {
            let uniform_buffer_state = uniform_buffer_state
                .as_ref()
                .unwrap();

            pass.set_pipeline(&self.text_pipeline);
            pass.set_vertex_buffer(
                0,
                text_vertex_state.text_vertex_buffer.slice(..),
            );
            pass.set_bind_group(
                0,
                &uniform_buffer_state.canvas2d_uniform_bind_group,
                &[call.uniform_offset as u32],
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
    }

    pub(crate) fn load_font(&mut self, file_data: &[u8]) -> Result<FontId> {
        // load font
        let font = FontArc::try_from_vec(file_data.into())?;

        // load into glyph brush
        let font_idx = self.glyph_brush.add_font(font.clone()).0;

        // add to vec
        self.fonts.push(font);
        
        // done
        Ok(FontId(font_idx))
    }

    pub(crate) fn lay_out_text(&self, text_block: &TextBlock) -> LayedOutTextBlock {
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
    //pt * 4.0 / 3.0
    pt * 20.0 / 12.0
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
