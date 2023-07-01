
use crate::{
    item::{
        ItemStack,
        ItemInstance,
    },
    gui::{
        blocks::{
            simple_gui_block::SimpleGuiBlock,
            *,
        },
        *,
    },
    game_data::GameData,
    asset::Assets,
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
};
use vek::*;


const DEFAULT_LOGICAL_SLOT_SIZE: f32 = 36.0;


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
        interactable: config.interactable,
        font_scale: 1.0,
    }
}

#[derive(Debug)]
pub struct ItemGridConfig {
    pub logical_gap: f32,
    pub scale_mesh: f32,
    pub interactable: bool,
}

impl Default for ItemGridConfig {
    fn default() -> Self {
        ItemGridConfig {
            logical_gap: 0.0,
            scale_mesh: 1.0,
            interactable: true,
        }
    }
}


#[derive(Debug)]
struct ItemGridGuiBlock<'a> {
    layout: Layout,
    slots: &'a [ItemSlot],
    guis: &'a mut [ItemSlotGui],
    scale_mesh: f32,
    interactable: bool,
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
    let mut canvas = canvas.reborrow()
        .scale(size)
        .begin_3d(Mat4::new(
            1.0,  0.0,  0.0, 0.5,
            0.0, -1.0,  0.0, 0.5,
            0.0,  0.0, 0.01, 0.5,
            0.0,  0.0,  0.0, 1.0,
        ));
    if item_mesh.block {
        canvas = canvas
            .scale(0.56)
            .rotate(Quaternion::rotation_x(-PI * 0.17))
            .rotate(Quaternion::rotation_y(PI / 4.0))
            .translate(-0.5);
    }
    canvas.reborrow()
        .draw_mesh(
            &item_mesh.mesh,
            &assets.blocks,
        );
}

