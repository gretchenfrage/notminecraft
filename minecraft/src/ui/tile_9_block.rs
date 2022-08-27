
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        GpuImage,
    },
};
use std::iter::once;
use image::DynamicImage;
use vek::*;


/// Specification for how to slice a 9-part tileable image from a base image.
#[derive(Debug, Clone)]
pub struct LoadTile9ImagesConfig {
    pub raw_image: DynamicImage,
    pub px_start: Vec2<u32>,
    pub px_extent: Extent2<u32>,
    pub px_top: u32,
    pub px_bottom: u32,
    pub px_left: u32,
    pub px_right: u32,
}

impl LoadTile9ImagesConfig {
    pub fn load(&self, renderer: &Renderer) -> Tile9Images {
        // TODO: we really could do the cropping on GPU relatively easily
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
            .map(|(bottom, right)| self.raw_image.crop_imm(
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
            .map(|(offset, extent)| self.raw_image.crop_imm(
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
            .map(|(offset, extent)| self.raw_image.crop_imm(
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
        let middle = self.raw_image
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

/// 9-part (corners, edges, center) tileable image.
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

#[derive(Debug, Clone)]
pub struct UiTile9BlockConfig {
    /// The images.
    pub images: Tile9Images,
    /// Size of the whole (unsliced) image before scaling and tiling.
    pub size_unscaled_untiled: Extent2<f32>,
    /// Fraction of the whole (unsliced) image taken by the top edge.
    pub frac_top: f32,
    /// Fraction of the whole (unsliced) image taken by the bottom edge.
    pub frac_bottom: f32,
    /// Fraction of the whole (unsliced) image taken by the left edge.
    pub frac_left: f32,
    /// Fraction of the whole (unsliced) image taken by the right edge.
    pub frac_right: f32,
}

/// UI block with a 9-part (corners, edges, center) tiling texture and settable
/// size.
#[derive(Debug, Clone)]
pub struct UiTile9Block {
    config: UiTile9BlockConfig,

    size: Extent2<f32>,
    scale: f32,

    draw_params: DrawParams,
}

#[derive(Debug, Copy, Clone)]
struct DrawParams {
    corners: [DrawPartParams; 4],
    h_edges: [DrawPartParams; 2],
    v_edges: [DrawPartParams; 2],
    middle: DrawPartParams,
}

#[derive(Debug, Copy, Clone)]
struct DrawPartParams {
    translate: Vec2<f32>,
    size: Extent2<f32>,
    tex_extent: Extent2<f32>,
}

impl DrawParams {
    fn new(
        config: &UiTile9BlockConfig,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self {
        let half_height = size.h / 2.0;
        let half_width = size.w / 2.0;

        let top = f32::min(size.h * config.frac_top * scale, half_height);
        let bottom = f32::min(size.h * config.frac_bottom * scale, half_height);

        let left = f32::min(size.w * config.frac_left * scale, half_width);
        let right = f32::min(size.w * config.frac_right * scale, half_width);

        let middle_size = size - Vec2 {
            x: left + right,
            y: top + bottom,
        };
        let middle_tex_extent = 
            middle_size
            / (
                Extent2::new(1.0, 1.0)
                - Extent2 {
                    w: config.frac_left + config.frac_right,
                    h: config.frac_top + config.frac_bottom,
                }
                * config.size_unscaled_untiled
                * scale
            );

        let corners = [
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ]
            .map(|(is_bottom, is_right)| DrawPartParams {
                translate: Vec2 {
                    x: match is_right {
                        false => 0.0,
                        true => size.w - right
                    },
                    y: match is_bottom {
                        false => 0.0,
                        true => size.h - bottom,
                    },
                },
                size: Extent2 {
                    w: match is_right {
                        false => left,
                        true => right,
                    },
                    h: match is_bottom {
                        false => top,
                        true => bottom,
                    },
                },
                tex_extent: [1.0, 1.0].into(),
            });

        let h_edges = [false, true]
            .map(|is_bottom| DrawPartParams {
                translate: Vec2 {
                    x: left,
                    y: match is_bottom {
                        false => 0.0,
                        true => size.h - bottom,
                    },
                },
                size: Extent2 {
                    w: middle_size.w,
                    h: match is_bottom {
                        false => top,
                        true => bottom,
                    },
                },
                tex_extent: Extent2 {
                    w: middle_tex_extent.w,
                    h: 1.0,
                },
            });
        let v_edges = [false, true]
            .map(|is_right| DrawPartParams {
                translate: Vec2 {
                    x: match is_right {
                        false => 0.0,
                        true => size.w - right,
                    },
                    y: top,
                },
                size: Extent2 {
                    w: match is_right {
                        false => left,
                        true => right,
                    },
                    h: middle_size.h,
                },
                tex_extent: Extent2 {
                    w: 1.0,
                    h: middle_tex_extent.h,
                },
            });
        let middle = DrawPartParams {
            translate: [left, top].into(),
            size: middle_size,
            tex_extent: middle_tex_extent,
        };

        DrawParams {
            corners,
            h_edges,
            v_edges,
            middle,
        }
    }

    pub fn iter_with_images<'a>(
        &'a self,
        images: &'a Tile9Images,
    ) -> impl Iterator<Item=(&'a DrawPartParams, &'a GpuImage)> + 'a
    {
        Iterator::zip(self.corners.iter(), images.corners.iter())
            .chain(Iterator::zip(self.h_edges.iter(), images.h_edges.iter()))
            .chain(Iterator::zip(self.v_edges.iter(), images.v_edges.iter()))
            .chain(once((&self.middle, &images.middle)))
    }
}

impl UiTile9Block {
    pub fn new(
        config: UiTile9BlockConfig,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self {
        let draw_params = DrawParams::new(&config, size, scale);

        UiTile9Block {
            config,

            size,
            scale,

            draw_params,
        }
    }
}

impl UiBlock for UiTile9Block {
    type WidthChanged = False;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        for (params, image) in self.draw_params
            .iter_with_images(&self.config.images)
        {
            canvas.reborrow()
                .translate(params.translate)
                .draw_image_uv(
                    image,
                    params.size,
                    [0.0, 0.0],
                    params.tex_extent,
                );
        }
    }

    fn width(&self) -> f32 {
        self.size.w
    }

    fn height(&self) -> f32 {
        self.size.h
    }

    fn scale(&self) -> f32 {
        self.scale
    }

    fn set_scale(&mut self, _: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    )
    {
        self.scale = scale;

        self.draw_params = DrawParams::new(
            &self.config,
            self.size,
            self.scale,
        );

        (False, False)
    }
}

impl UiBlockSetWidth for UiTile9Block {
    fn set_width(&mut self, _: &Renderer, width: f32) {
        self.size.w = width;;

        self.draw_params = DrawParams::new(
            &self.config,
            self.size,
            self.scale
        );
    }
}

impl UiBlockSetHeight for UiTile9Block {
    fn set_height(&mut self, _: &Renderer, height: f32) {
        self.size.h = height;

        self.draw_params = DrawParams::new(
            &self.config,
            self.size,
            self.scale
        );
    }
}
