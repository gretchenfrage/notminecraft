
use self::chat_input::ChatInput;
use crate::{
    gui::prelude::*,
    client::{
        gui_blocks::{
            item_grid::item_slot_gui_state::{
                ItemSlotGuiStateNoninteractive,
                ItemSlotGuiState,
            },
            chat::GuiChat,
        },
        meshing::{
            char_mesh::CharMesh,
            item_mesh::ItemMesh,
        },
        connection::Connection,
        InternalServer,
        PredictionToMake,
    },
    item::*,
    game_data::per_item::PerItem,
    settings::Settings,
    game_data::content::chest::ChestMenu,
};
use chunk_data::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
};
use vek::*;
use graphics::prelude::*;


pub mod chat_input;
pub mod esc_menu;
pub mod inventory;
pub mod settings;


pub const MENU_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0 - 0x6f as f32 / 0xde as f32];


#[derive(Debug)]
pub struct MenuResources {
    esc_menu_title_text: GuiTextBlock<true>,
    options_menu_title_text: GuiTextBlock<true>,
    exit_menu_button: MenuButton,
    exit_game_button: MenuButton,
    options_button: MenuButton,
    options_fog_button: OptionsOnOffButton,
    options_day_night_button: OptionsOnOffButton,
    options_load_dist_outline_button: OptionsOnOffButton,
    options_chunk_outline_button: OptionsOnOffButton,
    options_done_button: MenuButton,
    open_to_lan_button: MenuButton,

    effect_queue: MenuEffectQueue,
}

impl MenuResources {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let esc_menu_title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Game menu",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Bottom,
        });
        let options_menu_title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.options_title,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Bottom,
        });
        let exit_menu_button = menu_button("Back to game").build(ctx.assets);
        let exit_game_button = menu_button("Save and quit to title").build(ctx.assets);
        let options_button = menu_button(&ctx.assets.lang.menu_options).build(ctx.assets);
        let options_fog_button = OptionsOnOffButton::new("Fog");
        let options_day_night_button = OptionsOnOffButton::new("Day Night");
        let options_load_dist_outline_button = OptionsOnOffButton::new("Load Distance Outline");
        let options_chunk_outline_button = OptionsOnOffButton::new("Chunk Outline");
        let options_done_button = menu_button(&ctx.assets.lang.gui_done).build(ctx.assets);
        let open_to_lan_button = menu_button("Open to LAN").build(ctx.assets);

        MenuResources {
            esc_menu_title_text,
            options_menu_title_text,
            exit_menu_button,
            exit_game_button,
            options_button,
            options_fog_button,
            options_day_night_button,
            options_load_dist_outline_button,
            options_chunk_outline_button,
            options_done_button,
            open_to_lan_button,
            effect_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn process_effect_queue(&mut self, menu_stack: &mut Vec<Menu>) {
        while let Some(effect) = self.effect_queue.get_mut().pop_front() {
            match effect {
                MenuEffect::PopMenu => {
                    menu_stack.pop();
                }
                MenuEffect::PushMenu(menu) => {
                    menu_stack.push(menu);
                }
            }
        }
    }
}

#[derive(Debug)]
struct OptionsOnOffButton {
    name: String,
    button_on: Option<(MenuButton, bool)>,
}

impl OptionsOnOffButton {
    fn new(name: &str) -> Self {
        OptionsOnOffButton {
            name: name.to_owned(),
            button_on: None,
        }
    }

    fn gui<'a, F>(
        &'a mut self,
        ctx: &GuiGlobalContext, // TODO: "lazy block"
        mut settings_on: F,
    ) -> impl GuiBlock<'a, DimParentSets, DimChildSets>
    where
        F: FnMut(&mut Settings) -> &mut bool + 'a,
    {
        let on = *settings_on(&mut *ctx.settings.borrow_mut());
        if self.button_on.as_ref()
            .map(|&(_, cached_on)| cached_on != on)
            .unwrap_or(true)
        {
            let mut text = self.name.clone();
            text.push_str(": ");
            text.push_str(match on {
                true => &ctx.assets.lang.options_on,
                false => &ctx.assets.lang.options_off,
            });
            self.button_on = Some((menu_button(&text).build(ctx.assets), on));
        }

        self.button_on.as_mut().unwrap().0.gui(move |ctx| {
            {
                let mut settings = ctx.settings.borrow_mut();
                let on = settings_on(&mut *settings);
                *on = !*on;
            }
            ctx.save_settings();
        })
    }
}

pub type MenuEffectQueue = RefCell<VecDeque<MenuEffect>>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum MenuEffect {
    PopMenu,
    PushMenu(Menu),
}

