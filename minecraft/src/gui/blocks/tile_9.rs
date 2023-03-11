
use crate::{
    asset::loader::ImageClipper,
    gui::{
        GuiNode,
        GuiSpatialContext,
        GuiBlock,
        DimParentSets,
    },
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
use vek::*;
use anyhow::*;


// ==== image types and loading logic ====

#[derive(Debug, Clone)]
pub struct Tile9CropConfig<'a, 'b, 'c> {
    pub base: &'a ImageClipper<'b, 'c>,

    pub start: Vec2<u32>,
    pub extent: Extent2<u32>,

    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone)]
pub struct Tile9Parts<I>(pub [[I; 3]; 3]);

impl<I> Tile9Parts<I> {
    pub fn map<J, F: Fn(I) -> J>(self, f: F) -> Tile9Parts<J> {
        Tile9Parts(self.0
            .map(|col| col
                .map(|i| f(i))))
    }
}

pub fn tile_9_crop(
    cfg: &Tile9CropConfig,
) -> Tile9Parts<GpuImage>
{
    // assert ranges possible
    assert!(cfg.top + cfg.bottom < cfg.extent.h);
    assert!(cfg.left + cfg.right < cfg.extent.w);

    // ensure image sufficiently large
    let req_size = Extent2::<u32>::from(cfg.start + cfg.extent);

    // prep segments (per-axis arrays of 1D start+extent tuples)
    let h_segs = [
        (0, cfg.left),
        (cfg.left, cfg.extent.w - (cfg.left + cfg.right)),
        (cfg.extent.w - cfg.right, cfg.right),
    ];
    let v_segs = [
        (0, cfg.top),
        (cfg.top, cfg.extent.h - (cfg.top + cfg.bottom)),
        (cfg.extent.h - cfg.bottom, cfg.bottom),
    ];

    // prep regions (Tile9Parts of 2D start+extent tuples)
    let regions = Tile9Parts(h_segs
        .map(|(x_start, x_extent)| v_segs
            .map(|(y_start, y_extent)| (
                cfg.start + Vec2::new(x_start, y_start),
                Extent2::new(x_extent, y_extent),
            ))));

    // crop
    regions.map(|(start, extent)| cfg.base.load_clip(start, extent))
}


// ==== GUI block ====

pub fn tile_9<'a, I: Into<Extent2<f32>>>(
    images: &'a Tile9Parts<GpuImage>,
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
    images: &'a Tile9Parts<GpuImage>,
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

    fn draw(self, _: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let mut canvas = canvas.reborrow()
            .debug_tag("tile_9");

        // compute edge sizes
        let top = self.inner.logical_tile_size.h
            * self.inner.frac_top
            * self.scale;
        let bottom = self.inner.logical_tile_size.h
            * self.inner.frac_bottom
            * self.scale;
        let left = self.inner.logical_tile_size.w
            * self.inner.frac_left
            * self.scale;
        let right = self.inner.logical_tile_size.w
            * self.inner.frac_right
            * self.scale;

        // cap edge sizes if squashed too much
        let half_height = self.size.h / 2.0;
        let half_width = self.size.w / 2.0;
        
        let top = top.min(half_height);
        let bottom = bottom.min(half_height);
        let left = left.min(half_width);
        let right = right.min(half_width);

        // prep segments (per-axis 1D translate+size+tex extent tuples)
        let h_middle = self.size.w - (left + right);
        let v_middle = self.size.h - (top + bottom);

        let h_tile_middle = self.inner.logical_tile_size.w
            * (1.0 - (self.inner.frac_left + self.inner.frac_right))
            * self.scale;
        let v_tile_middle = self.inner.logical_tile_size.h
            * (1.0 - (self.inner.frac_top + self.inner.frac_bottom))
            * self.scale;

        let h_segs = [
            (0.0, left, 1.0),
            (left, h_middle, h_middle / h_tile_middle),
            (self.size.w - right, right, 1.0),
        ];
        let v_segs = [
            (0.0, top, 1.0),
            (top, v_middle, v_middle / v_tile_middle),
            (self.size.h - bottom, bottom, 1.0),
        ];
        //dbg!(&h_segs);
        //dbg!(&v_segs);

        // draw
        for i in 0..3 {
            for j in 0..3 {
                let (x_translate, w, tex_w) = h_segs[i];
                let (y_translate, h, tex_h) = v_segs[j];

                canvas.reborrow()
                    .translate([x_translate, y_translate])
                    .draw_image_uv(
                        &self.inner.images.0[i][j],
                        [w, h],
                        0.0,
                        [tex_w, tex_h],
                    );
            }
        }
    }
}
