
use crate::{
    item::{
        RawItemId,
        ItemStack,
        ItemInstance,
    },
    gui::{
        blocks::{
            simple_gui_block::*,
            *,
        },
        *,
    },
    game_data::GameData,
    asset::{
        Assets,
        meshes::ItemMeshMesh,
    },
};
use graphics::{
    frame_content::{
        Canvas2,
        HAlign,
        VAlign,
    },
    modifier::Transform2,
};
use std::{
    cell::RefCell,
    f32::consts::PI,
    iter::from_fn,
    mem::swap,
};
use vek::*;


const DEFAULT_LOGICAL_SLOT_SIZE: f32 = 36.0;
const DEFAULT_LOGICAL_FONT_SIZE: f32 = 16.0;


/// Holds item game data for a gui item slot.
#[derive(Debug, Default)]
pub struct ItemSlot(pub RefCell<Option<ItemStack>>);

impl ItemSlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_vec(count: usize) -> Vec<Self> {
        from_fn(|| Some(Self::new())).take(count).collect()
    }
}


/// Holds cached state for rendering a gui item slot.
#[derive(Debug, Default)]
pub struct ItemSlotGui {
    count_text: Option<(u16, GuiTextBlock)>,
    name_text: Option<(RawItemId, GuiTextBlock)>,
}

impl ItemSlotGui {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_vec(count: usize) -> Vec<Self> {
        from_fn(|| Some(Self::new())).take(count).collect()
    }
}


/// GUI block representing a grid of item slots.
pub fn item_grid<'a>(
    cols: u32,
    slots: &'a [ItemSlot],
    guis: &'a mut [ItemSlotGui],
    held_item: Option<&'a ItemSlot>,
    config: ItemGridConfig,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
    assert_ne!(cols, 0, "item grid must have positive number of cols");
    assert_eq!(slots.len(), guis.len(), "item grid must equal num slots and guis");
    ItemGridGuiBlock {
        layout: Layout {
            slot_size: DEFAULT_LOGICAL_SLOT_SIZE,
            gap: config.logical_gap,
            border: 2.0,
            cols,
            slots: slots.len() as u32,
        },
        slots,
        guis,
        scale_mesh: config.scale_mesh,
        held_item,
        font_scale: 1.0,
    }
}

#[derive(Debug)]
pub struct ItemGridConfig {
    pub logical_gap: f32,
    pub scale_mesh: f32,
}

impl Default for ItemGridConfig {
    fn default() -> Self {
        ItemGridConfig {
            logical_gap: 0.0,
            scale_mesh: 1.0,
        }
    }
}


#[derive(Debug)]
struct ItemGridGuiBlock<'a> {
    layout: Layout,
    slots: &'a [ItemSlot],
    guis: &'a mut [ItemSlotGui],
    scale_mesh: f32,
    held_item: Option<&'a ItemSlot>,
    font_scale: f32, // TODO: janky
}

// factored out for borrowing reasons
#[derive(Debug)]
struct Layout {
    slot_size: f32,
    gap: f32,
    border: f32,
    cols: u32,
    slots: u32,
}

impl Layout {
    fn rows(&self) -> u32 {
        self.slots / self.cols
            + if self.slots % self.cols != 0 { 1 } else { 0 }
    }

    fn width(&self) -> f32 {
        self.cols as f32 * self.slot_size
            + self.cols.saturating_sub(1) as f32 * self.gap
    }

    fn height(&self) -> f32 {
        let rows = self.rows();
        rows as f32 * self.slot_size
            + rows.saturating_sub(1) as f32 * self.gap
    }

    fn size(&self) -> Extent2<f32> {
        Extent2::new(self.width(), self.height())
    }

    fn slot_transform(&self, coords: Vec2<u32>) -> Transform2 {
        let transl = coords.map(|n| n as f32) * (self.slot_size + self.gap);
        let transf = Transform2::translate(transl);
        transf
    }

