
use crate::{
    gui::prelude::*,
    client::{
        gui_blocks::{
            item_grid::{
                item_slot_click_logic::{
                    MultiplayerItemSlotClickLogic,
                },
                item_slot_gui_state::{
                    ItemSlotGuiStateNoninteractive,
                    ItemSlotGuiState,
                },
                HeldItemGuiBlock,
                ItemGridConfig,
                ItemGrid,
            },
            chat::GuiChat,
            single_chest_bg::SingleChestBg,
        },
        meshing::{
            char_mesh::{
                CharMesh,
                CharMeshGuiBlock,
            },
            item_mesh::ItemMesh,
        },
        connection::Connection,
        InternalServer,
        PredictionToMake,
    },
    item::*,
    game_data::per_item::PerItem,
    settings::Settings,
};
use std::{
    iter::once,
    cell::RefCell,
    rc::Rc,
    collections::VecDeque,
};
use vek::*;
use graphics::prelude::*;


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
    ChatInput {
        t_preventer: bool,
        text: String,
        text_block: GuiTextBlock<true>,
        blinker: bool,
    },
    Settings,
    Chest {
        #[allow(dead_code)]
        gtc: Vec3<i64>,
    }
}

impl Menu {
    pub fn gui<'a>(
        &'a mut self,
        resources: &'a mut MenuResources,
        chat: &mut Option<&'a mut GuiChat>,
        internal_server: &'a mut Option<InternalServer>,
        items_mesh: &'a PerItem<ItemMesh>,
        connection: &'a Connection,
        predictions_to_make: &'a RefCell<VecDeque<PredictionToMake>>,

        held_item: &'a ItemSlot,
        held_item_state: &'a mut ItemSlotGuiStateNoninteractive,

        inventory_slots: &'a [ItemSlot; 36],
        inventory_slots_state: &'a mut Box<[ItemSlotGuiState; 36]>,

        inventory_slots_armor: &'a [ItemSlot; 4],
        inventory_slots_armor_state: &'a mut [ItemSlotGuiState; 4],
        
        inventory_slots_crafting: &'a [ItemSlot; 4],
        inventory_slots_crafting_state: &'a mut [ItemSlotGuiState; 4],

        inventory_slot_crafting_output: &'a ItemSlot,
        inventory_slot_crafting_output_state: &'a mut ItemSlotGuiState,

        char_mesh: &'a CharMesh,
        head_pitch: f32,
        pointing: bool,

        open_menu_msg_idx: Option<u64>,

        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
        let (
            inventory_slots_state_bottom,
            inventory_slots_state_top,
        ) = inventory_slots_state.split_at_mut(9);

