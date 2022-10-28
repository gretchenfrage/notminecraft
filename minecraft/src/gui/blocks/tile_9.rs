use super::*;
    use graphics::frame_content::GpuImage;
    use image::DynamicImage;
    use vek::*;

    /// Specification for how to slice a 9-part tileable image from a base image.
    #[derive(Debug, Clone)]
    pub struct LoadTile9ImagesConfig<'a> {
        pub raw_image: &'a DynamicImage,
        pub px_start: Vec2<u32>,
        pub px_extent: Extent2<u32>,
        pub px_top: u32,
        pub px_bottom: u32,
        pub px_left: u32,
        pub px_right: u32,
    }

    impl<'a> LoadTile9ImagesConfig<'a> {
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

    pub fn tile_9_gui_block<'a>(
        images: &'a Tile9Images,
        size_unscaled_untiled: Extent2<f32>,
        frac_top: f32,
        frac_bottom: f32,
        frac_left: f32,
        frac_right: f32,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        Tile9GuiBlock {
            images,
            size_unscaled_untiled,
            frac_top,
            frac_bottom,
            frac_left,
            frac_right,
        }
    }

    struct Tile9GuiBlock<'a> {
        images: &'a Tile9Images,
        /// Size of the whole (unsliced) image before scaling and tiling.
        size_unscaled_untiled: Extent2<f32>,
        /// Fraction of the whole (unsliced) image taken by the top edge.
        frac_top: f32,
        /// Fraction of the whole (unsliced) image taken by the bottom edge.
        frac_bottom: f32,
        /// Fraction of the whole (unsliced) image taken by the left edge.
        frac_left: f32,
        /// Fraction of the whole (unsliced) image taken by the right edge.
        frac_right: f32,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for Tile9GuiBlock<'a> {
        type Sized = Tile9SizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = Tile9SizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    struct Tile9SizedGuiBlock<'a> {
        block: Tile9GuiBlock<'a>,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for Tile9SizedGuiBlock<'a> {
        fn draw(mut self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let half_height = self.size.h / 2.0;
            let half_width = self.size.w / 2.0;

            let top = f32::min(self.block.size_unscaled_untiled.h * self.block.frac_top * self.scale, half_height);
            let bottom = f32::min(self.block.size_unscaled_untiled.h * self.block.frac_bottom * self.scale, half_height);

            let left = f32::min(self.block.size_unscaled_untiled.w * self.block.frac_left * self.scale, half_width);
            let right = f32::min(self.block.size_unscaled_untiled.w * self.block.frac_right * self.scale, half_width);

            let middle_size = self.size - Vec2 {
                x: left + right,
                y: top + bottom,
            };
            let middle_tex_extent = 
                middle_size
                / (
                    Extent2 {
                        w: 1.0 - (self.block.frac_left + self.block.frac_right),
                        h: 1.0 - (self.block.frac_top + self.block.frac_bottom),
                    }
                    * self.block.size_unscaled_untiled
                    * self.scale
                );
            

            for ((is_bottom, is_right), image) in [
                (false, false),
                (false, true),
                (true, false),
                (true, true),
            ].into_iter().zip(&self.block.images.corners)
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
                        [0.0; 2],
                        [1.0; 2],
                    );
            }

            for (is_bottom, image) in [false, true].iter()
                .zip(&self.block.images.h_edges)
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
                .zip(&self.block.images.v_edges)
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
                    &self.block.images.middle,
                    middle_size,
                    [0.0; 2],
                    middle_tex_extent,
                );
        }
    }