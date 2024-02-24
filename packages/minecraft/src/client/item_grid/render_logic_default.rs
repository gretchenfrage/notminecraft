
use super::*;
use crate::{
    game_data::per_item::PerItem,
    item::*,
    gui::prelude::*,
};
use std::f32::consts::PI;


const SLOT_DEFAULT_TEXT_SIZE: f32 = 16.0;


/// Type which can be owned for a frame to render a single item slot.
pub trait ItemSlotRenderer<'a> {
    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        item_slot_idx: usize,
        item_slot: &'a Option<ItemStack>,
        size: f32,
        scale: f32,
        is_cursor_over: bool,
        item_mesh: &'a PerItem<Mesh>,
        held_item: &'a Option<ItemStack>,
    );
}

/// Cache for the layed-out text for rendering an item slot.
///
/// Mut ref implements `ItemSlotRenderer`.
#[derive(Debug, Default)]
pub struct ItemSlotTextCache {
    count_text: CountTextCache,
    name_text: NameTextCache,
}

/// Cache for the layout-out text for rendering an item slot, excluding hover text.
#[derive(Debug, Default)]
pub struct ItemSlotTextCacheNonhoverable {
    count_text: CountTextCache,
}

/// Reasonable-defaults `ItemGridRenderLogic` implementation.
pub fn item_grid_default_render_logic<'a, I>(
    item_mesh: &'a PerItem<Mesh>,
    held_item: &'a Option<ItemStack>,
    slot_renderers: I,
) -> impl ItemGridRenderLogic<'a, Option<ItemStack>>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Debug,
    <I as IntoIterator>::Item: ItemSlotRenderer<'a>,
{
    ItemGridDefaultRenderLogic {
        item_mesh,
        held_item,
        slot_renderers: slot_renderers.into_iter(),
        next_item_slot_idx: 0,
    }
}

#[derive(Debug)]
struct ItemGridDefaultRenderLogic<'a, I> {
    item_mesh: &'a PerItem<Mesh>,
    held_item: &'a Option<ItemStack>,
    slot_renderers: I,
    next_item_slot_idx: usize,
}

#[derive(Debug, Default)]
struct CountTextCache {
    cached_count: Option<u8>,
    count_text: Option<GuiTextBlockInner>,
}

#[derive(Debug, Default)]
struct NameTextCache {
    cached_iid: Option<Option<RawItemId>>,
    name_text: Option<GuiTextBlockInner>,
}

impl<
    'a,
    I: Iterator + Debug,
> ItemGridRenderLogic<'a, Option<ItemStack>> for ItemGridDefaultRenderLogic<'a, I>
where
    <I as Iterator>::Item: ItemSlotRenderer<'a>,
{
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
        debug_assert!(self.next_item_slot_idx <= item_slot_idx);
        while self.next_item_slot_idx < item_slot_idx {
            self.slot_renderers.next().expect("wrong number of slot renderers");
            self.next_item_slot_idx += 1;
        }
        self.slot_renderers.next().expect("wrong number of slot renderers").draw(
            ctx,
            canvas,
            item_slot_idx,
            item_slot,
            size,
            scale,
            is_cursor_over,
            self.item_mesh,
            self.held_item,
        );
        self.next_item_slot_idx += 1;
    }
}

impl<'a> ItemSlotRenderer<'a> for &'a mut ItemSlotTextCache {
    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        _item_slot_idx: usize,
        item_slot: &'a Option<ItemStack>,
        size: f32,
        scale: f32,
        is_cursor_over: bool,
        item_mesh: &'a PerItem<Mesh>,
        held_item: &'a Option<ItemStack>,
    ) {
        let count_text = self.count_text.validate(item_slot, ctx);
        let name_text = self.name_text.validate(item_slot, ctx);
        draw_item_slot(
            count_text,
            name_text.filter(|_| is_cursor_over && held_item.is_none()),
            ctx,
            canvas,
            item_slot,
            size,
            scale,
            item_mesh,
            is_cursor_over,
        );
    }
}