        match self {
            &mut Menu::EscMenu => GuiEither::A(GuiEither::A(GuiEither::A(align(0.5,
                logical_size([400.0, 320.0],
                    v_align(0.0,
                        v_stack(0.0, (
                            &mut resources.esc_menu_title_text,
                            logical_height(72.0, gap()),
                            resources.exit_menu_button.gui(on_exit_menu_click(&resources.effect_queue)),
                            logical_height(8.0, gap()),
                            resources.exit_game_button.gui(on_exit_game_click),
                            logical_height(8.0, gap()),
                            resources.open_to_lan_button.gui(on_open_to_lan_click(internal_server)),
                            logical_height(56.0 - 48.0, gap()),
                            resources.options_button.gui(on_options_click(&resources.effect_queue)),
                        ))
                    )
                )
            )))),
            &mut Menu::Inventory => GuiEither::A(GuiEither::A(GuiEither::B(align(0.5,
                logical_size(Vec2::new(176.0, 166.0) * 2.0,
                    layer((
                        &ctx.assets().gui_inventory,
                        margin(52.0, 0.0, 160.0, 0.0,
                            align(0.0,
                                logical_size([104.0, 140.0],
                                    CharMeshGuiBlock {
                                        char_mesh,
                                        head_pitch,
                                        pointing,
                                    }
                                )
                            )
                        ),
                        margin(14.0, 0.0, 166.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: &inventory_slots[9..],
                                    slots_state: inventory_slots_state_top.iter_mut(),
                                    /*click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },*/
                                    click_logic: MultiplayerItemSlotClickLogic {
                                        slot_offset: 9,
                                        open_menu_msg_idx: open_menu_msg_idx.unwrap(),
                                        connection,
                                        predictions_to_make,
                                    },
                                    grid_size: [9, 3].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        margin(14.0, 0.0, 282.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: &inventory_slots[..9],
                                    slots_state: inventory_slots_state_bottom.iter_mut(),
                                    /*click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },*/
                                    click_logic: MultiplayerItemSlotClickLogic {
                                        // TODO: handling of open_menu_msg_idx Some vs None
                                        // and its relation to menu stack seems kinda delicate
                                        slot_offset: 0,
                                        open_menu_msg_idx: open_menu_msg_idx.unwrap(),
                                        connection,
                                        predictions_to_make,
                                    },
                                    grid_size: [9, 1].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),/*
                        margin(14.0, 0.0, 14.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: inventory_slots_armor,
                                    slots_state: inventory_slots_armor_state,
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [1, 4].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        margin(174.0, 0.0, 50.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: inventory_slots_crafting,
                                    slots_state: inventory_slots_crafting_state,
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [2, 2].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        margin(286.0, 0.0, 70.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: once(inventory_slot_crafting_output),
                                    slots_state: once(inventory_slot_crafting_output_state),
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [1, 1].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        */
                        HeldItemGuiBlock {
                            held: held_item,
                            held_state: held_item_state,
                            items_mesh: &items_mesh,
                        }
                    ))
                )
            )))),
            &mut Menu::ChatInput {
                ref mut text_block,
                ..
            } => GuiEither::A(GuiEither::B(GuiEither::A(v_align(1.0,
                v_stack(0.0, (
                    h_align(0.0,
                        chat.take().unwrap().gui(false)
                    ),
                    min_height(80.0, 1.0,
                        h_margin(4.0, 4.0,
                            v_pad(4.0, 4.0,
                                before_after(
                                    (
                                        solid(MENU_BACKGROUND),
                                    ),
                                    min_height(24.0, 1.0,
                                        h_margin(4.0, 4.0,
                                            v_pad(4.0, 4.0,
                                                text_block,
                                            )
                                        )
                                    ),
                                    (),
                                )
                            )
                        )
                    ),
                ))
            )))),
            &mut Menu::Settings => GuiEither::A(GuiEither::B(GuiEither::B(
                align([0.5, 0.0],
                    logical_width(400.0,
                        v_stack(0.0, (
                            logical_height(40.0, gap()),
                            &mut resources.options_menu_title_text,
                            logical_height(22.0, gap()),
                            h_align(0.5,
                                h_stack_auto(20.0, (
                                    logical_width(300.0,
                                        v_stack(8.0, (
                                            resources.options_day_night_button.gui(ctx.global(), |s| &mut s.day_night),
                                            resources.options_fog_button.gui(ctx.global(), |s| &mut s.fog),
                                        ))
                                    ),
                                    logical_width(300.0,
                                        v_stack(8.0, (
                                            resources.options_load_dist_outline_button.gui(ctx.global(), |s| &mut s.load_dist_outline),
                                            resources.options_chunk_outline_button.gui(ctx.global(), |s| &mut s.chunk_outline),
                                        ))
                                    ),
                                ))
                            ),
                            logical_height(32.0, gap()),
                            resources.options_done_button.gui(on_options_done_click(&resources.effect_queue)),
                        ))
                    )
                )
            ))),
            &mut Menu::Chest { gtc: _ /* TODO */ } => GuiEither::B(
                align(0.5,
                    logical_size([352.0, 336.0],
                        layer((
                            SingleChestBg,
                            /*
                            margin(14.0, 0.0, 170.0, 0.0,
                                align(0.0,
                                    ItemGrid {
                                        slots: inventory_slots_top,
                                        slots_state: inventory_slots_state_top.iter_mut(),
                                        click_logic: StorageItemSlotClickLogic {
                                            held: held_item,
                                        },
                                        grid_size: [9, 3].into(),
                                        config: ItemGridConfig::default(),
                                        items_mesh: &items_mesh,
                                    }
                                )
                            ),
                            margin(14.0, 0.0, 286.0, 0.0,
                                align(0.0,
                                    ItemGrid {
                                        slots: inventory_slots_bottom.clone(),
                                        slots_state: inventory_slots_state_bottom.iter_mut(),
                                        click_logic: StorageItemSlotClickLogic {
                                            held: held_item,
                                        },
                                        grid_size: [9, 1].into(),
                                        config: ItemGridConfig::default(),
                                        items_mesh: &items_mesh,
                                    }
                                )
                            ),*/
                            HeldItemGuiBlock {
                                held: held_item,
                                held_state: held_item_state,
                                items_mesh: &items_mesh,
                            }
                        ))
                    )
                )
            ),
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

fn on_exit_menu_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PopMenu);
    }
}

fn on_exit_game_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}

fn on_open_to_lan_click<'a>(internal_server: &'a mut Option<InternalServer>) -> impl FnOnce(&GuiGlobalContext) + 'a {
    move |_| {
        if let &mut Some(ref mut internal_server) = internal_server {
            if internal_server.bind_to_lan.is_none() {
                let bind_to = "0.0.0.0:35565";
                info!("binding to {}", bind_to);
                internal_server.bind_to_lan = Some(internal_server.server.open_to_network(bind_to));
            } else {
                error!("already bound to lan");
            }
        } else {
            error!("cannot open to LAN because not the host");
        }
    }
}

fn on_options_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PushMenu(Menu::Settings));
    }
}

fn on_options_done_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PopMenu);
    }
}
