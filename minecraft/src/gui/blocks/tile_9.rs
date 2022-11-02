
use crate::gui::{
    GuiNode,
    GuiSpatialContext,
    GuiBlock,
    DimParentSets,
};
use super::simple_gui_block::{
    SimpleGuiBlock,
    simple_blocks_cursor_impl,
};
use graphics::{
    Renderer,
    frame_content::{
        GpuImage,
        Canvas2,
    },
};
use image::DynamicImage;
use vek::*;


// ==== image types and loading logic ====


/// Builder for `Tile9Images`.
#[derive(Debug, Clone)]
pub struct Tile9ImagesBuilder<'a> {
    pub base_image: &'a DynamicImage,
    pub px_start: Vec2<u32>,
    pub px_extent: Extent2<u32>,
    pub px_top: u32,
    pub px_bottom: u32,
    pub px_left: u32,
    pub px_right: u32,
}

impl<'a> Tile9ImagesBuilder<'a> {
    /// Load the images into the renderer, building the `Tile9Images`.
    pub fn build(&self, renderer: &Renderer) -> Tile9Images {
        assert!(self.px_top + self.px_bottom < self.px_extent.h);
        assert!(self.px_left + self.px_right < self.px_extent.w);

        let px_h_middle = self.px_extent.w - self.px_left - self.px_right;
        let px_v_middle = self.px_extent.h - self.px_top - self.px_bottom;

        let corners = [
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ]
            .map(|(bottom, right)| self.base_image.crop_imm(
                // start x:
                self.px_start.x + match right {
                    false => 0,
                    true => self.px_extent.w - self.px_right,
                },
                // start y:
                self.px_start.y + match bottom {
                    false => 0,
                    true => self.px_extent.h - self.px_bottom,
                },
                // extent w:
                match right {
                    false => self.px_left,
                    true => self.px_right,
                },
                // extent h:
                match bottom {
                    false => self.px_top,
                    true => self.px_bottom,
                },
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let h_edges = [
            (0, self.px_top),
            (self.px_extent.h - self.px_bottom, self.px_bottom),
        ]
            .map(|(offset, extent)| self.base_image.crop_imm(
                // start x:
                self.px_start.x + self.px_left,
                // start y:
                self.px_start.y + offset,
                // extent w:
                px_h_middle,
                // extent h:
                extent,
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let v_edges = [
            (0, self.px_left),
            (self.px_extent.w - self.px_right, self.px_right)
        ]
            .map(|(offset, extent)| self.base_image.crop_imm(
                // start x:
                self.px_start.x + offset,
                // start y:
                self.px_start.y + self.px_top,
                // extent w:
                extent,
                // extent h:
                px_v_middle,
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let middle = self.base_image
            .crop_imm(
                self.px_start.x + self.px_left,
                self.px_start.y + self.px_top,
                px_h_middle,
                px_v_middle,
            );
        let middle = renderer.load_image_raw(middle);
      
        Tile9Images {
            corners,
            h_edges,
            v_edges,
            middle,
        }
    }
}

/// Images for a 9-part (corners, edges, center) tileable texture.
#[derive(Debug, Clone)]
pub struct Tile9Images {
    /// Top-left, top-right, bottom-left, bottom-right.
    pub corners: [GpuImage; 4],
    /// Top, bottom.
    pub h_edges: [GpuImage; 2],
    /// Left, right.
    pub v_edges: [GpuImage; 2],
    /// The middle image.
    pub middle: GpuImage,
}


// ==== GUI block ====


pub fn tile_9<'a, I: Into<Extent2<f32>>>(
    images: &'a Tile9Images,
    logical_tile_size: I,
    frac_top: f32,
    frac_bottom: f32,
    frac_left: f32,
    frac_right: f32,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    Tile9 {
        images,
        logical_tile_size: logical_tile_size.into(),
        frac_top,
        frac_bottom,
        frac_left,
        frac_right,
    }
}

#[derive(Debug)]
struct Tile9<'a> {
    images: &'a Tile9Images,
    /// Size of the whole (unsliced) image before scaling and tiling.
    logical_tile_size: Extent2<f32>,
    /// Fraction of the whole (unsliced) image taken by the top edge.
    frac_top: f32,
    /// Fraction of the whole (unsliced) image taken by the bottom edge.
    frac_bottom: f32,
    /// Fraction of the whole (unsliced) image taken by the left edge.
    frac_left: f32,
    /// Fraction of the whole (unsliced) image taken by the right edge.
    frac_right: f32,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<Tile9<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext, canvas: &mut Canvas2<'a, '_>) {
        let half_height = self.size.h / 2.0;
        let half_width = self.size.w / 2.0;

        let top = f32::min(self.inner.logical_tile_size.h * self.inner.frac_top * self.scale, half_height);
        let bottom = f32::min(self.inner.logical_tile_size.h * self.inner.frac_bottom * self.scale, half_height);

        let left = f32::min(self.inner.logical_tile_size.w * self.inner.frac_left * self.scale, half_width);
        let right = f32::min(self.inner.logical_tile_size.w * self.inner.frac_right * self.scale, half_width);

        let middle_size = self.size - Vec2 {
            x: left + right,
            y: top + bottom,
        };
        let middle_tex_extent = 
            middle_size
            / (
                Extent2 {
                    w: 1.0 - (self.inner.frac_left + self.inner.frac_right),
                    h: 1.0 - (self.inner.frac_top + self.inner.frac_bottom),
                }
                * self.inner.logical_tile_size
                * self.scale
            );
        

        for ((is_bottom, is_right), image) in [
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ].into_iter().zip(&self.inner.images.corners)
        {
            canvas.reborrow()
                .translate(Vec2 {
                    x: match is_right {
                        false => 0.0,
                        true => self.size.w - right
                    },
                    y: match is_bottom {
                        false => 0.0,
                        true => self.size.h - bottom,
                    },
                })
                .draw_image_uv(
                    image,
                    Extent2 {
                        w: match is_right {
                            false => left,
                            true => right,
                        },
                        h: match is_bottom {
                            false => top,
                            true => bottom,
                        },
                    },
                    0.0,
                    1.0,
                );
        }

        for (is_bottom, image) in [false, true].iter()
            .zip(&self.inner.images.h_edges)
        {
            canvas.reborrow()
                .translate(Vec2 {
                    x: left,
                    y: match is_bottom {
                        false => 0.0,
                        true => self.size.h - bottom,
                    },
                })
                .draw_image_uv(
                    image,
                    Extent2 {
                        w: middle_size.w,
                        h: match is_bottom {
                            false => top,
                            true => bottom,
                        },
                    },
                    [0.0; 2],
                    Extent2 {
                        w: middle_tex_extent.w,
                        h: 1.0,
                    },
                );
        }

        for (is_right, image) in [false, true].iter()
            .zip(&self.inner.images.v_edges)
        {
            canvas.reborrow()
                .translate(Vec2 {
                    x: match is_right {
                        false => 0.0,
                        true => self.size.w - right,
                    },
                    y: top,
                })
                .draw_image_uv(
                    image,
                    Extent2 {
                        w: match is_right {
                            false => left,
                            true => right,
                        },
                        h: middle_size.h,
                    },
                    [0.0; 2],
                    Extent2 {
                        w: 1.0,
                        h: middle_tex_extent.h,
                    },
                );
        }

        canvas.reborrow()
            .translate([left, top])
            .draw_image_uv(
                &self.inner.images.middle,
                middle_size,
                [0.0; 2],
                middle_tex_extent,
            );
    }
}