// actual function that draws it after all text validation and everything else is dealt with
fn draw_item_slot<'a>(
    count_text: Option<&'a mut GuiTextBlockInner>,
    name_text: Option<&'a mut GuiTextBlockInner>,
    ctx: GuiSpatialContext<'a>,
    canvas: &mut Canvas2<'a, '_>,
    item_slot: &'a Option<ItemStack>,
    size: f32,
    scale: f32,
    item_mesh: &'a PerItem<Mesh>,
    draw_highlight: bool,
) {
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
                &item_mesh[stack.iid],
                &ctx.assets().blocks,
            );
    }
    // draw count text
    if let Some(count_text) = count_text {
        count_text.draw(
            (size + 2.0 * scale).into(),
            scale,
            canvas,
            &ctx.global.renderer.borrow(),
        );
    }
    // draw highlight
    if draw_highlight {
        const SELECTED_ALPHA: f32 = (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32);
        canvas.reborrow()
            .color([1.0, 1.0, 1.0, SELECTED_ALPHA])
            .draw_solid(size);
    }
    // draw name tag
    if let (
        Some(mut name_pos),
        Some(name_text),
    ) = (ctx.cursor_pos, name_text) {
        const NAME_TAG_BG_ALPHA: f32 = (0xc6 as f32 - 0x31 as f32) / 0xc6 as f32;
        let [
            name_text_min,
            mut name_text_max,
        ] = name_text.content_bounds(None, scale, &*ctx.global.renderer.borrow());
        
        let px_adjust = SLOT_DEFAULT_TEXT_SIZE * scale / 8.0;
        name_text_max += Vec2::from(px_adjust);

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

impl ItemSlotTextCache {
    pub fn new() -> Self {
        Default::default()
    }
}

impl ItemSlotTextCacheNonhoverable {
    pub fn new() -> Self {
        Default::default()
    }

    /// Use this to produce a gui block which renders the held item.
    pub fn held_item_gui_block<'a>(
        &'a mut self,
        item_mesh: &'a PerItem<Mesh>,
        held_item: &'a Option<ItemStack>,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        HeldItemGuiBlock { text_cache: self, item_mesh, held_item }
    }
}

#[derive(Debug)]
struct HeldItemGuiBlock<'a> {
    text_cache: &'a mut ItemSlotTextCacheNonhoverable,
    item_mesh: &'a PerItem<Mesh>,
    held_item: &'a Option<ItemStack>,
}

impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for HeldItemGuiBlock<'a> {
    type Sized = HeldItemGuiBlockSized<'a>;

    fn size(
        self,
        _ctx: &GuiGlobalContext,
        _w_in: f32,
        _h_in: f32,
        scale: f32,
    ) -> ((), (), Self::Sized) {
        ((), (), HeldItemGuiBlockSized { inner: self, scale })
    }
}

#[derive(Debug)]
struct HeldItemGuiBlockSized<'a> {
    inner: HeldItemGuiBlock<'a>,
    scale: f32,
}

impl<'a> GuiNode<'a> for HeldItemGuiBlockSized<'a> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let count_text = self.inner.text_cache.count_text.validate(self.inner.held_item, ctx);
        if let Some(pos) = ctx.cursor_pos {
            let size = DEFAULT_SLOT_LOGICAL_SIZE * self.scale;
            let transl = Transform2::translate(pos - size / 2.0);
            draw_item_slot(
                count_text,
                None,
                {
                    let mut ctx = ctx;
                    ctx.relativize(transl);
                    ctx
                },
                &mut canvas.reborrow().modify(transl),
                self.inner.held_item,
                size,
                self.scale,
                self.inner.item_mesh,
                false,
            );
        }
    }
}

impl CountTextCache {
    fn validate(
        &mut self,
        item_slot: &Option<ItemStack>,
        ctx: GuiSpatialContext,
    ) -> Option<&mut GuiTextBlockInner> {
        let count = item_slot.as_ref().map(|stack| stack.count.get()).unwrap_or(0);
        if self.cached_count != Some(count) {
            self.cached_count = Some(count);
            self.count_text = if count > 1 {
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
        self.count_text.as_mut()
    }
}

impl NameTextCache {
    fn validate(
        &mut self,
        item_slot: &Option<ItemStack>,
        ctx: GuiSpatialContext,
    ) -> Option<&mut GuiTextBlockInner> {
        let iid = item_slot.as_ref().map(|stack| stack.iid);
        if self.cached_iid != Some(iid) {
            self.cached_iid = Some(iid);
            self.name_text = iid.map(|iid| {
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
        self.name_text.as_mut()
    }
}
