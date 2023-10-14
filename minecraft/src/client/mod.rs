
pub mod connection;
mod apply_edit;
mod prediction;
mod meshing;
mod gui_blocks;
mod menu;


use self::{
    connection::Connection,
    prediction::PredictionManager,
    gui_blocks::{
        item_grid::{
            item_slot_click_logic::{
                NoninteractiveItemSlotClickLogic,
            },
            item_slot_gui_state::{
                ItemSlotGuiState,
                ItemSlotGuiStateNoninteractive,
            },
            ItemGrid,
            ItemGridConfig,
        },
        chat::{
            GuiChat,
            make_chat_input_text_block,
        },
        vignette::Vignette,
        crosshair::Crosshair,
        world::WorldGuiBlock,
    },
    menu::{
        Menu,
        MenuResources,
    },
    meshing::{
        chunk_mesher::{
            ChunkMesher,
            MeshedChunk,
            MeshChunkAbortHandle,
        },
        tile_meshing::mesh_tile,
        char_mesh::CharMesh,
        item_mesh::ItemMesh,
    },
    apply_edit::EditWorld,
};
use crate::{
    block_update_queue::BlockUpdateQueue,
    chunk_mesh::ChunkMesh,
    gui::prelude::*,
    physics::prelude::*,
    util::{
        secs_rem::secs_rem,
        array::{
            ArrayBuilder,
            array_from_fn,
        },
        sparse_vec::SparseVec,
        number_key::num_row_key,
    },
    message::*,
    server::{
        ServerHandle,
        NetworkBindGuard,
    },
    save_file::SaveFile,
    item::*,
    game_data::{
        per_item::PerItem,
        item_mesh_logic::ItemMeshLogic,
    },
};
use chunk_data::*;
use mesh_data::*;
use graphics::prelude::*;
use std::{
    ops::Range,
    f32::consts::PI,
    cell::RefCell,
    rc::Rc,
    time::{
        Instant,
        Duration,
    },
    mem::{
        take,
        replace,
    },
    fmt::Debug,
};
use slab::Slab;
use anyhow::{Result, ensure, bail};
use vek::*;
use image::{
    DynamicImage,
    RgbaImage,
};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;


pub const CAMERA_HEIGHT: f32 = 1.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_WIDTH: f32 = 0.6;

const PLAYER_BOX_EXT: [f32; 3] = [PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH];
const PLAYER_BOX_POS_ADJUST: [f32; 3] = [PLAYER_WIDTH / 2.0, 0.0, PLAYER_WIDTH / 2.0];

const BOB_ANIMATION_LOOP_TIME: f32 = 0.7;
const MAX_BOB_ROLL_DEGS: f32 =  0.06;
const MAX_BOB_SHIFT_H: f32 = 0.03;
const MAX_BOB_SHIFT_V: f32 = 0.05;

const GROUND_DETECTION_PERIOD: f32 = 1.0 / 20.0;


/// GUI state frame for multiplayer game client.
#[derive(Debug)]
pub struct Client {
    internal_server: Option<InternalServer>,
    connection: Connection,

    char_mesh: CharMesh,
    char_name_layed_out: LayedOutTextBlock,

    char_state: CharState,
    noclip: bool,
    vel: Vec3<f32>,
    time_since_ground: f32,
    time_since_jumped: f32,

    char_state_last_sent: CharState,
    char_state_last_sent_time: Instant, // TODO: time handling is kinda a mess

    bob_animation: f32,
    third_person: bool,

    chunk_mesher: ChunkMesher,

    chunks: LoadedChunks,
    ci_reverse_lookup: SparseVec<Vec3<i64>>,

    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<MaybePendingChunkMesh>,
    block_updates: BlockUpdateQueue,

    prediction: PredictionManager,

    clients: Slab<()>,
    my_client_key: Option<usize>,
    
    client_username: SparseVec<String>,
    client_char_state: SparseVec<CharState>,
    client_char_name_layed_out: SparseVec<LayedOutTextBlock>,

    menu_stack: Vec<Menu>,
    menu_resources: MenuResources,

    items_mesh: PerItem<ItemMesh>,

    chat: GuiChat,

    day_night_time: f32,
    white_pixel: GpuImageArray,
    stars: Mesh,

    held_item: RefCell<ItemSlot>,
    held_item_state: ItemSlotGuiStateNoninteractive,

    inventory_slots: Box<[ItemSlot; 36]>,
    inventory_slots_state: Box<[ItemSlotGuiState; 36]>,

    inventory_slots_armor: [ItemSlot; 4],
    inventory_slots_armor_state: [ItemSlotGuiState; 4],
    
    inventory_slots_crafting: [ItemSlot; 4],
    inventory_slots_crafting_state: [ItemSlotGuiState; 4],

    inventory_slot_crafting_output: ItemSlot,
    inventory_slot_crafting_output_state: ItemSlotGuiState,

    hotbar_slots_state: [ItemSlotGuiStateNoninteractive; 9],
    hotbar_selected: u8,

    open_menu_msg_idx: Option<u64>,
}

#[derive(Debug)]
pub enum MaybePendingChunkMesh {
    ChunkMesh(ChunkMesh),
    Pending(PendingChunkMesh),
}

#[derive(Debug)]
pub struct PendingChunkMesh {
    abort: MeshChunkAbortHandle,
    buffered_updates: Vec<u16>,
    update_buffered: PerTileBool,
}

#[derive(Debug)]
pub struct InternalServer {
    server: ServerHandle,
    bind_to_lan: Option<NetworkBindGuard>,
}

fn get_username() -> String {
    // TODO: temporary hacky sillyness
    use std::process::Command;

    let command = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "echo %username%"])
            .output()
            .expect("Failed to execute command")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("echo $USER")
            .output()
            .expect("Failed to execute command")
    };

    let output = String::from_utf8_lossy(&command.stdout);
    let username = output.trim().to_string();
    if username.is_empty() {
        "username_not_found".into()
    } else {
        username
    }
}

