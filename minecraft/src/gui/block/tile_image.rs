use super::*;
    use graphics::frame_content::GpuImage;

    pub fn tile_image_gui_block<'a, E: Into<Extent2<f32>>>(image: &'a GpuImage, size_unscaled_untiled: E) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        let size_unscaled_untiled = size_unscaled_untiled.into();
        TileImageGuiBlock {
            image,
            size_unscaled_untiled,
        }
    }

    struct TileImageGuiBlock<'a> {
        image: &'a GpuImage,
        size_unscaled_untiled: Extent2<f32>,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for TileImageGuiBlock<'a> {
        type Sized = SizedTileImageGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = SizedTileImageGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    struct SizedTileImageGuiBlock<'a> {
        block: TileImageGuiBlock<'a>,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for SizedTileImageGuiBlock<'a> {
        fn draw(mut self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let tex_extent = self.size / (self.block.size_unscaled_untiled * self.scale);
            canvas.reborrow()
                .draw_image_uv(
                    &self.block.image,
                    self.size,
                    [0.0, 0.0],
                    tex_extent,
                );
        }
    }