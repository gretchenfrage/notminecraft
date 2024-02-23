
use super::*;
use crate::{
    game_data::per_item::PerItem,
    item::*,
    gui::prelude::*,
};
use std::{
    f32::consts::PI,
    slice,
};


/// Reasonable-defaults `ItemGridRenderLogic` implementation.
#[derive(Debug)]
pub struct ItemGridDefaultRenderLogic<'a> {
    pub item_mesh: &'a PerItem<Mesh>,
    pub text_caches: slice::IterMut<'a, ItemSlotTextCache>,
}

/// Cache for the layed-out text for rendering an item slot.
#[derive(Debug, Default)]
pub struct ItemSlotTextCache {
    cached_count: Option<u8>,
    count_text: Option<GuiTextBlockInner>,
    cached_iid: Option<Option<RawItemId>>,
    name_text: Option<GuiTextBlockInner>,
}

impl<'a> ItemGridRenderLogic<'a, Option<ItemStack>> for ItemGridDefaultRenderLogic<'a> {
    fn draw(
        &mut self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        item_slot_idx: usize,
        item_slot: &'a Option<ItemStack>,
        size: f32,
        scale: f32,
        is_cursor_over: bool,
    ) {
        let text_cache = self.text_caches.next().expect("wrong number of text caches");
        // validate count text cache
        const SLOT_DEFAULT_TEXT_SIZE: f32 = 16.0;
        let count = item_slot.as_ref().map(|stack| stack.count.get()).unwrap_or(0);
        if text_cache.cached_count != Some(count) {
            text_cache.cached_count = Some(count);
            text_cache.count_text = if count > 1 {
                Some(GuiTextBlockInner::new(
                    &GuiTextBlockConfig {
                        text: &count.to_string(),
                        font: ctx.assets().font,
                        logical_font_size: SLOT_DEFAULT_TEXT_SIZE,
                        color: Rgba::white(),
                        h_align: HAlign::Right,
                        v_align: VAlign::Bottom,
                        shadow: true,
                    },
                    false,
                ))
            } else {
                None
            };
        }
        // validate name text cache
        let iid = item_slot.as_ref().map(|stack| stack.iid);
        if text_cache.cached_iid != Some(iid) {
            text_cache.cached_iid = Some(iid);
            text_cache.name_text = iid.map(|iid| {
                GuiTextBlockInner::new(
                    &GuiTextBlockConfig {
                        text: ctx.game().items_name[iid]
                            .map(|lang_key| &ctx.assets().lang[lang_key])
                            .unwrap_or_else(|| &ctx.game().items_machine_name[iid]),
                        font: ctx.assets().font,
                        logical_font_size: SLOT_DEFAULT_TEXT_SIZE,
                        color: Rgba::white(),
                        h_align: HAlign::Left,
                        v_align: VAlign::Top,
                        shadow: true,
                    },
                    false,
                )
            });
        }
        // draw model
        if let Some(stack) = item_slot.as_ref() {
            canvas.reborrow()
                .translate(size / 2.0)
                .scale(size * 0.616)
                .translate(-0.5)
                .begin_3d(
                    Mat4::new(
                        1.0,  0.0,  0.0, 0.5,
                        0.0, -1.0,  0.0, 0.5,
                        0.0,  0.0, 0.01, 0.5,
                        0.0,  0.0,  0.0, 1.0,
                    ),
                    Fog::None,
                )
                .rotate(Quaternion::rotation_x(-PI * 0.17))
                .rotate(Quaternion::rotation_y(PI / 4.0))
                .translate(-0.5)
                .draw_mesh(
                    &self.item_mesh[stack.iid],
                    &ctx.assets().blocks,
                );
        }
        // draw count text
        if let Some(count_text) = text_cache.count_text.as_mut() {
            count_text.draw(
                (size + 2.0 * scale).into(),
                scale,
                canvas,
                &ctx.global.renderer.borrow(),
            );
        }
        if is_cursor_over {
            // draw highlight
            const SELECTED_ALPHA: f32 = (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32);
            canvas.reborrow()
                .color([1.0, 1.0, 1.0, SELECTED_ALPHA])
                .draw_solid(size);

            // draw name tag
            if let (
                Some(name_pos),
                Some(name_text),
            ) = (ctx.cursor_pos, text_cache.name_text.as_mut()) {
                const NAME_TAG_BG_ALPHA: f32 = (0xc6 as f32 - 0x31 as f32) / 0xc6 as f32;
                let [
                    name_text_min,
                    mut name_text_max,
                ] = name_text.content_bounds(None, scale, &*ctx.global.renderer.borrow());
                
                let px_adjust = SLOT_DEFAULT_TEXT_SIZE * scale / 8.0;
                name_text_max += Vec2::from(px_adjust);

                let mut name_pos = ctx.cursor_pos.unwrap();
                name_pos -= name_pos % (2.0 * scale);
                name_pos += Vec2::new(18.0, -31.0) * scale;
                name_pos -= name_text_min;

                let border = px_adjust * 3.0;

                let name_tag_size = name_text_max - name_text_min + 2.0 * border;

                let mut canvas = canvas.reborrow()
                    .translate(name_pos);

                // name tag background
                canvas.reborrow()
                    .color([0.0, 0.0, 0.0, NAME_TAG_BG_ALPHA])
                    .draw_solid(name_tag_size);

                // name tag text
                name_text.draw(
                    0.0.into(),
                    scale,
                    &mut canvas.reborrow().translate(border),
                    &*ctx.global.renderer.borrow(),
                )
            }
        }
    }
}

impl ItemSlotTextCache {
    pub fn new() -> Self {
        Default::default()
    }
}