impl<'a> GuiNode<'a> for ItemGridGuiBlock<'a> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        self.layout.slot_cursor_at(ctx).is_some()
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let mut coords = Vec2::new(0, 0);

        for (slot, slot_gui) in self.slots.iter().zip(self.guis.iter_mut())
        {
            let transf = self.layout.slot_transform(coords);

            if let Some(stack) = slot.0.borrow().as_ref() {
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


                if slot_gui.count_text.as_ref()
                    .map(|&(cached_count, _)| cached_count != stack.count.get())
                    .unwrap_or(true)
                {
                    slot_gui.count_text = Some((
                        stack.count.get(),
                        GuiTextBlock::new(&GuiTextBlockConfig {
                            text: &stack.count.get().to_string(),
                            font: ctx.assets().font,
                            logical_font_size: 16.0,
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

            coords.x += 1;
            if coords.x == self.layout.cols {
                coords.x = 0;
                coords.y += 1;
            }
        }

        if self.interactable {
            // 0xff*a + 0x8b*(1 - a) = 0xc5
            // 0xff*a + 0x8b - 0x8b*a = 0xc5
            // 0xff*a - 0x8b*a = 0xc5 - 0x8b
            // (0xff - 0x8b)*a = 0xc5 - 0x8b
            // a = (0xc5 - 0x8b) / (0xff - 0x8b)

            if let Some((_, coords)) = self.layout.slot_cursor_at(ctx) {
                let transf = self.layout.slot_transform(coords);

                canvas.reborrow()
                    .modify(transf)
                    .color([
                        1.0, 1.0, 1.0,
                        (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32),
                    ])
                    .translate(self.layout.border)
                    .draw_solid(self.layout.slot_size - self.layout.border);
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
        if button != MouseButton::Middle { return }

        if let Some((index, _)) = self.layout.slot_cursor_at(ctx) {
            let mut slot = self.slots[index].0.borrow_mut();
            if let Some(stack) = slot.as_mut() {
                if stack.item.iid == ctx.game().iid_stone && stack.count.get() < 64 {
                    stack.count = (stack.count.get() + 1).try_into().unwrap();
                }
            } else {
                *slot = Some(ItemStack::one(ItemInstance::new(
                    ctx.game().iid_stone,
                    (),
                )));
            }
        }
    }
}

/*
/*
macro_rules! forward_or_backward {
    ($forward:expr {$(
        $s:stmt;
    )*}=>{
        if $forward {$(
            $s;
        )*} else {
            forward_or_backward!(@rev {$(
                $s;
            )*} {});
        }
    };
    (@rev {} {$(
        $accum:stmt;
    )*})=>{{$(
        $accum;
    )*}};
    (@rev {
        $head:stmt;
        $( $tail:stmt; )*
    } {$(
        $accum:stmt;
    )*})=>{
        forward_or_backward!(@rev {$(
            $tail;
        )*} {
            $( $accum; )*
            $head;
        })
    };
}
*/
impl<'a> SizedGuiBlock<'a> for ItemGridGuiBlock<'a> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        /*
        forward_or_backward!(forward {

        });
        */

    }
}


/// Gui node to render a single item and implementing clickcing behavior,
/// but doesn't handle its text.
struct 
*/


/*
pub fn gui_slot_grid<F, E, const LEN: usize>(
    slots: &[ItemSlot; LEN],
    dims: E,
    config: F,
) -> impl GuiBlockSeq<DimChildSets, DimParentSets>
where
    F: FnMut() -> SlotGuiConfig,
    E: Into<Extent2<u32>>,
{
    let dims = dims.into();
    slots
}*/
//pub struct ItemGridBuilder<

/*
pub const DEFAULT_SLOT_SIZE: f32 = 36.0;


#[derive(Debug, Clone)]
pub struct SlotGuiConfig {
    pub interactable: bool,
}

impl SlotGuiConfig {
    pub fn new() -> Self {
        SlotGuiConfig {
            interactable: true,
        }
    }

    pub fn non_interactable(mut self) -> Self {
        self.interactable = false;
        self
    }
}

impl Default for SlotGuiConfig {
    fn default() -> Self {
        SlotGuiConfig::new()
    }
}


#[derive(Debug)]
pub struct ItemSlot(pub RefCell<Option<ItemStack>>);

impl ItemSlot {
    pub fn gui<'a>(
        &'a self,
        config: SlotGuiConfig,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        ItemSlotGuiBlock {
            slot: self,
            config,
        }
    }
}

impl Default for ItemSlot {
    fn default() -> Self {
        ItemSlot(RefCell::new(None))
    }
}


#[derive(Debug)]
struct ItemSlotGuiBlock<'a> {
    slot: &'a ItemSlot,
    config: SlotGuiConfig,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<ItemSlotGuiBlock<'a>> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        ctx.cursor_in_area(0.0, self.size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let view_proj = Mat4::new(
            1.0,  0.0,  0.0, 0.5,
            0.0, -1.0,  0.0, 0.5,
            0.0,  0.0, 0.01, 0.5,
            0.0,  0.0,  0.0, 1.0,
        );
        if let Some(stack) = self.inner.slot.0.borrow().as_ref() {
            let imi = *ctx.game().items_mesh_index.get(stack.item.iid);
            let item_mesh = &ctx.assets().item_meshes[imi];

            let mut canvas = canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj);

            if item_mesh.block {
                canvas = canvas
                    .scale(0.56)
                    .rotate(Quaternion::rotation_x(-PI * 0.17))
                    .rotate(Quaternion::rotation_y(PI / 4.0))
                    .translate(-0.5);
            }

            canvas.reborrow()
                .draw_mesh(
                    &item_mesh.mesh,
                    &ctx.assets().blocks,
                );
        }
        //canvas.reborrow()
        //    .scale(size)
        //    .begin_3d(view_proj)
        //    .scale(0.5)
        //    .rotate(Quaternion::rotation_x(-PI / 5.0))
        //    .rotate(Quaternion::rotation_y(PI / 4.0))
        //    .translate(-0.5)
        //    .draw_mesh(
        //        &ctx.assets().block_item_mesh,
        //        &ctx.assets().blocks,
        //    );
        if self.inner.config.interactable {
            if ctx.cursor_in_area(0.0, self.size) {
                canvas.reborrow()
                    .color([0.0, 0.0, 0.0, 0.25])
                    .draw_solid(self.size);
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
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Middle { return }

        let mut slot = self.inner.slot.0.borrow_mut();
        *slot = Some(ItemStack::one(ItemInstance::new(ctx.game().iid_stone, ())));
    }
}


/*
#[derive(Debug)]
struct ItemSlotGuiBlock<'a> {
    inner: &'a ItemSlot,
    size: f32,
    interactable: bool,
}

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for ItemSlotGuiBlock<'a> {
    type Sized = Self;

    fn size(
        mut self,
        _: &GuiGlobalContext<'a>,
        _w_in: (),
        _h_in: (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        self.size *= scale;
        (self.size, self.size, self)
    }
}

impl<'a> GuiNode<'a> for ItemSlotGuiBlock<'a> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        ctx.cursor_in_area(0.0, self.size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let view_proj = Mat4::new(
            1.0,  0.0,  0.0, 0.5,
            0.0, -1.0,  0.0, 0.5,
            0.0,  0.0, 0.01, 0.5,
            0.0,  0.0,  0.0, 1.0,
        );
        if let Some(stack) = self.inner.0.borrow().as_ref() {
            let imi = *ctx.game().items_mesh_index.get(stack.item.iid);
            canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj)
                .draw_mesh(
                    &ctx.assets().item_meshes[imi],
                    &ctx.assets().blocks,
                );
        }
        //canvas.reborrow()
        //    .scale(size)
        //    .begin_3d(view_proj)
        //    .scale(0.5)
        //    .rotate(Quaternion::rotation_x(-PI / 5.0))
        //    .rotate(Quaternion::rotation_y(PI / 4.0))
        //    .translate(-0.5)
        //    .draw_mesh(
        //        &ctx.assets().block_item_mesh,
        //        &ctx.assets().blocks,
        //    );
        if self.interactable {
            if ctx.cursor_in_area(0.0, self.size) {
                canvas.reborrow()
                    .color([0.0, 0.0, 0.0, 0.25])
                    .draw_solid(self.size);
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
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Middle { return }

        let mut slot = self.inner.0.borrow_mut();
        *slot = Some(ItemStack::one(ItemInstance::new(ctx.game().iid_stone, ())));
    }
}

/*
impl ItemSlot {
    pub fn gui<'a>(&'a mut self) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        logical_size(DEFAULT_UI_SIZE,
            
        )
    }
}
*/

//impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> {
//    
//}

/*
pub struct HeldItem {
    pub content: RefCell<Option<ItemStack>>,
}
*/
*/*/