    /// If the given position is in an item slot in the grid, return that
    /// slot's index and grid coordinates.
    fn slot_at(&self, pos: Vec2<f32>) -> Option<(usize, Vec2<u32>)> {
        let slot_gap_size = self.slot_size + self.gap;

        if !pos.are_all_positive() {
            // negative
            return None;
        }

        let coords = (pos / slot_gap_size).map(|n| n as u32);

        if coords.x >= self.cols {
            // too far to the right
            return None;
        }

        let index = coords.y * self.cols + coords.x;

        if index >= self.slots {
            // too far down in its column
            return None;
        }

        for n in pos {
            if n % slot_gap_size > self.slot_size {
                // in a gap between slots
                return None;
            }
        }

        Some((index as usize, coords))
    }

    fn slot_cursor_at(&self, ctx: GuiSpatialContext) -> Option<(usize, Vec2<u32>)> {
        ctx.cursor_pos.and_then(|pos| self.slot_at(pos))
    }
}

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for ItemGridGuiBlock<'a> {
    type Sized = Self;

    fn size(
        mut self,
        ctx: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized)
    {
        self.layout.slot_size *= scale;
        self.layout.gap *= scale;
        self.layout.border *= scale;
        self.font_scale *= scale;

        (self.layout.width(), self.layout.height(), self)
    }
}

fn draw_item_mesh<'a>(
    item: &ItemInstance,
    size: f32,
    canvas: &mut Canvas2<'a, '_>,
    game: &GameData,
    assets: &'a Assets,
) {
    let imi = *game.items_mesh_index.get(item.iid);
    let item_mesh = &assets.item_meshes[imi];

    match item_mesh.mesh {
        ItemMeshMesh::Block(ref mesh) => {
            canvas.reborrow()
                .scale(size)
                .begin_3d(Mat4::new(
                    1.0,  0.0,  0.0, 0.5,
                    0.0, -1.0,  0.0, 0.5,
                    0.0,  0.0, 0.01, 0.5,
                    0.0,  0.0,  0.0, 1.0,
                ))
                .scale(0.56)
                .rotate(Quaternion::rotation_x(-PI * 0.17))
                .rotate(Quaternion::rotation_y(PI / 4.0))
                .translate(-0.5)
                .draw_mesh(
                    mesh,
                    &assets.blocks,
                );
        }
        ItemMeshMesh::Item(tex_index) => {
            canvas.reborrow()
                .translate(size / 18.0)
                .draw_image(
                    &assets.items,
                    tex_index,
                    size * (16.0 / 18.0),
                );
        }
    }

}