impl Client {
    /// Start an internal server and create a client connected to it.
    pub fn new_internal(save: SaveFile, ctx: &GuiGlobalContext) -> Self {
        let internal_server = ServerHandle::start(
            save,
            ctx.game,
            ctx.tokio,
            ctx.thread_pool,
        );
        let connection = internal_server.internal_connection();
        Self::inner_new(Some(internal_server), connection, ctx)
    }

    /// Connect to a server via network.
    pub fn connect(address: &str, ctx: &GuiGlobalContext) -> Self {
        let connection = Connection::connect(address, ctx.tokio, ctx.game);
        Self::inner_new(None, connection, ctx)
    }

    fn inner_new(
        internal_server: Option<ServerHandle>,
        connection: Connection,
        ctx: &GuiGlobalContext,
    ) -> Self {
        let username = get_username();
        let char_state = CharState {
            pos: [0.0, 80.0, 0.0].into(),
            pitch: f32::to_radians(-30.0),
            yaw: f32::to_radians(0.0),
            pointing: false,
            load_dist: 6,
        };

        connection.send(up::LogIn {
            username: username.clone(),
        });
        connection.send(up::JoinGame {});

        let char_mesh = CharMesh::new(ctx);
        let char_name_layed_out = ctx.renderer.borrow()
            .lay_out_text(&TextBlock {
                spans: &[TextSpan {
                    text: &username,
                    font: ctx.assets.font,
                    font_size: 16.0,
                    color: Rgba::white(),
                }],
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                wrap_width: None,
            });

        let chunk_mesher = ChunkMesher::new(
            ctx.thread_pool,
            || ctx.renderer.borrow().create_async_gpu_vec_context(),
            ctx.game,
        );

        let mut image = RgbaImage::new(1, 1);
        image[(0, 0)] = [0xff; 4].into();
        let white_pixel = ctx.renderer.borrow().load_image_array_raw(
            [1, 1].into(),
            [DynamicImage::from(image)],
        );

        let mut rng = ChaCha20Rng::from_seed([0; 32]);
        let mut stars = MeshData::new();
        let mut star = MeshData::new();
        for _ in 0..1500 {
            let u1: f32 = rng.gen_range(0.0..1.0);
            let u2: f32 = rng.gen_range(0.0..1.0);
            let u3: f32 = rng.gen_range(0.0..1.0);

            let w = (1.0 - u1).sqrt() * (2.0 * PI * u2).sin();
            let x = (1.0 - u1).sqrt() * (2.0 * PI * u2).cos();
            let y = u1.sqrt() * (2.0 * PI * u3).sin();
            let z = u1.sqrt() * (2.0 * PI * u3).cos();

            let star_quat = Quaternion { w, x, y, z };
            let star_size = rng.gen_range(0.5..2.0);
            let star_light = rng.gen_range(1.0f32..10.0).powf(2.0) / 100.0;

            star.add_quad(&Quad {
                pos_start: [-star_size / 2.0, -star_size / 2.0, 300.0].into(),
                pos_ext_1: [0.0, star_size, 0.0].into(),
                pos_ext_2: [star_size, 0.0, 0.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[1.0, 1.0, 1.0, star_light].into(); 4],
                tex_index: 0,
            });

            for v in &mut star.vertices {
                v.pos = star_quat * v.pos;
            }

            stars.extend(star.vertices.iter().copied(), star.indices.iter().copied());
            star.clear();
        }
        let stars = stars.upload(&*ctx.renderer.borrow());
        
        let mut items_mesh = PerItem::new_no_default();
        for iid in ctx.game.items.iter() {
            items_mesh.set(iid, match &ctx.game.items_mesh_logic[iid] {
                &ItemMeshLogic::FullCube {
                    top_tex_index,
                    left_tex_index,
                    right_tex_index,
                } => {
                    const LEFT_SHADE: f32 = 0x48 as f32 / 0x8f as f32;
                    const RIGHT_SHADE: f32 = 0x39 as f32 / 0x8f as f32;

                    let mut mesh_buf = MeshData::new();
                    mesh_buf.add_quad(&Quad {
                        pos_start: [1.0, 1.0, 0.0].into(),
                        pos_ext_1: [-1.0, 0.0, 0.0].into(),
                        pos_ext_2: [0.0, 0.0, 1.0].into(),
                        tex_start: 0.0.into(),
                        tex_extent: 1.0.into(),
                        vert_colors: [Rgba::white(); 4],
                        tex_index: top_tex_index,
                    });
                    mesh_buf.add_quad(&Quad {
                        pos_start: [0.0, 0.0, 0.0].into(),
                        pos_ext_1: [0.0, 1.0, 0.0].into(),
                        pos_ext_2: [1.0, 0.0, 0.0].into(),
                        tex_start: 0.0.into(),
                        tex_extent: 1.0.into(),
                        vert_colors: [[LEFT_SHADE, LEFT_SHADE, LEFT_SHADE, 1.0].into(); 4],
                        tex_index: left_tex_index,
                    });
                    mesh_buf.add_quad(&Quad {
                        pos_start: [1.0, 0.0, 0.0].into(),
                        pos_ext_1: [0.0, 1.0, 0.0].into(),
                        pos_ext_2: [0.0, 0.0, 1.0].into(),
                        tex_start: 0.0.into(),
                        tex_extent: 1.0.into(),
                        vert_colors: [[RIGHT_SHADE, RIGHT_SHADE, RIGHT_SHADE, 1.0].into(); 4],
                        tex_index: right_tex_index,
                    });
                    ItemMesh {
                        mesh: mesh_buf.upload(&*ctx.renderer.borrow()),
                    }
                }
            });
        }

        ctx.capture_mouse();

        Client {
            internal_server: internal_server.map(|server| InternalServer {
                server,
                bind_to_lan: None,
            }),
            connection,

            char_mesh,
            char_name_layed_out,


            char_state,
            noclip: true,
            vel: 0.0.into(),
            time_since_ground: f32::INFINITY,
            time_since_jumped: f32::INFINITY,
            
            char_state_last_sent: char_state,
            char_state_last_sent_time: Instant::now(),

            bob_animation: 0.0,
            third_person: false,

            chunk_mesher,

            chunks: LoadedChunks::new(),
            ci_reverse_lookup: SparseVec::new(),

            tile_blocks: PerChunk::new(),
            tile_meshes: PerChunk::new(),
            block_updates: BlockUpdateQueue::new(),

            prediction: PredictionManager::new(),

            clients: Slab::new(),
            my_client_key: None,

            client_username: SparseVec::new(),
            client_char_state: SparseVec::new(),
            client_char_name_layed_out: SparseVec::new(),

            menu_stack: Vec::new(),
            menu_resources: MenuResources::new(ctx),

            items_mesh,

            chat: GuiChat::new(),

            day_night_time: 0.1,
            white_pixel,
            stars,

            held_item: RefCell::new(None),
            held_item_state: ItemSlotGuiStateNoninteractive::new(),

            inventory_slots: Box::new(array_from_fn(|_| None)),
            //inventory_slots,
            inventory_slots_state: Box::new(array_from_fn(|_| ItemSlotGuiState::new())),

            inventory_slots_armor: array_from_fn(|_| None),
            inventory_slots_armor_state: array_from_fn(|_| ItemSlotGuiState::new()),

            inventory_slots_crafting: array_from_fn(|_| None),
            inventory_slots_crafting_state: array_from_fn(|_| ItemSlotGuiState::new()),

            inventory_slot_crafting_output: None,
            inventory_slot_crafting_output_state: ItemSlotGuiState::new(),

            hotbar_slots_state: array_from_fn(|_| ItemSlotGuiStateNoninteractive::new()),
            hotbar_selected: 0,

            open_menu_msg_idx: None,
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        const MENU_DARKENED_BACKGROUND_ALPHA: f32 = 1.0 - 0x2a as f32 / 0x97 as f32;

        let mut chat = Some(&mut self.chat);
        
        let (
            inventory_slots_bottom,
            inventory_slots_top,
        ) = self.inventory_slots.split_at_mut(9);

        let mut rc_inventory_slots_bottom = ArrayBuilder::new();
        for slot in inventory_slots_bottom {
            rc_inventory_slots_bottom.push(Rc::new(RefCell::new(slot)));
        }
        let rc_inventory_slots_bottom: [Rc<RefCell<&'a mut ItemSlot>>; 9] = rc_inventory_slots_bottom.build();

        let menu_gui = self.menu_stack.iter_mut().rev().next()
            .map(|open_menu| layer((
                if open_menu.has_darkened_background() {
                    Some(solid([0.0, 0.0, 0.0, MENU_DARKENED_BACKGROUND_ALPHA]))
                } else { None },
                open_menu.gui(
                    &mut self.menu_resources,
                    &mut chat,
                    &mut self.internal_server,
                    &self.items_mesh,
                    &self.connection,
                    &self.held_item,
                    &mut self.held_item_state,
                    &rc_inventory_slots_bottom,
                    inventory_slots_top,
                    &mut self.inventory_slots_state,
                    &mut self.inventory_slots_armor,
                    &mut self.inventory_slots_armor_state,
                    &mut self.inventory_slots_crafting,
                    &mut self.inventory_slots_crafting_state,
                    &mut self.inventory_slot_crafting_output,
                    &mut self.inventory_slot_crafting_output_state,
                    &self.char_mesh,
                    self.char_state.pitch,
                    self.char_state.pointing,
                    self.open_menu_msg_idx,
                    ctx,
                ),
            )));
        layer((
            WorldGuiBlock {
                pos: self.char_state.pos,
                pitch: self.char_state.pitch,
                yaw: self.char_state.yaw,
                pointing: self.char_state.pointing,
                load_dist: self.char_state.load_dist,

                chunks: &self.chunks,
                tile_blocks: &self.tile_blocks,
                tile_meshes: &mut self.tile_meshes,

                bob_animation: self.bob_animation,
                third_person: self.third_person,

                char_mesh: &self.char_mesh,
                char_name_layed_out: &self.char_name_layed_out,

                my_client_key: self.my_client_key,
                client_char_state: &self.client_char_state,
                client_char_name_layed_out: &self.client_char_name_layed_out,

                day_night_time: self.day_night_time,
                stars: &self.stars,
                white_pixel: &self.white_pixel,
            },
            align([0.5, 1.0],
                logical_size([364.0, 44.0],
                    layer((
                        &ctx.assets().hud_hotbar,
                        align(0.5,
                            ItemGrid {
                                slots: rc_inventory_slots_bottom,
                                slots_state: self.hotbar_slots_state.iter_mut(),
                                click_logic: NoninteractiveItemSlotClickLogic,
                                grid_size: [9, 1].into(),
                                config: ItemGridConfig {
                                    pad: 4.0,
                                    ..ItemGridConfig::default()
                                },
                                items_mesh: &self.items_mesh,
                            }
                        ),
                        align([self.hotbar_selected as f32 / 8.0, 0.5],
                            logical_size(44.0,
                                align(0.5,
                                    logical_size(48.0,
                                        &ctx.assets().hud_hotbar_selected,
                                    )
                                )
                            )
                        )
                    ))
                )
            ),
            align(0.5,
                logical_size(30.0,
                    Crosshair
                )
            ),
            Vignette,
            chat.map(|chat| 
                v_margin(0.0, 80.0,
                    align([0.0, 1.0],
                        chat.gui(true)
                    )
                )
            ),
            if menu_gui.is_none() {
                Some(mouse_capturer())
            } else { None },
            menu_gui,
        ))
    }

    fn on_network_message(&mut self, msg: DownMessage, ctx: &GuiGlobalContext) -> Result<()> {
        match msg {
            DownMessage::Close(msg) => self.on_network_message_close(msg)?,
            DownMessage::AcceptLogin(msg) => self.on_network_message_accept_login(msg)?,
            DownMessage::ShouldJoinGame(msg) => self.on_network_message_should_join_game(msg)?,
            DownMessage::AddChunk(msg) => self.on_network_message_add_chunk(msg, ctx)?,
            DownMessage::RemoveChunk(msg) => self.on_network_message_remove_chunk(msg, ctx)?,
            DownMessage::AddClient(msg) => self.on_network_message_add_client(msg, ctx)?,
            DownMessage::RemoveClient(msg) => self.on_network_message_remove_client(msg)?,
            DownMessage::ApplyEdit(msg) => self.on_network_message_apply_edit(msg)?,
            DownMessage::Ack(msg) => self.on_network_message_ack(msg)?,
            DownMessage::ChatLine(msg) => self.on_network_message_chat_line(msg, ctx)?,
            DownMessage::SetCharState(msg) => self.on_network_message_set_char_state(msg)?,
            DownMessage::CloseGameMenu(msg) => self.on_network_message_close_game_menu(msg)?,
        }
        Ok(())
    }

    fn on_network_message_close(&mut self, msg: down::Close) -> Result<()> {
        let down::Close { message } = msg;
        bail!("server closed connection: {:?}", message);
    }

    fn on_network_message_accept_login(&mut self, msg: down::AcceptLogin) -> Result<()> {
        let down::AcceptLogin { inventory_slots } = msg;
        *self.inventory_slots = inventory_slots;
        info!("server accepted login");
        Ok(())
    }

    fn on_network_message_should_join_game(&mut self, msg: down::ShouldJoinGame) -> Result<()> {
        let down::ShouldJoinGame { own_client_key } = msg;
        info!("client fully initialized");
        self.my_client_key = Some(own_client_key);
        self.char_state = self.client_char_state[own_client_key];
        self.noclip = false;
        Ok(())
    }
    
    fn on_network_message_add_chunk(&mut self, msg: down::AddChunk, ctx: &GuiGlobalContext) -> Result<()> {
        let down::AddChunk { cc, ci, chunk_tile_blocks } = msg;

        // insert into data structures
        ensure!(
            self.chunks.add(cc) == ci,
            "AddChunk message ci did not correspond to slab behavior",
        );
        self.ci_reverse_lookup.set(ci, cc);

        self.tile_blocks.add(cc, ci, chunk_tile_blocks);
        self.block_updates.add_chunk(cc, ci);

        self.prediction.add_chunk(cc, ci);
        
        // request it be meshed asynchronously
        // TODO: if the cloning here is expensive, we could potentially optimize
        //       it a fair bit by doing some arc cow thing
        self.tile_meshes.add(cc, ci, MaybePendingChunkMesh::Pending(PendingChunkMesh {
            abort: self.chunk_mesher.request(
                cc,
                ci,
                ctx.game.clone_chunk_blocks(self.tile_blocks.get(cc, ci)),
            ),
            buffered_updates: Vec::new(),
            update_buffered: PerTileBool::new(),
        }));

        // enqueue block updates to neighbors
        let getter = self.chunks.getter();
        for fec in FACES_EDGES_CORNERS {
            let ranges: Vec3<Range<i64>> = fec
                .to_signs()
                .zip(CHUNK_EXTENT)
                .map(|(sign, extent)| match sign {
                    Sign::Neg => -1..0,
                    Sign::Zero => 0..extent,
                    Sign::Pos => extent..extent + 1,
                });

            for z in ranges.z {
                for y in ranges.y.clone() {
                    for x in ranges.x.clone() {
                        let gtc = cc * CHUNK_EXTENT + Vec3 { x, y, z };
                        self.block_updates.enqueue(gtc, &getter);
                    }
                }
            }
        }

        Ok(())
    }

    fn on_network_message_remove_chunk(&mut self, msg: down::RemoveChunk, ctx: &GuiGlobalContext) -> Result<()> {
        let down::RemoveChunk { cc, ci } = msg;

        // removing a chunk from the block update queue requires that there
        // exist no pending block updates
        self.do_block_updates(ctx);

        ensure!(
            self.chunks.getter().get(cc) == Some(ci),
            "RemoveChunk message cc and ci did not match",
        );

        self.chunks.remove(cc);
        self.ci_reverse_lookup.remove(ci);
        self.tile_blocks.remove(cc, ci);
        if let MaybePendingChunkMesh::Pending(PendingChunkMesh {
            abort,
            ..
        }) = self.tile_meshes.remove(cc, ci) {
            self.connection.send(up::AcceptMoreChunks { number: 1 });
            abort.abort();
        }
        self.block_updates.remove_chunk(cc, ci);
        self.prediction.remove_chunk(cc, ci);

        Ok(())
    }
    
    fn on_network_message_add_client(&mut self, msg: down::AddClient, ctx: &GuiGlobalContext) -> Result<()> {
        let down::AddClient { client_key, username, char_state } = msg;
        ensure!(
            self.clients.insert(()) == client_key,
            "AddClient message client key did not correspond to slab behavior",
        );
        // TODO: deduplicate this
        let char_name_layed_out = ctx.renderer.borrow()
            .lay_out_text(&TextBlock {
                spans: &[TextSpan {
                    text: &username,
                    font: ctx.assets.font,
                    font_size: 16.0,
                    color: Rgba::white(),
                }],
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                wrap_width: None,
            });
        self.client_username.set(client_key, username);
        self.client_char_state.set(client_key, char_state);
        self.client_char_name_layed_out.set(client_key, char_name_layed_out);

        Ok(())
    }
    
    fn on_network_message_remove_client(&mut self, msg: down::RemoveClient) -> Result<()> {
        let down::RemoveClient { client_key } = msg;
        debug!(?client_key, "client removing");
        self.clients.remove(client_key);
        self.client_username.remove(client_key);
        self.client_char_state.remove(client_key);
        self.client_char_name_layed_out.remove(client_key);

        Ok(())
    }
    
    fn on_network_message_apply_edit(&mut self, msg: down::ApplyEdit) -> Result<()> {
        self.prediction.process_apply_edit_msg(
            msg,
            &mut EditWorld {
                chunks: &self.chunks,
                getter: &self.chunks.getter(),
                ci_reverse_lookup: &self.ci_reverse_lookup,
                tile_blocks: &mut self.tile_blocks,
                block_updates: &mut self.block_updates,
                inventory_slots: &mut self.inventory_slots,
            },
        );

        Ok(())
    }
    
    fn on_network_message_ack(&mut self, msg: down::Ack) -> Result<()> {
        let down::Ack { last_processed } = msg;
        self.prediction.process_ack(
            last_processed,
            &mut EditWorld {
                chunks: &self.chunks,
                getter: &self.chunks.getter(),
                ci_reverse_lookup: &self.ci_reverse_lookup,
                tile_blocks: &mut self.tile_blocks,
                block_updates: &mut self.block_updates,
                inventory_slots: &mut self.inventory_slots,
            },
        );

        Ok(())
    }
    
    fn on_network_message_chat_line(&mut self, msg: down::ChatLine, ctx: &GuiGlobalContext) -> Result<()> {
        let down::ChatLine { line } = msg;
        self.chat.add_line(line, ctx);

        Ok(())
    }

    fn on_network_message_set_char_state(&mut self, msg: down::SetCharState) -> Result<()> {
        let down::SetCharState { client_key, char_state } = msg;
        let () = self.clients[client_key];
        self.client_char_state[client_key] = char_state;

        Ok(())
    }

    fn on_network_message_close_game_menu(&mut self, msg: down::CloseGameMenu) -> Result<()> {
        let down::CloseGameMenu { open_menu_msg_idx } = msg;
        if self.open_menu_msg_idx == Some(open_menu_msg_idx) {
            self.menu_stack.pop().unwrap();
            self.open_menu_msg_idx = None;
        }

        Ok(())
    }

    fn on_chunk_meshed(&mut self, meshed_chunk: MeshedChunk) {
        let MeshedChunk { cc, ci, mesh } = meshed_chunk;

        self.connection.send(up::AcceptMoreChunks { number: 1 });

        // enqueue buffered block updates
        let chunk_tile_meshes = self.tile_meshes.get_mut(cc, ci);
        match chunk_tile_meshes {
            &mut MaybePendingChunkMesh::Pending(
                PendingChunkMesh { ref buffered_updates, .. }
            ) => for &lti in buffered_updates {
                self.block_updates.enqueue_tile_key(TileKey { cc, ci, lti });
            },
            &mut MaybePendingChunkMesh::ChunkMesh(_) => unreachable!(),
        }

        // enqueue block updates for internal fecs
        for fec in FACES_EDGES_CORNERS {
            let ranges: Vec3<Range<i64>> = fec
                .to_signs()
                .zip(CHUNK_EXTENT)
                .map(|(sign, extent)| match sign {
                    Sign::Neg => 0..1,
                    Sign::Zero => 1..extent - 1,
                    Sign::Pos => extent - 1..extent,
                });

            for z in ranges.z {
                for y in ranges.y.clone() {
                    for x in ranges.x.clone() {
                        self.block_updates.enqueue_tile_key(TileKey {
                            cc,
                            ci,
                            lti: ltc_to_lti(Vec3 { x, y, z }),
                        });
                    }
                }
            }
        }

        // switch over
        *chunk_tile_meshes = MaybePendingChunkMesh::ChunkMesh(mesh);
    }

    fn on_ground(&self) -> bool {
        self.time_since_ground < GROUND_DETECTION_PERIOD
        && self.time_since_jumped > GROUND_DETECTION_PERIOD
    }

    fn do_block_updates(&mut self, ctx: &GuiGlobalContext) {
        let mut mesh_buf = MeshData::new();
        let getter = self.chunks.getter();
        while let Some(tile) = self.block_updates.pop() {
            match self.tile_meshes.get_mut(tile.cc, tile.ci) {
                &mut MaybePendingChunkMesh::ChunkMesh(ref mut chunk_mesh) => {
                    // re-mesh tile
                    mesh_buf.clear();
                    mesh_tile(
                        &mut mesh_buf,
                        tile,
                        &getter,
                        &self.tile_blocks,
                        ctx.game,
                    );
                    mesh_buf.translate(lti_to_ltc(tile.lti).map(|n| n as f32));
                    chunk_mesh.set_tile_submesh(tile.lti, &mesh_buf);
                }
                &mut MaybePendingChunkMesh::Pending(PendingChunkMesh {
                    ref mut buffered_updates,
                    ref mut update_buffered,
                    ..
                }) => {
                    // buffer update to be re-applied when the initial mesh is received
                    if !update_buffered.get(tile.lti) {
                        update_buffered.set(tile.lti, true);
                        buffered_updates.push(tile.lti);
                    }
                }
            }
        }
    }

    fn try_jump(&mut self) {
        if !self.noclip && self.on_ground() {
            self.vel.y += 9.2;
            self.time_since_jumped = 0.0;
        }
    }
}


impl GuiStateFrame for Client {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
        // process async events

        // TODO: this is a crude form of rate limiting
        //       a better version would involve QUIC and winit user events
        let mut t = Instant::now();
        let process_async_cutoff = t + ctx.global().frame_duration_target / 3;
        let mut opt_t_msg = Some(Duration::ZERO);
        let mut opt_t_chunk = Some(Duration::ZERO);
        loop {
            if let Some(t_msg) = opt_t_msg
                .filter(|&t_msg| opt_t_chunk
                    .map(|t_chunk| t_msg <= t_chunk)
                    .unwrap_or(true))
            {
                // messages from the server
                let opt_msg = match self.connection.poll() {
                    Ok(opt_msg) => opt_msg,
                    Err(e) => {
                        error!(%e, "client connection error");
                        ctx.global().pop_state_frame();
                        return;
                    }
                };
                if let Some(msg) = opt_msg {
                    if let Err(e) = self.on_network_message(msg, ctx.global()) {
                        error!(%e, "error processing message from server");
                        ctx.global().pop_state_frame();
                        return;
                    }

                    let old_t = replace(&mut t, Instant::now());
                    opt_t_msg = Some(t_msg + (t - old_t));
                } else {
                    opt_t_msg = None;
                }
            } else if let Some(t_chunk) = opt_t_chunk
                .filter(|&t_chunk| opt_t_msg
                    .map(|t_msg| t_chunk <= t_msg)
                    .unwrap_or(true))
            {
                // chunks finished being meshed
                if let Some(meshed_chunk) = self.chunk_mesher.try_recv() {
                    self.on_chunk_meshed(meshed_chunk);
                    let old_t = replace(&mut t, Instant::now());
                    opt_t_chunk = Some(t_chunk + (t - old_t));
                } else {
                    opt_t_chunk = None;
                }
            } else {
                break;
            }

            if t > process_async_cutoff {
                break;
            }
        }

        // menu stuff
        self.menu_resources.process_effect_queue(&mut self.menu_stack);

        if let Some(&mut Menu::ChatInput {
            ref mut t_preventer,
            ref mut blinker,
            ref text,
            ref mut text_block,
        }) = self.menu_stack.iter_mut().rev().next() {
            *t_preventer = false;

            let prev_blinker = *blinker;
            *blinker = secs_rem(ctx.global().time_since_epoch, 2.0 / 3.0) < 1.0 / 3.0;
            if *blinker != prev_blinker {
                *text_block = make_chat_input_text_block(text, *blinker, ctx.global());
            }
        }

        // do block updates
        self.do_block_updates(ctx.global());

        const WALK_SPEED: f32 = 4.0;
        const WALK_ACCEL: f32 = 50.0;
        const WALK_DECEL: f32 = 30.0;
        const NOCLIP_SPEED: f32 = 7.0;
        const NOCLIP_FAST_MULTIPLIER: f32 = 8.0;

        // WASD buttons
        let mut walking_xz = Vec2::from(0.0);
        if ctx.global().focus_level == FocusLevel::MouseCaptured {
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::W) {
                walking_xz.y += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::S) {
                walking_xz.y -= 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::D) {
                walking_xz.x += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::A) {
                walking_xz.x -= 1.0;
            }
        }
        walking_xz.rotate_z(self.char_state.yaw);

        if !self.noclip {
            // walking
            walking_xz *= WALK_SPEED;

            // accelerate self.vel xz towards the target value of walking_xz
            let accel_rate = if walking_xz != Vec2::from(0.0) {
                WALK_ACCEL
            } else {
                WALK_DECEL
            };

            let mut vel_xz = Vec2::new(self.vel.x, self.vel.z);
            let vel_xz_deviation = walking_xz - vel_xz;
            let vel_xz_deviation_magnitude = vel_xz_deviation.magnitude();
            let max_delta_vel_xz_magnitude = accel_rate * elapsed;
            if max_delta_vel_xz_magnitude > vel_xz_deviation_magnitude {
                vel_xz = walking_xz;
            } else {
                vel_xz += vel_xz_deviation / vel_xz_deviation_magnitude * max_delta_vel_xz_magnitude;
            }
            self.vel.x = vel_xz.x;
            self.vel.z = vel_xz.y;

            // jumping
            if ctx.global().focus_level == FocusLevel::MouseCaptured
                && ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::Space)
            {
                self.try_jump();
            }
        } else {
            // noclip reset physics variables
            self.vel = 0.0.into();
            self.time_since_jumped = f32::INFINITY;
            self.time_since_ground = f32::INFINITY;

            // noclip movement
            let mut noclip_move = Vec3::new(walking_xz.x, 0.0, walking_xz.y);

            if ctx.global().focus_level == FocusLevel::MouseCaptured {
                if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::Space) {
                    noclip_move.y += 1.0;
                }
                if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::LShift) {
                    noclip_move.y -= 1.0;
                }

