
use super::UiSize;
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        GpuImage,
    },
};
use image::DynamicImage;
use vek::*;


pub struct UiTile9 {
    size: UiSize,

    // top-left, top-right, bottom-left, bottom-right
    corners: [GpuImage; 4],
    // top, bottom
    h_edges: [GpuImage; 2],
    // left, right
    v_edges: [GpuImage; 2],
    middle: GpuImage,

    texture_scale: f32,
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,

    h_middle: u32,
    v_middle: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Tile9PxRanges { // TODO px ranges instead?
    pub start: Vec2<u32>,
    pub extent: Extent2<u32>,
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl UiTile9 {
    pub fn new(
        renderer: &Renderer,
        texture: DynamicImage,
        ranges: Tile9PxRanges,
        texture_scale: f32,
        size: UiSize,
    ) -> Self
    {
        assert!(ranges.top + ranges.bottom < ranges.extent.h);
        assert!(ranges.left + ranges.right < ranges.extent.w);

        let h_middle = ranges.extent.w - ranges.left - ranges.right;
        let v_middle = ranges.extent.h - ranges.top - ranges.bottom;

        let corners = [
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ]
            .map(|(bottom, right)| texture.crop_imm(
                // start x:
                ranges.start.x + match right {
                    false => 0,
                    true => ranges.extent.w - ranges.right,
                },
                // start y:
                ranges.start.y + match bottom {
                    false => 0,
                    true => ranges.extent.h - ranges.bottom,
                },
                // extent w:
                match right {
                    false => ranges.left,
                    true => ranges.right,
                },
                // extent h:
                match bottom {
                    false => ranges.top,
                    true => ranges.bottom,
                },
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let h_edges = [
            (0, ranges.top),
            (ranges.extent.h - ranges.bottom, ranges.bottom),
        ]
            .map(|(offset, extent)| texture.crop_imm(
                // start x:
                ranges.start.x + ranges.left,
                // start y:
                ranges.start.y + offset,
                // extent w:
                h_middle,
                // extent h:
                extent,
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let v_edges = [
            (0, ranges.left),
            (ranges.extent.w - ranges.right, ranges.right)
        ]
            .map(|(offset, extent)| texture.crop_imm(
                // start x:
                ranges.start.x + offset,
                // start y:
                ranges.start.y + ranges.top,
                // extent w:
                extent,
                // extent h:
                v_middle,
            ))
            .map(|texture| renderer.load_image_raw(texture));
        let middle = texture
            .crop_imm(
                ranges.start.x + ranges.left,
                ranges.start.y + ranges.top,
                h_middle,
                v_middle,
            );
        let middle = renderer.load_image_raw(middle);

        UiTile9 {
            size,
            corners,
            h_edges,
            v_edges,
            middle,
            texture_scale,
            top: ranges.top as f32,
            bottom: ranges.bottom as f32,
            left: ranges.left as f32,
            right: ranges.right as f32,
            h_middle,
            v_middle,
        }
    }

    pub fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        for (i, (bottom, right)) in [
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ].into_iter().enumerate()
        {
            canvas.reborrow()
                .translate([
                    match right {
                        false => 0.0,
                        true => self.size.size.w - self.right * self.texture_scale * self.size.scale,
                    },
                    match bottom {
                        false => 0.0,
                        true => self.size.size.h - self.bottom * self.texture_scale * self.size.scale,
                    },
                ])
                .draw_image(
                    &self.corners[i],
                    Extent2 {
                        w: match right {
                            false => self.left,
                            true => self.right,
                        },
                        h: match bottom {
                            false => self.top,
                            true => self.bottom,
                        },
                    } * self.texture_scale * self.size.scale,
                );
        }

        for (i, bottom) in [false, true].into_iter().enumerate()
        {
            canvas.reborrow()
                .translate([
                    self.left * self.texture_scale * self.size.scale,
                    match bottom {
                        false => 0.0,
                        true => self.size.size.h - self.bottom * self.texture_scale * self.size.scale,
                    },
                ])
                .draw_image_uv(
                    &self.h_edges[i],
                    [
                        self.size.size.w - (self.left + self.right) * self.texture_scale * self.size.scale,
                        match bottom {
                            false => self.top,
                            true => self.bottom,
                        } * self.texture_scale * self.size.scale,
                    ],
                    [0.0, 0.0],
                    [
                        (self.size.size.w / self.texture_scale / self.size.scale - self.left - self.right) / self.h_middle as f32,
                        1.0,
                    ],
                );
        }

        for (i, right) in [false, true].into_iter().enumerate()
        {
            canvas.reborrow()
                .translate([
                    match right {
                        false => 0.0,
                        true => self.size.size.w - self.right * self.texture_scale * self.size.scale,
                    },
                    self.top * self.texture_scale * self.size.scale,
                ])
                .draw_image_uv(
                    &self.v_edges[i],
                    [
                        match right {
                            false => self.left,
                            true => self.right
                        } * self.texture_scale * self.size.scale,
                        self.size.size.h - (self.top + self.bottom) * self.texture_scale * self.size.scale,
                    ],
                    [0.0, 0.0],
                    [
                        1.0,
                        (self.size.size.h / self.texture_scale / self.size.scale - self.top - self.bottom) / self.v_middle as f32,
                    ],
                );
        }

        canvas.reborrow()
            .translate(
                Vec2 { x: self.left, y: self.top }
                * self.texture_scale
                * self.size.scale
            )
            .draw_image_uv(
                &self.middle,
                [
                    self.size.size.w - (self.left + self.right) * self.texture_scale * self.size.scale,
                    self.size.size.h - (self.top + self.bottom) * self.texture_scale * self.size.scale,
                ],
                [0.0, 0.0],
                [
                    (self.size.size.w / self.texture_scale / self.size.scale - self.left - self.right) / self.h_middle as f32,
                    (self.size.size.h / self.texture_scale / self.size.scale - self.top - self.bottom) / self.v_middle as f32,
                ],
            );
    }

    pub fn set_size(&mut self, size: impl Into<Extent2<f32>>) {
        self.size.size = size.into();
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.size.scale = scale;
    }
}