impl<'a> GuiNode<'a> for ItemGridGuiBlock<'a> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        self.layout.slot_cursor_at(ctx).is_some()
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let (index_cursor_at, slot_cursor_at) = self.layout.slot_cursor_at(ctx).unzip();

        let mut coords = Vec2::new(0, 0);
        let mut name_text = None;

        for (slot, slot_gui) in self.slots.iter().zip(self.guis.iter_mut())
        {
            let transf = self.layout.slot_transform(coords);

            if let Some(stack) = slot.0.borrow().as_ref() {
                // draw item
                draw_item_mesh(
                    &stack.item,
                    self.layout.slot_size,
                    &mut canvas.reborrow()
                        .modify(transf)
                        .translate(self.layout.slot_size * 0.5)
                        .scale(self.scale_mesh)
                        .translate(self.layout.slot_size * -0.5),
                    ctx.game(),
                    ctx.assets(),
                );

                if stack.count.get() > 1 {
                    // draw item count
                    if slot_gui.count_text.as_ref()
                        .map(|&(cached_count, _)| cached_count != stack.count.get())
                        .unwrap_or(true)
                    {
                        slot_gui.count_text = Some((
                            stack.count.get(),
                            GuiTextBlock::new(&GuiTextBlockConfig {
                                text: &stack.count.get().to_string(),
                                font: ctx.assets().font,
                                logical_font_size: DEFAULT_LOGICAL_FONT_SIZE,
                                color: Rgba::white(),
                                h_align: HAlign::Right,
                                v_align: VAlign::Bottom,
                                wrap: false,
                            }),
                        ));
                    }

                    slot_gui.count_text.as_mut().unwrap()
                        .1
                        .draw(
                            self.layout.slot_size,
                            self.font_scale,
                            &mut canvas.reborrow()
                                .modify(transf),
                            &ctx.global.renderer.borrow(),
                        );
                }
            }

            if self.held_item.is_some() && slot_cursor_at == Some(coords) {
                // 0xff*a + 0x8b*(1 - a) = 0xc5
                // 0xff*a + 0x8b - 0x8b*a = 0xc5
                // 0xff*a - 0x8b*a = 0xc5 - 0x8b
                // (0xff - 0x8b)*a = 0xc5 - 0x8b
                // a = (0xc5 - 0x8b) / (0xff - 0x8b)

                // draw white "selected" overlay indicator
                canvas.reborrow()
                    .modify(transf)
                    .color([
                        1.0, 1.0, 1.0,
                        (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32),
                    ])
                    .translate(self.layout.border)
                    .draw_solid(self.layout.slot_size - self.layout.border);

                // draw item name afterwards
                if self.held_item.as_ref().unwrap().0.borrow().is_none() {
                    name_text = Some(&mut slot_gui.name_text);
                }
            }

            coords.x += 1;
            if coords.x == self.layout.cols {
                coords.x = 0;
                coords.y += 1;
            }
        }

        // draw item name hovered over
        if let Some(name_text) = name_text {
            if let Some(stack) = self.slots[index_cursor_at.unwrap()].0.borrow().as_ref() {
                // revalidate
                if name_text.as_ref()
                    .map(|&(cached_iid, _)| cached_iid != stack.item.iid)
                    .unwrap_or(true)
                {
                    let imi = *ctx.game().items_mesh_index.get(stack.item.iid);
                    let name = &ctx.assets().item_meshes[imi].name;

                    *name_text = Some((
                        stack.item.iid,
                        GuiTextBlock::new(&GuiTextBlockConfig {
                            text: name,
                            font: ctx.assets().font,
                            logical_font_size: DEFAULT_LOGICAL_FONT_SIZE,
                            color: Rgba::white(),
                            h_align: HAlign::Left,
                            v_align: VAlign::Top,
                            wrap: false,
                        }),
                    ));
                }

                // layout
                let [
                    mut name_text_min,
                    mut name_text_max,
                ] = name_text.as_mut().unwrap()
                    .1
                    .content_bounds(
                        0.0,
                        self.font_scale,
                        &ctx.global.renderer.borrow(),
                    );

                let px_adjust = DEFAULT_LOGICAL_FONT_SIZE * self.font_scale / 8.0;
                name_text_max += Vec2::from(px_adjust);
                
                let mut name_pos = ctx.cursor_pos.unwrap();
                name_pos -= name_pos % (2.0 * self.font_scale);
                name_pos += Vec2::new(18.0, -31.0) * self.font_scale;
                name_pos -= name_text_min;

                let border = px_adjust * 3.0;
                
                let name_tag_size =
                    name_text_max - name_text_min
                    + 2.0 * border;

                let mut canvas = canvas.reborrow()
                    .translate(name_pos);
                
                // name tag darkened background
                canvas.reborrow()
                    .color([
                        0.0, 0.0, 0.0,
                        (0xc6 as f32 - 0x31 as f32) / 0xc6 as f32,
                    ])
                    .draw_solid(name_tag_size);

                // actual item name
                name_text.as_mut().unwrap()
                    .1
                    .draw(
                        0.0,
                        self.font_scale,
                        &mut canvas.reborrow()
                            .translate(border),
                        &ctx.global.renderer.borrow(),
                    );
            }
        }

    }

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if let Some((index, _)) = self.layout.slot_cursor_at(ctx) {
            let mut slot = self.slots[index].0.borrow_mut();
            if button == MouseButton::Middle {
                let iid =
                    if ctx.global.pressed_keys_semantic.contains(&VirtualKeyCode::G) {
                        ctx.game().iid_stick
                    } else {
                        ctx.game().iid_stone
                    };
                if let Some(stack) = slot.as_mut() {
                    if stack.item.iid == iid && stack.count.get() < 64 {
                        stack.count = (stack.count.get() + 1).try_into().unwrap();
                    }
                } else {
                    *slot = Some(ItemStack::one(ItemInstance::new(
                        iid,
                        (),
                    )));
                }
            } else if button == MouseButton::Left {
                if let Some(ref held_item_slot) = self.held_item {
                    let mut transferred = false;

                    let mut opt_held_stack = held_item_slot.0.borrow_mut();
                    if let (
                        Some(stack),
                        Some(held_stack),
                    ) = (
                        slot.as_mut(),
                        opt_held_stack.as_mut(),
                    ) {
                        if stack.item.iid == held_stack.item.iid {
                            let transfer = u16::min(
                                64 - stack.count.get(),
                                held_stack.count.get(),
                            );
                            stack.count = (stack.count.get() + transfer).try_into().unwrap();
                            if let Ok(n) = (held_stack.count.get() - transfer).try_into() {
                                held_stack.count = n;
                            } else {
                                *opt_held_stack = None;
                            }
                            transferred = true;
                        }
                    }

                    if !transferred {
                        swap(
                            &mut *slot,
                            &mut *opt_held_stack,
                        );
                    }
                }
            } else if button == MouseButton::Right {
                if let Some(ref held_item_slot) = self.held_item {
                    let mut opt_held_stack = held_item_slot.0.borrow_mut();
                    if let Some(held_stack) = opt_held_stack.as_mut() {
                        let mut deposited = false;

                        if let Some(stack) = slot.as_mut() {
                            if stack.item.iid == held_stack.item.iid && stack.count.get() < 64 {
                                stack.count = (stack.count.get() + 1).try_into().unwrap();
                                deposited = true;
                            }
                        } else {
                            *slot = Some(ItemStack::one(ItemInstance {
                                iid: held_stack.item.iid,
                                meta: held_stack.item.meta.clone(),
                            }));
                            deposited = true;
                        }

                        if deposited {
                            if let Ok(n) = (held_stack.count.get() - 1).try_into() {
                                held_stack.count = n;
                            } else {
                                *opt_held_stack = None;
                            }
                        }
                    } else {
                        if let Some(stack) = slot.as_mut() {
                            let split_off = (stack.count.get() / 2)
                                .try_into()
                                .ok()
                                .map(|count| ItemStack {
                                    item: stack.item.clone(),
                                    count,
                                });
                            stack.count = (stack.count.get() / 2 + stack.count.get() % 2).try_into().unwrap();
                            *opt_held_stack = slot.take();
                            *slot = split_off;
                        }
                    }
                }
            }
        }
    }
}

