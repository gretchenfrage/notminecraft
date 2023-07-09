
use crate::{
    vertex::{
        VertexStruct,
        vertex_struct,
    },
    shader::load_shader,
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
};
use std::sync::Arc;
use glyph_brush::{
    self as gb,
    ab_glyph::{
        FontArc,
        Font,
    },
    GlyphBrush,
    GlyphBrushBuilder,
    GlyphPositioner,
};
use wgpu::{ 
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,   
    },
};
use vek::*;
use anyhow::*;
use tracing::*;


// ==== text block ====


/// Block of text with specification of how to display it.
#[derive(Debug, Copy, Clone)]
pub struct TextBlock<'a> {
    /// The spans of text to flow together.
    pub spans: &'a [TextSpan<'a>],
    /// Specification of horizontal align/wrap behavior.
    pub h_align: HAlign,
    /// Specification of vertical align/wrap behavior.
    pub v_align: VAlign,
    pub wrap_width: Option<f32>,
}

/// Specification of text horizontal align/wrap behavior.
#[derive(Debug, Copy, Clone)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

impl HAlign {
    pub fn sign(self) -> i8 { // TODO make this all a single Sign enum in the first place
        match self {
            HAlign::Left => -1,
            HAlign::Center => 0,
            HAlign::Right => 1,
        }
    }
}

/// Specification of text vertical align/wrap behavior.
#[derive(Debug, Copy, Clone)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

impl VAlign {
    pub fn sign(self) -> i8 {
        match self {
            VAlign::Top => -1,
            VAlign::Center => 0,
            VAlign::Bottom => 1,
        }
    }
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
    pub font: FontId,
    /// Text height units.
    pub font_size: f32,
    /// Text color.
    pub color: Rgba<f32>,
}


// ==== layed out text block ====

impl<'a> TextBlock<'a> {
    /// Produce a corresponding glyph_brush `Layout`.
    fn to_layout(&self) -> gb::Layout<gb::BuiltInLineBreaker> {
        let gb_h_align = match self.h_align {
            HAlign::Left => gb::HorizontalAlign::Left,
            HAlign::Center => gb::HorizontalAlign::Center,
            HAlign::Right => gb::HorizontalAlign::Right,
            /*
            HAlign::Left { .. } => gb::HorizontalAlign::Left,
            HAlign::Center { .. } => gb::HorizontalAlign::Center,
            HAlign::Right { .. } => gb::HorizontalAlign::Right,*/
        };
        let gb_v_align = match self.v_align {
            VAlign::Top => gb::VerticalAlign::Top,
            VAlign::Center => gb::VerticalAlign::Center,
            VAlign::Bottom => gb::VerticalAlign::Bottom,
            /*VAlign::Top => gb::VerticalAlign::Top,
            VAlign::Center { .. } => gb::VerticalAlign::Center,
            VAlign::Bottom { .. } => gb::VerticalAlign::Bottom,*/
        };
        if self.wrap_width.is_some() {
            gb::Layout::Wrap {
                line_breaker: gb::BuiltInLineBreaker::UnicodeLineBreaker,
                h_align: gb_h_align,
                v_align: gb_v_align,
            }
        } else {
            gb::Layout::SingleLine {
                line_breaker: gb::BuiltInLineBreaker::UnicodeLineBreaker,
                h_align: gb_h_align,
                v_align: gb_v_align,
            }
        }
        /*
        let single_line = matches!(
            self.horizontal_align,
            HAlign::Left { width: None },
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
        */
    }