                if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::LControl) {
                    noclip_move *= NOCLIP_FAST_MULTIPLIER;

                    if ctx.global().pressed_mouse_buttons.contains(&MouseButton::Middle) {
                        noclip_move *= NOCLIP_FAST_MULTIPLIER;
                    }
                }
            }

            self.char_state.pos += noclip_move * NOCLIP_SPEED * elapsed;
        }

        const GRAVITY_ACCEL: f32 = 32.0;
        const FALL_SPEED_DECAY: f32 = 0.98;

        // gravity
        let getter = self.chunks.getter();
        if !self.noclip {
            self.vel.y -= GRAVITY_ACCEL * elapsed;
            self.vel.y *= f32::exp(20.0 * f32::ln(FALL_SPEED_DECAY) * elapsed);

            // movement and collision
            self.char_state.pos -= Vec3::from(PLAYER_BOX_POS_ADJUST);
            let physics = do_physics(
                elapsed,
                &mut self.char_state.pos,
                &mut self.vel,
                &AaBoxCollisionObject {
                    ext: PLAYER_BOX_EXT.into(),
                },
                &WorldPhysicsGeometry {
                    getter: &getter,
                    tile_blocks: &self.tile_blocks,
                    game: ctx.game(),
                },
            );
            self.char_state.pos += Vec3::from(PLAYER_BOX_POS_ADJUST);
            self.time_since_ground += elapsed;
            self.time_since_jumped += elapsed;
            if physics.on_ground {
                self.time_since_ground = 0.0;
            }
        }

        // pointing
        self.char_state.pointing = ctx.global().pressed_mouse_buttons.contains(&MouseButton::Middle);

        // send up char state, maybe
        let now = Instant::now();
        if self.char_state != self.char_state_last_sent
            && (now - self.char_state_last_sent_time).as_secs_f32() >= 1.0 / 120.0
        {
            self.connection.send(up::SetCharState {
                char_state: self.char_state,
            });
            self.char_state_last_sent = self.char_state;
            self.char_state_last_sent_time = now;
        }

        // animations
        let ground_speed = if self.on_ground() {
            Vec2::new(self.vel.x, self.vel.z).magnitude()
        } else {
            0.0
        };
        debug_assert!(self.bob_animation >= 0.0);
        debug_assert!(self.bob_animation <= 1.0);
        let mut bob_animation_elapsed = elapsed / BOB_ANIMATION_LOOP_TIME;
        if ground_speed >= WALK_SPEED / 2.0 {
            self.bob_animation += bob_animation_elapsed;
            self.bob_animation %= 1.0;
        } else if !(self.bob_animation == 0.0 || self.bob_animation == 0.5) {
            bob_animation_elapsed *= 1.75;
            
            if self.bob_animation < 0.25 {
                self.bob_animation += (0.25 - self.bob_animation) * 2.0;
            } else if self.bob_animation > 0.5 && self.bob_animation < 0.75 {
                self.bob_animation += (0.75 - self.bob_animation) * 2.0;
            }
            
            if self.bob_animation < 0.5 && self.bob_animation + bob_animation_elapsed > 0.5 {
                self.bob_animation = 0.5;
            } else if self.bob_animation + bob_animation_elapsed > 1.0 {
                self.bob_animation = 0.0;
            } else {
                self.bob_animation += bob_animation_elapsed;
                self.bob_animation %= 1.0;
            }
        }

        // sun
        if ctx.settings().day_night {
            self.day_night_time += elapsed / 240.0;
            self.day_night_time %= 1.0;
        } else {
            self.day_night_time = 0.25;
        }
    }

    fn on_captured_mouse_move(&mut self, _: &GuiWindowContext, amount: Vec2<f32>) {
        let sensitivity = 1.0 / 1600.0;
        
        self.char_state.pitch = (self.char_state.pitch - amount.y * sensitivity).clamp(-PI / 2.0, PI / 2.0);
        self.char_state.yaw = (self.char_state.yaw - amount.x * sensitivity) % (PI * 2.0);
    }

    fn on_captured_mouse_click(&mut self, ctx: &GuiWindowContext, button: MouseButton) {
        let getter = self.chunks.getter();
        if let Some(looking_at) = compute_looking_at(
            // position
            self.char_state.pos + Vec3::new(0.0, CAMERA_HEIGHT, 0.0),
            // direction
            cam_dir(self.char_state.pitch, self.char_state.yaw),
            // reach
            50.0,
            // geometry
            &getter,
            &self.tile_blocks,
            ctx.game(),
        ) {
            if let Some((tile, mut bid_meta, placing)) = match button {
                MouseButton::Left => Some((
                    looking_at.tile,
                    ErasedBidMeta::new(AIR, ()),
                    false,
                )),
                MouseButton::Right => {
                    if looking_at.tile.get(&self.tile_blocks).get() == ctx.game().content.chest.bid_chest {
                        self.menu_stack.push(Menu::Chest {
                            gtc: looking_at.tile.gtc(),
                        });
                        ctx.global().uncapture_mouse();
                        None
                    } else {
                        let gtc = looking_at.tile.gtc() + looking_at.face
                            .map(|face| face.to_vec())
                            .unwrap_or(0.into());
                        getter.gtc_get(gtc).map(|tile| (
                            tile,
                            ErasedBidMeta::new(
                                ctx.global().game.content.chest.bid_chest,
                                Default::default(),
                            ),
                            true,
                        ))
                    }
                }
                _ => None
            } {
                if placing {
                    const EPSILON: f32 = 0.0001;
                    let old_bid_meta = tile
                        .get(&mut self.tile_blocks)
                        .erased_replace(bid_meta);
                    let placing_blocked = WorldPhysicsGeometry {
                        getter: &getter,
                        tile_blocks: &self.tile_blocks,
                        game: ctx.game(),
                    }.box_intersects(AaBox {
                        pos: self.char_state.pos - Vec3::from(PLAYER_BOX_POS_ADJUST),
                        ext: PLAYER_BOX_EXT.into(),
                    }.expand(EPSILON));
                    bid_meta = tile
                        .get(&mut self.tile_blocks)
                        .erased_replace(old_bid_meta);
                    if placing_blocked {
                        return;
                    }
                }

                if !placing {
                    //ctx.sound_player().play(&ctx.assets().grass_dig_sound, 1.0);
                }
                
                self.connection.send(up::SetTileBlock {
                    gtc: tile.gtc(),
                    bid_meta: ctx.game().clone_erased_tile_block(&bid_meta),
                });
                self.prediction.make_prediction(
                    edit::Tile {
                        ci: tile.ci,
                        lti: tile.lti,
                        edit: tile_edit::SetTileBlock {
                            bid_meta,
                        }.into(),
                    }.into(),
                    &mut EditWorld {
                        chunks: &self.chunks,
                        getter: &getter,
                        ci_reverse_lookup: &self.ci_reverse_lookup,
                        tile_blocks: &mut self.tile_blocks,
                        block_updates: &mut self.block_updates,
                        inventory_slots: &mut self.inventory_slots,
                    },
                    &self.connection,
                );
            }
        }
    }

    fn on_key_press_semantic(&mut self, ctx: &GuiWindowContext, key: VirtualKeyCode) {
        if self.menu_stack.is_empty() {
            // game style
            if key == VirtualKeyCode::Escape {
                ctx.global().uncapture_mouse();
                self.menu_stack.push(Menu::EscMenu);
            } else if key == VirtualKeyCode::Space {
                self.try_jump();
            } else if key == VirtualKeyCode::E {
                ctx.global().uncapture_mouse();
                self.menu_stack.push(Menu::Inventory);
                self.connection.send(up::OpenGameMenu {
                    menu: GameMenu::Inventory,
                });
                self.open_menu_msg_idx = Some(self.connection.up_msg_idx());
            } else if key == VirtualKeyCode::T {
                ctx.global().uncapture_mouse();
                let blinker = secs_rem(ctx.global().time_since_epoch, 2.0 / 3.0) < 1.0 / 3.0;
                self.menu_stack.push(Menu::ChatInput {
                    t_preventer: true,
                    text: String::new(),
                    text_block: make_chat_input_text_block("", blinker, ctx.global()),
                    blinker,
                });
            } else if key == VirtualKeyCode::F5 {
                self.third_person = !self.third_person;
            } else if key == VirtualKeyCode::F9 {
                self.noclip = !self.noclip;
            } else if key == VirtualKeyCode::PageUp {
                self.char_state.load_dist = self.char_state.load_dist.saturating_add(1);
            } else if key == VirtualKeyCode::PageDown {
                self.char_state.load_dist = self.char_state.load_dist.saturating_sub(1);
            } else if let Some(n) = num_row_key(key) {
                if n >= 1 && n <= 9 {
                    self.hotbar_selected = n - 1;
                }
            }
        } else {
            // menu style
            if key == VirtualKeyCode::Escape
                || (
                    key == VirtualKeyCode::E
                    && self.menu_stack.iter().rev().next().unwrap()
                        .exitable_via_inventory_button()
                )
            {
                self.menu_stack.pop().unwrap();
                self.connection.send(up::CloseGameMenu {});
                self.open_menu_msg_idx = None;
                ctx.global().capture_mouse();
            } else if key == VirtualKeyCode::V && ctx.global().is_command_key_pressed() {
                if let Some(&mut Menu::ChatInput {
                    t_preventer: _,
                    ref mut text,
                    ref mut text_block,
                    blinker,
                }) = self.menu_stack.iter_mut().rev().next() {
                    text.push_str(&ctx.global().clipboard.get());
                    *text_block = make_chat_input_text_block(text, blinker, ctx.global())
                }
            } else if key == VirtualKeyCode::Return || key == VirtualKeyCode::NumpadEnter {
                if let Some(&mut Menu::ChatInput {
                    ref mut text,
                    ..
                }) = self.menu_stack.iter_mut().rev().next() {
                    self.connection.send(up::Say {
                        text: take(text),
                    });
                    self.menu_stack.pop().unwrap();
                    ctx.global().capture_mouse();
                }
            }
        }
    }

    fn on_character_input(&mut self, ctx: &GuiWindowContext, c: char) {
        if let Some(&mut Menu::ChatInput {
            ref mut t_preventer,
            ref mut text,
            ref mut text_block,
            blinker,
        }) = self.menu_stack.iter_mut().rev().next() {
            if c.is_control() {
                if c == '\u{8}' {
                    // backspace
                    text.pop();
                } else {
                    trace!(?c, "ignoring unknown control character");
                    return;
                }
            } else {
                // to prevent the T key press that opens the chat input from also causing
                // a t character to be typed into the chat input, we have this kind of hacky
                // "t preventer" where, if the first character input after opening the chat
                // and before rendering is a t, we ignore it.
                let prev_t_preventer = *t_preventer;
                *t_preventer = false;
                if (c == 't' || c == 'T') && prev_t_preventer {
                    return;
                }

                text.push(c);
            }
            *text_block = make_chat_input_text_block(text, blinker, ctx.global())
        }
    }

    fn on_captured_mouse_scroll(&mut self, _: &GuiWindowContext, amount: ScrolledAmount) {
        let _ = amount; // TODO
        //self.day_night_time += amount.to_pixels(16.0).y / 8000.0;
        //self.day_night_time %= 1.0;
        
        /*self.scroll_accum += amount.to_pixles(16.0) / 16.0;
        if self.scroll_accum.abs() > 1.0 {

            self.hotbar_selected = self.hotbar_selected
                .wrapping_add((((self.scroll_accum as i32 % 0xff) + 0xff) % 0xff) as u8) % 9;

        }*/ // TODO im too tired to do this right now
    }

    fn on_focus_change(&mut self, ctx: &GuiWindowContext) {
        if ctx.global().focus_level != FocusLevel::MouseCaptured
            && self.menu_stack.is_empty() {
            self.menu_stack.push(Menu::EscMenu);
        }
    }
}

pub fn cam_dir(pitch: f32, yaw: f32) -> Vec3<f32> {
    Quaternion::rotation_y(-yaw)
        * Quaternion::rotation_x(-pitch)
        * Vec3::new(0.0, 0.0, 1.0)
}