pub fn held_item_gui<'a>(
    slot: &'a ItemSlot,
    slot_gui: &'a mut ItemSlotGui,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    HeldItemGuiBlock { slot, slot_gui }
}

#[derive(Debug)]
struct HeldItemGuiBlock<'a> {
    slot: &'a ItemSlot,
    slot_gui: &'a mut ItemSlotGui,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<HeldItemGuiBlock<'a>> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        if let (
            Some(stack),
            Some(mut cursor_pos),
        ) = (
            self.inner.slot.0.borrow().as_ref(),
            ctx.cursor_pos,
        ) {
            let slot_size = DEFAULT_LOGICAL_SLOT_SIZE * self.scale;
            cursor_pos -= cursor_pos % (2.0 * self.scale);

            // draw item
            draw_item_mesh(
                &stack.item,
                slot_size,
                &mut canvas.reborrow()
                    .translate(-slot_size / 2.0)
                    .translate(cursor_pos),
                ctx.game(),
                ctx.assets(),
            );

            if stack.count.get() > 1 {
                // draw item count
                // TODO: this should be deduplicated
                if self.inner.slot_gui.count_text.as_ref()
                    .map(|&(cached_count, _)| cached_count != stack.count.get())
                    .unwrap_or(true)
                {
                    self.inner.slot_gui.count_text = Some((
                        stack.count.get(),
                        GuiTextBlock::new(&GuiTextBlockConfig {
                            text: &stack.count.get().to_string(),
                            font: ctx.assets().font,
                            logical_font_size: DEFAULT_LOGICAL_FONT_SIZE,
                            color: Rgba::white(),
                            h_align: HAlign::Right,
                            v_align: VAlign::Bottom,
                            wrap: false,
                        }),
                    ));
                }

                self.inner.slot_gui.count_text.as_mut().unwrap()
                    .1
                    .draw(
                        slot_size,
                        self.scale,
                        &mut canvas.reborrow()
                            .translate(-slot_size / 2.0)
                            .translate(cursor_pos),
                        &ctx.global.renderer.borrow(),
                    );
            }
        }
    }
}