/// Menu that can be opened over the world. Different from in-world GUIs. Form
/// a stack.
#[derive(Debug)]
pub enum Menu {
    EscMenu,
    Inventory,
    ChatInput(ChatInput),
    Settings,
    Chest(ChestMenu),
}

#[allow(unused_variables)]
pub struct MenuGuiParams<'a, 'b> {
    pub chat: &'b mut Option<&'a mut GuiChat>,
    pub internal_server: &'a mut Option<InternalServer>,
    pub items_mesh: &'a PerItem<ItemMesh>,
    pub connection: &'a Connection,
    pub predictions_to_make: &'a RefCell<VecDeque<PredictionToMake>>,

    pub held_item: &'a ItemSlot,
    pub held_item_state: &'a mut ItemSlotGuiStateNoninteractive,

    pub inventory_slots: &'a [ItemSlot; 36],
    pub inventory_slots_state: &'a mut Box<[ItemSlotGuiState; 36]>,

    pub inventory_slots_armor: &'a [ItemSlot; 4],
    pub inventory_slots_armor_state: &'a mut [ItemSlotGuiState; 4],
    
    pub inventory_slots_crafting: &'a [ItemSlot; 4],
    pub inventory_slots_crafting_state: &'a mut [ItemSlotGuiState; 4],

    pub inventory_slot_crafting_output: &'a ItemSlot,
    pub inventory_slot_crafting_output_state: &'a mut ItemSlotGuiState,

    pub char_mesh: &'a CharMesh,
    pub head_pitch: f32,
    pub pointing: bool,

    pub open_menu_msg_idx: Option<u64>,

    pub getter: &'b Getter<'b>,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,

    pub ctx: &'a GuiWindowContext<'a>,
}

impl Menu {
    pub fn gui<'a>(
        &'a mut self,
        args: MenuGuiParams<'a, '_>,
        resources: &'a mut MenuResources,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
        match self {
            &mut Menu::EscMenu => GuiEither::A(GuiEither::A(GuiEither::A(esc_menu::gui(args, resources)))),
            &mut Menu::Inventory => GuiEither::A(GuiEither::A(GuiEither::B(inventory::gui(args)))),
            &mut Menu::ChatInput(ref mut menu) => GuiEither::A(GuiEither::B(GuiEither::A(menu.gui(args)))),
            &mut Menu::Settings => GuiEither::A(GuiEither::B(GuiEither::B(settings::gui(args, resources)))),
            &mut Menu::Chest(ref mut menu) => GuiEither::B(menu.gui(args)),
        }
    }

    pub fn update(&mut self, ctx: &GuiGlobalContext) {
        match self {
            &mut Menu::ChatInput(ref mut menu) => menu.update(ctx),
            &mut Menu::EscMenu
            | &mut Menu::Inventory
            | &mut Menu::Settings
            | &mut Menu::Chest(_) => (),
        }
    }

    pub fn exitable_via_inventory_button(&self) -> bool {
        match self {
            &Menu::EscMenu => false,
            &Menu::Inventory => true,
            &Menu::ChatInput { .. } => false,
            &Menu::Settings => false,
            &Menu::Chest { .. } => true,
        }
    }

    pub fn has_darkened_background(&self) -> bool {
        match self {
            &Menu::ChatInput { .. } => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub struct MenuStack {
    stack: Vec<Menu>,
    resources: MenuResources, // TODO: merge resources into this?
}

impl MenuStack {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        MenuStack {
            stack: Vec::new(),
            resources: MenuResources::new(ctx),
        }
    }

    #[must_use]
    pub fn pop(&mut self) -> Option<Menu> {
        self.stack.pop()
    }

    pub fn push(&mut self, menu: impl Into<Menu>) {
        self.stack.push(menu.into());
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn top(&self) -> Option<&Menu> {
        self.stack.iter().rev().next()
    }

    pub fn top_mut(&mut self) -> Option<&mut Menu> {
        self.stack.iter_mut().rev().next()
    }

    pub fn update(&mut self, ctx: &GuiGlobalContext) {
        if let Some(menu) = self.stack.iter_mut().rev().next() {
            menu.update(ctx);
        }
    }

    pub fn gui<'a>(
        &'a mut self,
        args: MenuGuiParams<'a, '_>,
    ) -> Option<impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a> {
        const MENU_DARKENED_BACKGROUND_ALPHA: f32 = 1.0 - 0x2a as f32 / 0x97 as f32;
        self.stack.iter_mut().rev().next()
            .map(|menu| layer((
                if menu.has_darkened_background() {
                    Some(solid([0.0, 0.0, 0.0, MENU_DARKENED_BACKGROUND_ALPHA]))
                } else { None },
                menu.gui(args, &mut self.resources),
            )))
    }

    pub fn process_effect_queue(&mut self) {
        self.resources.process_effect_queue(&mut self.stack);
    }
}