    /// Produce a corresponding glyph_brush `SectionGeometry`.
    fn to_section_geometry(&self) -> gb::SectionGeometry {
        /*
        let width = match self.horizontal_align {
            HAlign::Left { width } => width,
            HAlign::Center { width } => Some(width),
            HAlign::Right { width } => Some(width),
        };
        let height = match self.vertical_align {
            VAlign::Top => None,
            VAlign::Center { height } => Some(height),
            VAlign::Bottom { height } => Some(height),
        };*/
        gb::SectionGeometry {
            screen_position: (0.0, 0.0),
            bounds: (
                self.wrap_width.unwrap_or(f32::INFINITY),
                f32::INFINITY,
                /*
                width.unwrap_or(f32::INFINITY),
                height.unwrap_or(f32::INFINITY),*/
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
            font_id: gb::FontId(self.font.0),
        }
    }
}

/// Block of text with a pre-computed plan for how to lay it out.
#[derive(Debug, Clone)]
pub struct LayedOutTextBlock(Arc<LayedOutTextBlockInner>);

#[derive(Debug, Clone)]
pub struct LayedOutTextBlockInner {
    glyphs: Vec<LayedOutGlyph>,
    bounds: gb::ab_glyph::Rect,
    content_bounds: [Vec2<f32>; 2],
}

#[derive(Debug, Clone)]
struct LayedOutGlyph {
    section_glyph: gb::SectionGlyph,
    color: Rgba<u8>,
}

impl LayedOutTextBlock {
    pub fn content_bounds(&self) -> [Vec2<f32>; 2] {
        self.0.content_bounds
    }
}


// ==== pipeline ====

pub struct TextPipeline {
    text_pipeline: RenderPipeline,    
    glyph_brush: GlyphBrush<TextQuad, GlyphExtra>,
    fonts: Vec<FontArc>,
    glyph_cache_texture: Texture,
    glyph_cache_sampler: Sampler,
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
    dst: (Vec2<f32>, Extent2<f32>),
    color: Rgba<u8>,
    draw_text_call_index: usize,
}

#[derive(Debug, Copy, Clone)]
struct TextVertex {
    pos: Vec2<f32>,
    tex: Vec2<f32>,
    color: Rgba<u8>,
}

vertex_struct!(TextVertex {
    (pos:   Vec2<f32>) (layout(location=0) in vec2),
    (tex:   Vec2<f32>) (layout(location=1) in vec2),
    (color: Rgba<u8> ) (layout(location=2) in vec4),
});

pub struct PreRenderer<'a> {
    pipeline: &'a mut TextPipeline,
    next_draw_text_call_index: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct PreppedDrawText(usize);


impl TextPipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self>
    {
        let text_vs_module = device
            .create_shader_module(load_shader!("text.vert")?);
        let text_fs_module = device
            .create_shader_module(load_shader!("text.frag")?);
        let glyph_cache_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("glyph cache bind group layout"),
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
        let text_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("text pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
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
                        Some(ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        }),
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState::default(),
                multiview: None,
            });
        let glyph_brush = GlyphBrushBuilder::using_fonts::<FontArc>(Vec::new())
            .build();
        let glyph_cache_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("glyph cache sampler"),
                ..Default::default()
            });
        let (
            glyph_cache_texture,
            glyph_cache_bind_group,
        ) = finish_glyph_cache_creation(
            &device,
            &glyph_cache_bind_group_layout,
            &glyph_cache_sampler,
            glyph_brush.texture_dimensions().into(),
        );

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

    pub(crate) fn begin_pre_render(&mut self) -> PreRenderer {
        PreRenderer {
            pipeline: self,
            next_draw_text_call_index: 0,
        }
    }
}

fn update_glyph_cache_texture(
    glyph_cache_texture: &Texture,
    queue: &Queue,
    rect: gb::Rectangle<u32>,
    unpadded_data: &[u8],
) {
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
                texture: glyph_cache_texture,
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
}

/// Convert an `gb_glyph::Rect` to a (start, extent) tuple.
fn rect_to_src_extent(rect: gb::ab_glyph::Rect) -> (Vec2<f32>, Extent2<f32>) {
    (
        Vec2::new(rect.min.x, rect.min.y),
        (
            Vec2::new(rect.max.x, rect.max.y)
            - Vec2::new(rect.min.x, rect.min.y)
        ).into(),
    )
}

impl<'a> PreRenderer<'a> {
    pub(crate) fn pre_render(
        &mut self,
        text: &LayedOutTextBlock,
    ) -> PreppedDrawText
    {
        // get and increment the draw text call index to identify this batch of
        // glyphs
        let draw_text_call_index = self.next_draw_text_call_index;
        self.next_draw_text_call_index += 1;

        // extract the section glyphs from the text block
        let section_glyphs = text.0
            .glyphs
            .iter()
            .map(|glyph| glyph.section_glyph.clone())
            .collect();

        // produce the matching extra data for the text block
        let extra = text.0
            .glyphs
            .iter()
            .map(|glyph| GlyphExtra {
                draw_text_call_index,
                color: glyph.color,
            })
            .collect();

        // queue to the glyph brush
        self.pipeline
            .glyph_brush
            .queue_pre_positioned(
                section_glyphs,
                extra,
                text.0.bounds,
            );

        PreppedDrawText(draw_text_call_index)
    }

