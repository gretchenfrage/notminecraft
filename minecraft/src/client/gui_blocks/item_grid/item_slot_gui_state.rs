
use crate::{
    gui::prelude::*,
    item::*,
    client::{
        gui_blocks::item_grid::{
            SLOT_DEFAULT_TEXT_SIZE,
            borrow_item_slot::BorrowItemSlot,
            layout_calcs::ItemSlotLayoutCalcs,
            ItemGridLayoutCalcs,
        },
        meshing::item_mesh::ItemMesh,
    },
    game_data::per_item::PerItem,
};
use graphics::prelude::*;
use std::f32::consts::*;
use vek::*;


#[derive(Debug)]
pub struct ItemNameDrawer<'a, I> {
    pub borrow_slot: I,
    pub cached_iid: &'a mut Option<RawItemId>,
    pub name_text: &'a mut Option<GuiTextBlockInner>,
}

impl<'a, I: BorrowItemSlot> ItemNameDrawer<'a, I> {
    pub fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
    ) {
        const NAME_TAG_BG_ALPHA: f32 = (0xc6 as f32 - 0x31 as f32) / 0xc6 as f32;

        let ItemNameDrawer { mut borrow_slot, cached_iid, name_text } = self;

        let mut slot_guard = borrow_slot.borrow();
        let slot = I::deref(&mut slot_guard);

        // revalidate name text
        let iid = slot.as_ref().map(|stack| stack.iid);
        if *cached_iid != iid {
            *cached_iid = iid;
            *name_text = iid.map(|iid| GuiTextBlockInner::new(
                &GuiTextBlockConfig {
                    text: ctx.game().items_name[iid]
                        .map(|lang_key| &ctx.assets().lang[lang_key])
                        .unwrap_or_else(|| &ctx.game().items_machine_name[iid]),
                    font: ctx.assets().font,
                    logical_font_size: SLOT_DEFAULT_TEXT_SIZE,
                    color: Rgba::white(),
                    h_align: HAlign::Left,
                    v_align: VAlign::Top,
                },
                false,
            ));
        }

        // draw name tag
        if let Some(name_text) = name_text.as_mut() {
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

pub fn draw_item_noninteractive<'a>(
    ctx: GuiSpatialContext<'a>,
    canvas: &mut Canvas2<'a, '_>,
    scale: f32,
    layout: &ItemSlotLayoutCalcs,
    stack: Option<&ItemStack>,
    slot_state: &'a mut ItemSlotGuiStateNoninteractive,
    items_mesh: &'a PerItem<ItemMesh>,
) {
    // revalidate count text
    let count = stack
        .map(|stack| stack.count.get())
        .filter(|&n| n > 1);
    if count != slot_state.cached_count {
        slot_state.cached_count = count;
        slot_state.count_text = count
            .map(|n| GuiTextBlockInner::new(
                &GuiTextBlockConfig {
                    text: &n.to_string(),
                    font: ctx.assets().font,
                    logical_font_size: SLOT_DEFAULT_TEXT_SIZE,
                    color: Rgba::white(),
                    h_align: HAlign::Right,
                    v_align: VAlign::Bottom,
                },
                false,
            ));
    }

    if let Some(stack) = stack {
        // draw item mesh
        let mesh_size = layout.slot_inner_size * 1.1;
        canvas.reborrow()
            .translate(layout.slot_outer_size / 2.0)
            .scale(mesh_size)
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
            .scale(0.56)
            .rotate(Quaternion::rotation_x(-PI * 0.17))
            .rotate(Quaternion::rotation_y(PI / 4.0))
            .translate(-0.5)
            .draw_mesh(
                &items_mesh[stack.iid].mesh,
                &ctx.assets().blocks,
            );

        // draw count text
        if let Some(count_text) = slot_state.count_text.as_mut() {
            count_text.draw(
                layout.slot_outer_size.into(),
                scale,
                canvas,
                &ctx.global.renderer.borrow(),
            );
        }
    }
}

pub trait ItemSlotGuiStateGeneral<'a, I> {
    type DrawCursorOverState;

    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
        layout: &ItemGridLayoutCalcs,
        borrow_slot: I,
        items_mesh: &'a PerItem<ItemMesh>,
    ) -> Self::DrawCursorOverState;

    fn draw_cursor_over(
        state: Self::DrawCursorOverState,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
        xy: Vec2<u32>,
        layout: &ItemGridLayoutCalcs,
    );
}