    pub(crate) fn finalize_pre_render(
        self,
        device: &Device,
        queue: &Queue,
    )
    {
        // process the queued glyph brush data
        // update the glyph cache texture as necessary
        // update the text vertex buffer as necessary

        

        // loop until glyph cache texture is large enough
        for attempt in 0.. {
            assert!(attempt < 100, "glyph cache update loop not breaking");
            let result = self.pipeline.glyph_brush
                .process_queued(
                    // callback for updating the glyph cache texture
                    |rect, unpadded_data| update_glyph_cache_texture(
                        &self.pipeline.glyph_cache_texture,
                        queue,
                        rect,
                        unpadded_data,
                    ),
                    // callback to convert glyph_brush vertex to text quad
                    |glyph_vertex| TextQuad {
                        src: rect_to_src_extent(glyph_vertex.tex_coords),
                        dst: rect_to_src_extent(glyph_vertex.pixel_coords),
                        color: glyph_vertex.extra.color,
                        draw_text_call_index: glyph_vertex.extra.draw_text_call_index,
                    },
                );
            match result {
                // TODO wtf??
                core::result::Result::Ok(gb::BrushAction::Draw(mut quads)) => {
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
                    for draw_idx in 0..self.next_draw_text_call_index {
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
                            .pipeline
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
                            self.pipeline.text_vertex_state = Some(TextVertexState {
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
                // TODO wtf??
                core::result::Result::Ok(gb::BrushAction::ReDraw) => break, // reuse existing vertex buffer
                Err(gb::BrushError::TextureTooSmall {
                    suggested: (w, h),
                }) => {
                    // increase glyph cache texture size and try again
                    trace!("increasing glyph cache texture size");
                    self.pipeline.glyph_brush.resize_texture(w, h);
                    let (glyph_cache_texture, glyph_cache_bind_group) = finish_glyph_cache_creation(
                        device,
                        &self.pipeline.glyph_cache_bind_group_layout,
                        &self.pipeline.glyph_cache_sampler,
                        Extent2::new(w, h),
                    );
                    self.pipeline.glyph_cache_texture = glyph_cache_texture;
                    self.pipeline.glyph_cache_bind_group = glyph_cache_bind_group;
                },
            };
        }
    }
}

impl TextPipeline {
    pub(crate) fn render<'a>(
        &'a self,
        text: PreppedDrawText,
        pass: &mut RenderPass<'a>,
    )
    {
        let text_vertex_state = self
            .text_vertex_state
            .as_ref()
            .unwrap();
        let vertex_range = text_vertex_state
            .draw_text_call_ranges[text.0];
        if let Some((start, end)) = vertex_range {
            pass.set_pipeline(&self.text_pipeline);
            pass
                .set_bind_group(
                    3,
                    &self.glyph_cache_bind_group,
                    &[],
                );
            pass.set_vertex_buffer(
                0,
                text_vertex_state.text_vertex_buffer.slice(..),
            );
            pass.draw(
                start as u32..end as u32, // TODO more careful casting?
                0..1,
            );
        }
    }

    pub(crate) fn load_font(&mut self, font: FontArc) -> FontId {
        // load into glyph brush
        let font_idx = self.glyph_brush.add_font(font.clone()).0;

        // add to vec
        self.fonts.push(font);
        
        // done
        FontId(font_idx)
    }

    pub(crate) fn lay_out_text(&self, text: &TextBlock) -> LayedOutTextBlock {
        // convert to glyph_brush types
        let layout = text.to_layout();
        let section_geometry = text.to_section_geometry();

        // have glyph_brush lay it out
        let section_glyphs = layout
            .calculate_glyphs(
                &self.fonts,
                &section_geometry,
                text.as_sections(),
            );
        let bounds = layout.bounds_rect(&section_geometry);

        let mut content_bounds = [
            Vec2::from(f32::INFINITY),
            Vec2::from(f32::NEG_INFINITY),
        ];
        for sec_glyph in section_glyphs.iter() {
            let pos = sec_glyph.glyph.position;
            let pos = Vec2::new(pos.x, pos.y);

            let font = &self.fonts[sec_glyph.font_id.0];

            if let Some(outline) = font.outline(sec_glyph.glyph.id)
            {
                let bounds = outline.bounds;

                let mut rel_min = Vec2::new(bounds.min.x, bounds.min.y);
                let mut rel_max = Vec2::new(bounds.max.x, bounds.max.y);

                for rel in [&mut rel_min, &mut rel_max] {
                    rel.y *= -1.0;
                    *rel /= font.units_per_em().unwrap_or(1.0);
                    *rel *= text.spans[sec_glyph.section_index].font_size;
                }

                let glyph_min = pos + rel_min;
                let glyph_max = pos + rel_max;

                content_bounds[0] = content_bounds[0]
                    .zip(glyph_min)
                    .map(|(a, b)| f32::min(a, b));
                content_bounds[1] = content_bounds[1]
                    .zip(glyph_max)
                    .map(|(a, b)| f32::max(a, b));
            }
        }

        // re-associate the color data
        let glyphs = section_glyphs
            .into_iter()
            .map(|section_glyph| LayedOutGlyph {
                color: text
                    .spans[section_glyph.section_index]
                    .color
                    .map(|n| (n * 255.0) as u8), // TODO better casting  handling everywhere for this
                section_glyph,
            })
            .collect();



        // done
        LayedOutTextBlock(Arc::new(LayedOutTextBlockInner {
            glyphs,
            bounds,
            content_bounds,
        }))
    }
}

/// Create a glyph cache texture with the given size, then create a glyph cache
/// bind group using that texture and a pre-created sampler and layout.
///
/// This logic is shared between the initial construction of the glyph cache,
/// and the resizing of the glyph cache.
fn finish_glyph_cache_creation(
    device: &Device,
    glyph_cache_bind_group_layout: &BindGroupLayout,
    glyph_cache_sampler: &Sampler,
    size: Extent2<u32>,
) -> (Texture, BindGroup)
{
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