impl<'a, I: BorrowItemSlot> ItemSlotGuiStateGeneral<'a, I> for &'a mut ItemSlotGuiState {
    type DrawCursorOverState = ItemNameDrawer<'a, I>;

    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
        layout: &ItemGridLayoutCalcs,
        mut borrow_slot: I,
        items_mesh: &'a PerItem<ItemMesh>,
    ) -> Self::DrawCursorOverState {
        {
            let mut slot_guard = borrow_slot.borrow();
            let slot = I::deref(&mut slot_guard); 
            draw_item_noninteractive(
                ctx,
                canvas,
                scale,
                &layout.inner,
                slot.as_ref(),
                &mut self.inner,
                items_mesh,
            );
        }
        ItemNameDrawer {
            borrow_slot,
            cached_iid: &mut self.cached_iid,
            name_text: &mut self.name_text,
        }
    }

    fn draw_cursor_over(
        state: Self::DrawCursorOverState,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
        xy: Vec2<u32>,
        layout: &ItemGridLayoutCalcs,
    ) {
        const SELECTED_ALPHA: f32 = (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32);
            
        // slot "moused over" highlight
        canvas.reborrow()
            .translate(xy.map(|n| n as f32) * layout.inner.slot_outer_size)
            .translate(layout.inner.pad_size)
            .color([1.0, 1.0, 1.0, SELECTED_ALPHA])
            .draw_solid(layout.inner.slot_inner_size);

        state.draw(
            ctx,
            canvas,
            scale,
        );
    }
}

impl<'a, I: BorrowItemSlot> ItemSlotGuiStateGeneral<'a, I> for &'a mut ItemSlotGuiStateNoninteractive {
    type DrawCursorOverState = ();

    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        scale: f32,
        layout: &ItemGridLayoutCalcs,
        mut borrow_slot: I,
        items_mesh: &'a PerItem<ItemMesh>,
    ) -> Self::DrawCursorOverState {
        let mut slot_guard = borrow_slot.borrow();
        let slot = I::deref(&mut slot_guard);
        draw_item_noninteractive(
            ctx,
            canvas,
            scale,
            &layout.inner,
            slot.as_ref(),
            self,
            items_mesh,
        );
    }

    fn draw_cursor_over(
        _state: Self::DrawCursorOverState,
        _ctx: GuiSpatialContext<'a>,
        _canvas: &mut Canvas2<'a, '_>,
        _scale: f32,
        _xy: Vec2<u32>,
        _layout: &ItemGridLayoutCalcs,
    ) {}
}

#[derive(Debug)]
pub struct ItemSlotGuiStateNoninteractive {
    cached_count: Option<u8>,
    count_text: Option<GuiTextBlockInner>,
}

impl ItemSlotGuiStateNoninteractive {
    pub fn new() -> Self {
        ItemSlotGuiStateNoninteractive {
            cached_count: None,
            count_text: None,
        }
    }
}

#[derive(Debug)]
pub struct ItemSlotGuiState {
    inner: ItemSlotGuiStateNoninteractive,

    cached_iid: Option<RawItemId>,
    name_text: Option<GuiTextBlockInner>,
}

impl ItemSlotGuiState {
    pub fn new() -> Self {
        ItemSlotGuiState {
            inner: ItemSlotGuiStateNoninteractive::new(),

            cached_iid: None,
            name_text: None,
        }
    }
}

