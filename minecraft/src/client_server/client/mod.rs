
pub mod connection;
mod apply_edit;
mod tile_meshing;
mod prediction;
mod chunk_mesher;

use self::{
    connection::Connection,
    tile_meshing::mesh_tile,
    prediction::PredictionManager,
    chunk_mesher::{
        ChunkMesher,
        MeshedChunk,
        MeshChunkAbortHandle,
    },
};
use crate::{
    asset::Assets,
    block_update_queue::BlockUpdateQueue,
    chunk_mesh::ChunkMesh,
    gui::prelude::*,
    physics::prelude::*,
    util::{
        hex_color::hex_color,
        secs_rem::secs_rem,
        array::{
            ArrayBuilder,
            array_from_fn,
            array_each_mut,
        },
        sparse_vec::SparseVec,
    },
    settings::Settings,
    client_server::{
        message::*,
        server::{
            ServerHandle,
            NetworkBindGuard,
        },
    },
    save_file::SaveFile,
    item::*,
    game_data::{
        GameData,
        per_item::PerItem,
        item_mesh_logic::ItemMeshLogic,
    },
};
use chunk_data::*;
use mesh_data::*;
use graphics::{
    prelude::*,
    frame_content::{
        DrawObj2,
        DrawInvert,
        DrawSky,
    },
};
use std::{
    ops::{
        Range,
        Deref,
    },
    f32::consts::PI,
    cell::{
        self,
        RefCell
    },
    rc::Rc,
    collections::VecDeque,
    time::{
        Instant,
        Duration,
    },
    mem::{
        take,
        replace,
    },
    fmt::Debug,
    iter::once,
    sync::Arc,
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


const CAMERA_HEIGHT: f32 = 1.6;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_WIDTH: f32 = 0.6;

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
}

#[derive(Debug)]
enum MaybePendingChunkMesh {
    ChunkMesh(ChunkMesh),
    Pending(PendingChunkMesh),
}

#[derive(Debug)]
struct PendingChunkMesh {
    abort: MeshChunkAbortHandle,
    buffered_updates: Vec<u16>,
    update_buffered: PerTileBool,
}

#[derive(Debug)]
struct InternalServer {
    server: ServerHandle,
    bind_to_lan: Option<NetworkBindGuard>,
}

#[derive(Debug)]
struct ItemMesh {
    mesh: Mesh,
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
        mut connection: Connection,
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

        let mut inventory_slots = Box::new(array_from_fn(|_| None));
        inventory_slots[7] = Some(ItemStack {
            iid: ctx.game.content.stone.iid_stone.into(),
            meta: ItemMeta::new(()),
            count: 14.try_into().unwrap(),
            damage: 40,
        });

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

            //inventory_slots: Box::new(array_from_fn(|_| RefCell::new(None))),
            inventory_slots,
            inventory_slots_state: Box::new(array_from_fn(|_| ItemSlotGuiState::new())),

            inventory_slots_armor: array_from_fn(|_| None),
            inventory_slots_armor_state: array_from_fn(|_| ItemSlotGuiState::new()),

            inventory_slots_crafting: array_from_fn(|_| None),
            inventory_slots_crafting_state: array_from_fn(|_| ItemSlotGuiState::new()),

            inventory_slot_crafting_output: None,
            inventory_slot_crafting_output_state: ItemSlotGuiState::new(),

            hotbar_slots_state: array_from_fn(|_| ItemSlotGuiStateNoninteractive::new()),
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
        }
        Ok(())
    }

    fn on_network_message_close(&mut self, msg: down::Close) -> Result<()> {
        let down::Close { message } = msg;
        bail!("server closed connection: {:?}", message);
    }

    fn on_network_message_accept_login(&mut self, msg: down::AcceptLogin) -> Result<()> {
        let down::AcceptLogin {} = msg;
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
        debug!(?client_key, vacant_key=?self.clients.vacant_key(), "client adding");
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
            &self.chunks,
            &self.ci_reverse_lookup,
            &mut self.tile_blocks,
            &mut self.block_updates,
        );

        Ok(())
    }
    
    fn on_network_message_ack(&mut self, msg: down::Ack) -> Result<()> {
        let down::Ack { last_processed } = msg;
        self.prediction.process_ack(
            last_processed,
            &self.chunks,
            &self.ci_reverse_lookup,
            &mut self.tile_blocks,
            &mut self.block_updates,
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
                
                self.connection.send(up::SetTileBlock {
                    gtc: tile.gtc(),
                    bid_meta: ctx.game().clone_erased_tile_block(&bid_meta),
                });
                self.prediction.make_prediction(
                    edit::SetTileBlock {
                        lti: tile.lti,
                        bid_meta,
                    }.into(),
                    tile.cc,
                    tile.ci,
                    &getter,
                    &self.connection,
                    &mut self.tile_blocks,
                    &mut self.block_updates,
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
                self.menu_stack.clear();
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
        self.day_night_time += amount.to_pixels(16.0).y / 8000.0;
        self.day_night_time %= 1.0;
    }

    fn on_focus_change(&mut self, ctx: &GuiWindowContext) {
        if ctx.global().focus_level != FocusLevel::MouseCaptured
            && self.menu_stack.is_empty() {
            self.menu_stack.push(Menu::EscMenu);
        }
    }
}

fn cam_dir(pitch: f32, yaw: f32) -> Vec3<f32> {
    Quaternion::rotation_y(-yaw)
        * Quaternion::rotation_x(-pitch)
        * Vec3::new(0.0, 0.0, 1.0)
}

/// GUI block that draws the 3D game world from the player's perspective.
#[derive(Debug)]
struct WorldGuiBlock<'a> {
    // TODO: probably make this just like a substruct within client
    pos: Vec3<f32>,
    pitch: f32,
    yaw: f32,
    pointing: bool,
    load_dist: u8,

    char_mesh: &'a CharMesh,
    char_name_layed_out: &'a LayedOutTextBlock,

    my_client_key: Option<usize>,
    client_char_state: &'a SparseVec<CharState>,
    client_char_name_layed_out: &'a SparseVec<LayedOutTextBlock>,

    day_night_time: f32,
    stars: &'a Mesh,
    white_pixel: &'a GpuImageArray,

    bob_animation: f32,
    third_person: bool,

    chunks: &'a LoadedChunks,
    tile_blocks: &'a PerChunk<ChunkBlocks>,
    tile_meshes: &'a mut PerChunk<MaybePendingChunkMesh>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let SimpleGuiBlock { inner, size, scale: _ } = self;

        // apply any pending chunk tile mesh patches
        for (cc, ci) in inner.chunks.iter() {
            if let &mut MaybePendingChunkMesh::ChunkMesh(ref mut chunk_mesh) = inner.tile_meshes.get_mut(cc, ci) {
                chunk_mesh.patch(&*ctx.global.renderer.borrow());
            }
        }

        // bob animation
        let mut bob_roll = 0.0;
        let mut bob_translate = Vec3::from(0.0);
        if !inner.third_person {
            let bob_animation_sine = f32::sin(inner.bob_animation * 2.0 * PI);
            bob_roll = bob_animation_sine * f32::to_radians(MAX_BOB_ROLL_DEGS);
            let bob_shift = Vec2 {
                x: bob_animation_sine * MAX_BOB_SHIFT_H,
                y: -(bob_animation_sine * bob_animation_sine) * MAX_BOB_SHIFT_V,
            };
            bob_translate = Vec3 {
                x: f32::cos(inner.yaw) * bob_shift.x,
                y: bob_shift.y,
                z: f32::sin(inner.yaw) * bob_shift.x,
            };
        }

        // determine view proj
        let cam_dir = cam_dir(inner.pitch, inner.yaw);
        let mut cam_pos = inner.pos + Vec3::new(0.0, CAMERA_HEIGHT, 0.0) + bob_translate;
        if inner.third_person {
            cam_pos -= cam_dir * 5.0;
        }
        let view_proj = ViewProj::perspective(
            // position
            cam_pos,
            // direction
            Quaternion::rotation_x(inner.pitch)
            * Quaternion::rotation_z(bob_roll)
            * Quaternion::rotation_y(inner.yaw),
            // field of view
            f32::to_radians(120.0),
            // size
            size,
        );

        // determine fog
        let fog = match ctx.settings().fog {
            true => Fog::Earth {
                start: 100.0,
                end: 150.0,
                day_night_time: inner.day_night_time,
            },
            false => Fog::None,
        };

        // draw sky
        canvas.reborrow()
            .scale(self.size)
            .draw(DrawObj2::Sky(DrawSky {
                view_proj,
                day_night_time: inner.day_night_time,
            }));

        // draw stars
        // intensity of it being day as opposed to night
        let day = (f32::sin(inner.day_night_time * PI * 2.0) + 0.6).clamp(0.0, 1.0);
        canvas.reborrow()
            .scale(self.size)
            .begin_3d(view_proj, Fog::None)
            .translate(cam_pos)
            .rotate(Quaternion::rotation_x(-inner.day_night_time * PI * 2.0))
            .color([1.0, 1.0, 1.0, 1.0 - day])
            .draw_mesh(inner.stars, inner.white_pixel);

        // draw sun and moon
        {
            let mut canvas = canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj, Fog::None)
                .translate(cam_pos)
                .rotate(Quaternion::rotation_x(-inner.day_night_time * PI * 2.0));
            let sun_moon_transl = Vec3::new(-0.5, -0.5, 1.6);
            //let sun_oversat = 0.22 + day * 1.3;
            let sun_oversat = (day + 1.0).powf(2.0) - 0.8;
            canvas.reborrow()
                .translate(sun_moon_transl)
                .color([sun_oversat, sun_oversat, sun_oversat, 1.0])
                .draw_image(&ctx.assets().sun, 0, 0.0, 1.0);
            canvas.reborrow()
                .rotate(Quaternion::rotation_x(PI))
                .translate(sun_moon_transl)
                .draw_image(&ctx.assets().moon, 0, 0.0, 1.0);
        }
        

        // begin 3D perspective
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(view_proj, fog);

        // chunks
        for (cc, ci) in inner.chunks.iter() {
            // frustum culling
            let pos = (cc * CHUNK_EXTENT).map(|n| n as f32);
            let ext = CHUNK_EXTENT.map(|n| n as f32).into();
            if !view_proj.is_volume_visible(pos, ext) {
                continue;
            }

            // blocks
            if let &MaybePendingChunkMesh::ChunkMesh(ref chunk_mesh) = (&*inner.tile_meshes).get(cc, ci) {
                canvas.reborrow()
                    .translate(pos)
                    .draw_mesh(chunk_mesh.mesh(), &ctx.assets().blocks);
            }

            // debug outline
            if ctx.settings().chunk_outline {
                draw_debug_box(&mut canvas, pos, ext);
            }
        }

        // my character
        if inner.third_person {
            let mut canvas = canvas.reborrow()
                .translate(inner.pos)
                .rotate(Quaternion::rotation_y(-inner.yaw));
            inner.char_mesh.draw(&mut canvas, ctx.assets(), inner.pitch, inner.pointing);
            canvas.reborrow()
                .translate([0.0, 2.0, 0.0])
                .scale(0.25 / 16.0)
                .scale([1.0, -1.0, 1.0])
                .rotate(Quaternion::rotation_y(PI))
                .color([1.0, 1.0, 1.0, 0.5])
                .draw_text(&inner.char_name_layed_out);
        }

        // other characters
        if let Some(my_client_key) = inner.my_client_key {
            for (client_key, client_char_state) in inner.client_char_state.iter() {
                if client_key == my_client_key {
                    continue;
                }

                // TODO: deduplicate this part with above
                let mut canvas = canvas.reborrow()
                    .translate(client_char_state.pos)
                    .rotate(Quaternion::rotation_y(-client_char_state.yaw));
                inner.char_mesh.draw(&mut canvas, ctx.assets(), client_char_state.pitch, client_char_state.pointing);
                canvas.reborrow()
                    .translate([0.0, 2.0, 0.0])
                    .scale(0.25 / 16.0)
                    .scale([1.0, -1.0, 1.0])
                    .rotate(Quaternion::rotation_y(PI))
                    .draw_text(&inner.client_char_name_layed_out[client_key]);
            }
        }

        // outline for block being looked at
        let getter = inner.chunks.getter();
        if let Some(looking_at) = compute_looking_at(
            // pos
            cam_pos,
            // dir
            cam_dir,
            // reach
            50.0,
            &getter,
            &inner.tile_blocks,
            ctx.game(),
        ) {
            const GAP: f32 = 0.002;

            let mut canvas = canvas.reborrow()
                .translate(looking_at.tile.gtc().map(|n| n as f32))
                .color([0.0, 0.0, 0.0, 0.65]);

            for face in FACES {
                for edge in face.to_edges() {
                    let [start, end] = edge.to_corners()
                        .map(|corner| corner.to_poles()
                            .map(|pole| match pole {
                                Pole::Neg => 0.0 + GAP,
                                Pole::Pos => 1.0 - GAP,
                            })
                            + face.to_vec().map(|n| n as f32) * 2.0 * GAP);
                    canvas.reborrow()
                        .draw_line(start, end);
                }
            }
        }

        // debug box for load dist
        if ctx.settings().load_dist_outline {
            let load_dist = inner.load_dist as f32;
            let chunk_ext = CHUNK_EXTENT.map(|n| n as f32);

            let mut load_cc_start = (inner.pos / chunk_ext).map(|n| n.floor()) - load_dist;
            load_cc_start.y = 0.0;
            let mut load_cc_ext = Vec3::from(1.0 + load_dist * 2.0);
            load_cc_ext.y = 2.0;

            draw_debug_box(
                &mut canvas,
                load_cc_start * chunk_ext,
                load_cc_ext * chunk_ext,
            );
        }
    }
}

fn draw_debug_box(canvas: &mut Canvas3, pos: impl Into<Vec3<f32>>, ext: impl Into<Vec3<f32>>) {
    let ext = ext.into();
    let mut canvas = canvas.reborrow()
        .translate(pos.into())
        .color(Rgba::red());
    for edge in EDGES {
        let [start, end] = edge.to_corners()
            .map(|corner| corner.to_poles()
                .map(|pole| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => 1.0,
                }) * ext);
        canvas.reborrow()
            .draw_line(start, end);
    }
}

// ==== menu stuff ====

#[derive(Debug)]
struct MenuResources {
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
    fn new(ctx: &GuiGlobalContext) -> Self {
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

    fn process_effect_queue(&mut self, menu_stack: &mut Vec<Menu>) {
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

type MenuEffectQueue = RefCell<VecDeque<MenuEffect>>;

#[derive(Debug)]
#[allow(dead_code)]
enum MenuEffect {
    PopMenu,
    PushMenu(Menu),
}

/// Menu that can be opened over the world. Different from in-world GUIs. Form
/// a stack.
#[derive(Debug)]
enum Menu {
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
        gtc: Vec3<i64>,
    }
}

impl Menu {
    fn gui<'a>(
        &'a mut self,
        resources: &'a mut MenuResources,
        chat: &mut Option<&'a mut GuiChat>,
        internal_server: &'a mut Option<InternalServer>,
        items_mesh: &'a PerItem<ItemMesh>,

        held_item: &'a RefCell<ItemSlot>,
        held_item_state: &'a mut ItemSlotGuiStateNoninteractive,

        inventory_slots_bottom: &[Rc<RefCell<&'a mut ItemSlot>>; 9],
        inventory_slots_top: &'a mut [ItemSlot],
        inventory_slots_state: &'a mut Box<[ItemSlotGuiState; 36]>,

        inventory_slots_armor: &'a mut [ItemSlot; 4],
        inventory_slots_armor_state: &'a mut [ItemSlotGuiState; 4],
        
        inventory_slots_crafting: &'a mut [ItemSlot; 4],
        inventory_slots_crafting_state: &'a mut [ItemSlotGuiState; 4],

        inventory_slot_crafting_output: &'a mut ItemSlot,
        inventory_slot_crafting_output_state: &'a mut ItemSlotGuiState,

        char_mesh: &'a CharMesh,
        head_pitch: f32,
        pointing: bool,

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
                        margin(14.0, 0.0, 282.0, 0.0,
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
                        ),
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
                                        solid(CHAT_BACKGROUND),
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
            &mut Menu::Chest { gtc } => GuiEither::B(
                align(0.5,
                    logical_size([352.0, 444.0],
                        layer((
                            &ctx.assets().gui_chest,
                            margin(14.0, 0.0, 278.0, 0.0,
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
                            
                            margin(14.0, 0.0, 394.0, 0.0,
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
                            ),
                            
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

    fn exitable_via_inventory_button(&self) -> bool {
        match self {
            &Menu::EscMenu => false,
            &Menu::Inventory => true,
            &Menu::ChatInput { .. } => false,
            &Menu::Settings => false,
            &Menu::Chest { .. } => true,
        }
    }

    fn has_darkened_background(&self) -> bool {
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

#[derive(Debug)]
struct CharMeshGuiBlock<'a> {
    char_mesh: &'a CharMesh,
    head_pitch: f32,
    pointing: bool,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<CharMeshGuiBlock<'a>> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(
                ViewProj::orthographic(
                    [0.0, PLAYER_HEIGHT / 2.0, -5.0],
                    Quaternion::identity(),
                    3.2,
                    self.size
                ),
                Fog::None,
            )
            .rotate(Quaternion::rotation_y(PI));
        self.inner.char_mesh.draw(&mut canvas, ctx.assets(), self.inner.head_pitch, self.inner.pointing);
    }
}

/*
/// Like layer but works with an iterator of GuiBlock rather than a
/// GuiBlockSeq. GuiBlockSeq is more difficult to implement than an iterator,
/// but those difficulties can be bypassed in cases where the sequence length
/// or the elements sizing logic don't affect the sizing logic of the parent,
/// such as in this case.
fn iter_layer<'a, I>(iter: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: DoubleEndedIterator,
    <I as IntoIterator>::Item: GuiBlock<'a, DimParentSets, DimParentSets>,
{
    IterLayer(iter.into_iter())
}


#[derive(Debug)]
struct IterLayer<I>(I);

impl<
    'a,
    I: Iterator + DoubleEndedIterator,
> GuiBlock<'a, DimParentSets, DimParentSets> for IterLayer<I>
where
    <I as Iterator>::Item: GuiBlock<'a, DimParentSets, DimParentSets>,
{
    type Sized = IterLayerSized<I>;

    fn size(
        self,
        _: &GuiGlobalContext<'a>,
        w: f32,
        h: f32,
        scale: f32,
    ) -> ((), (), Self::Sized) {
        ((), (), IterLayerSized {
            iter: self.0,
            w,
            h,
            scale,
        })
    }
}


#[derive(Debug)]
struct IterLayerSized<I> {
    iter: I,
    w: f32,
    h: f32,
    scale: f32,
}

impl<
    'a,
    I: Iterator + DoubleEndedIterator,
> SizedGuiBlock<'a> for IterLayerSized<I>
where
    <I as Iterator>::Item: GuiBlock<'a, DimParentSets, DimParentSets>,
{
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        if forward {
            for block in self.iter {
                block
                    .size(visitor.ctx.global, self.w, self.h, self.scale)
                    .2.visit_nodes(visitor, true);
            }
        } else {
            for block in self.iter.rev() {
                block
                    .size(visitor.ctx.global, self.w, self.h, self.scale)
                    .2.visit_nodes(visitor, false);
            }
        }
    }
}
*/

#[derive(Debug)]
struct ItemSlotGuiStateNoninteractive {
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
struct ItemSlotGuiState {
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

#[derive(Debug, Copy, Clone)]
struct ItemGridConfig {
    /// Makes slots bigger than their default logical size of 32 
    slot_scale: f32,
    /// Logical padding around slots.
    pad: f32,
}

impl Default for ItemGridConfig {
    fn default() -> Self {
        ItemGridConfig {
            slot_scale: 1.0,
            pad: 2.0,
        }
    }
}

const SLOT_DEFAULT_SLOT_SIZE: f32 = 32.0;
const SLOT_DEFAULT_TEXT_SIZE: f32 = 16.0;

#[derive(Debug)]
struct ItemGrid<'a, I1, I2, C> {
    slots: I1,
    slots_state: I2,
    click_logic: C,
    grid_size: Extent2<u32>,
    config: ItemGridConfig,
    items_mesh: &'a PerItem<ItemMesh>,
}

#[derive(Debug)]
struct ItemGridSized<'a, I1, I2, C> {
    inner: ItemGrid<'a, I1, I2, C>,
    scale: f32,
}

impl<
    'a,
    I1: IntoIterator + Debug,
    I2: IntoIterator + Debug,
    C: ItemSlotClickLogic + Debug,
> GuiBlock<'a, DimChildSets, DimChildSets> for ItemGrid<'a, I1, I2, C>
where
    <I1 as IntoIterator>::Item: BorrowItemSlot,
    <I2 as IntoIterator>::Item: ItemSlotGuiStateGeneral<'a, <I1 as IntoIterator>::Item>,
{
    type Sized = ItemGridSized<'a, I1, I2, C>;

    fn size(
        self,
        _: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        let size = self.grid_size.map(|n| n as f32)
            * (SLOT_DEFAULT_SLOT_SIZE * self.config.slot_scale + self.config.pad * 2.0)
            * scale;
        (size.w, size.h, ItemGridSized {
            inner: self,
            scale,
        })
    }
}

#[derive(Debug, Copy, Clone)]
struct ItemSlotLayoutCalcs {
    // side length of each slot not including pad 
    slot_inner_size: f32,
    // thickness of pad around each slot
    pad_size: f32,
    // side length of each slot including pad
    slot_outer_size: f32,
}

impl ItemSlotLayoutCalcs {
    fn new(scale: f32, config: &ItemGridConfig) -> Self {
        let slot_inner_size = SLOT_DEFAULT_SLOT_SIZE * config.slot_scale * scale;
        let pad_size = config.pad * scale;
        let slot_outer_size = slot_inner_size + pad_size * 2.0;

        ItemSlotLayoutCalcs {
            slot_inner_size,
            pad_size,
            slot_outer_size,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct ItemGridLayoutCalcs {
    inner: ItemSlotLayoutCalcs,
    // size of entire grid
    size: Extent2<f32>,
    // grid coordinates of moused-over slot
    cursor_over: Option<Vec2<u32>>,
}

impl ItemGridLayoutCalcs {
    fn new(
        ctx: GuiSpatialContext,
        scale: f32,
        grid_size: Extent2<u32>,
        config: &ItemGridConfig,
    ) -> Self {
        let inner = ItemSlotLayoutCalcs::new(scale, config);
        let size = grid_size.map(|n| n as f32) * inner.slot_outer_size;

        let cursor_over = ctx.cursor_pos
            .map(|pos| pos / inner.slot_outer_size)
            .map(|xy| xy.map(|n| n.floor() as i64))
            .filter(|xy| xy
                .zip::<u32>(grid_size.into())
                .map(|(n, bound)| n >= 0 && n < bound as i64)
                .reduce_and())
            .map(|xy| xy.map(|n| n as u32));

        ItemGridLayoutCalcs {
            inner,
            size,
            cursor_over,
        }
    }
}

#[derive(Debug)]
struct HeldItemGuiBlock<'a, H> {
    held: H,
    held_state: &'a mut ItemSlotGuiStateNoninteractive,
    items_mesh: &'a PerItem<ItemMesh>,
}

#[derive(Debug)]
struct HeldItemGuiBlockSized<'a, H> {
    inner: HeldItemGuiBlock<'a, H>,
    scale: f32,
}

impl<
    'a,
    H: BorrowItemSlot + Debug,
> GuiBlock<'a, DimParentSets, DimParentSets> for HeldItemGuiBlock<'a, H> {
    type Sized = HeldItemGuiBlockSized<'a, H>;

    fn size(self, _: &GuiGlobalContext, _: f32, _: f32, scale: f32) -> ((), (), Self::Sized) {
        ((), (), HeldItemGuiBlockSized { inner: self, scale })
    }
}

impl<
    'a,
    H: BorrowItemSlot + Debug,
> GuiNode<'a> for HeldItemGuiBlockSized<'a, H> {
    never_blocks_cursor_impl!();

    fn draw(mut self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        if let Some(pos) = ctx.cursor_pos {
            let layout = ItemSlotLayoutCalcs::new(self.scale, &ItemGridConfig::default());
            let mut canvas = canvas.reborrow()
                .translate(pos)
                .translate(-layout.slot_outer_size / 2.0);
            let mut held_guard = self.inner.held.borrow();
            let held = H::deref(&mut held_guard);
            draw_item_noninteractive(
                ctx,
                &mut canvas,
                self.scale,
                &layout,
                held.as_ref(),
                self.inner.held_state,
                self.inner.items_mesh,
            );
        }
    }
}

fn draw_item_noninteractive<'a>(
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

#[derive(Debug)]
struct ItemNameDrawer<'a, I> {
    borrow_slot: I,
    cached_iid: &'a mut Option<RawItemId>,
    name_text: &'a mut Option<GuiTextBlockInner>,
}

impl<'a, I: BorrowItemSlot> ItemNameDrawer<'a, I> {
    fn draw(
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

trait ItemSlotClickLogic {
    fn on_click(
        self,
        slot: &mut ItemSlot,
        button: MouseButton,
        game: &Arc<GameData>,
    );
}

#[derive(Debug, Copy, Clone)]
struct NoninteractiveItemSlotClickLogic;

impl ItemSlotClickLogic for NoninteractiveItemSlotClickLogic {
    fn on_click(
        self,
        _slot: &mut ItemSlot,
        _button: MouseButton,
        _game: &Arc<GameData>,
    ) {}
}

#[derive(Debug, Copy, Clone)]
struct StorageItemSlotClickLogic<H> {
    held: H,
}

impl<H: BorrowItemSlot> ItemSlotClickLogic for StorageItemSlotClickLogic<H> {
    fn on_click(
        mut self,
        slot_mut: & mut ItemSlot,
        button: MouseButton,
        game: &Arc<GameData>,
    ) {
        // borrow
        let mut held_guard = self.held.borrow();
        let mut held_mut = H::deref(&mut held_guard);

        if button == MouseButton::Left {
            // left click
            // take ownership of both stacks, remember to put them back if we want to
            match (held_mut.take(), slot_mut.take()) {
                (Some(mut held), Some(mut slot)) => {
                    // both held and slot have stack
                    if held.iid == slot.iid
                        && held.meta == slot.meta
                        && held.damage == slot.damage
                    {
                        // stacks have same item

                        // number of items to transfer from held to slot
                        let transfer = u8::min(
                            // number of items in held
                            held.count.get(),
                            // number of additional items slot could receive
                            game.items_max_count[slot.iid].get().saturating_sub(slot.count.get()),
                        );

                        // add to slot, give back ownership
                        slot.count = (slot.count.get() + transfer).try_into().unwrap();
                        *slot_mut = Some(slot);

                        // subtract from held, give back ownership or leave it none
                        if let Ok(held_new_count) = (held.count.get() - transfer).try_into() {
                            held.count = held_new_count;
                            *held_mut = Some(held)
                        }
                    } else {
                        // stacks have different items
                        // swap them
                        *held_mut = Some(slot);
                        *slot_mut = Some(held);
                    }
                }
                (opt_held, opt_slot) => {
                    // otherwise, swap them (regardless of further specifics)
                    *held_mut = opt_slot;
                    *slot_mut = opt_held;
                }
            }
        } else if button == MouseButton::Right {
            // right click
            // take ownership of both stacks, remember to put them back if we want to
            match (held_mut.take(), slot_mut.take()) {
                (Some(mut held), Some(mut slot)) => {
                    // both held and slot have stack
                    if held.iid == slot.iid
                        && held.meta == slot.meta
                        && held.damage == slot.damage
                    {
                        // stacks have same item
                        if let Some(slot_new_count) = slot.count.get()
                            .checked_add(1)
                            .filter(|&n| n <= game.items_max_count[held.iid].get())
                        {
                            // slot has room for another item
                            
                            // add to slot, give back ownership
                            slot.count = slot_new_count.try_into().unwrap();
                            *slot_mut = Some(slot);

                            // subtract from held, give back ownership or leave it none
                            if let Ok(held_new_count) = (held.count.get() - 1).try_into() {
                                held.count = held_new_count;
                                *held_mut = Some(held)
                            }
                        } else {
                            // slot is full
                            // give back ownership of both without modifying
                            *held_mut = Some(held);
                            *slot_mut = Some(slot);
                        }
                    } else {
                        // stacks have different items
                        // swap them
                        *held_mut = Some(slot);
                        *slot_mut = Some(held);
                    }
                }
                (Some(mut held), None) => {
                    // only held has stack

                    // put one item in slot
                    *slot_mut = Some(ItemStack {
                        iid: held.iid,
                        meta: held.meta.clone(),
                        count: 1.try_into().unwrap(),
                        damage: held.damage,
                    });

                    // subtract from held, give back ownership or leave it none
                    if let Ok(held_new_count) = (held.count.get() - 1).try_into() {
                        held.count = held_new_count;
                        *held_mut = Some(held);
                    }

                }
                (None, Some(mut slot)) => {
                    // only slot has stack

                    // amount to leave = half, round down
                    let slot_new_count = slot.count.get() / 2;
                    // amount to take = half, round up
                    let held_new_count = slot.count.get() - slot_new_count;

                    // put in held
                    *held_mut = Some(ItemStack {
                        iid: slot.iid,
                        meta: slot.meta.clone(),
                        count: held_new_count.try_into().unwrap(),
                        damage: slot.damage,
                    });

                    // subtract from slot, give back ownership or leave it none
                    if let Ok(slot_new_count) = slot_new_count.try_into() {
                        slot.count = slot_new_count;
                        *slot_mut = Some(slot)
                    }
                }
                (None, None) => {} // both are empty, nothing to do
            }
        }
    }
}

trait ItemSlotGuiStateGeneral<'a, I> {
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

trait BorrowItemSlot {
    type Guard<'a>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a>;

    fn deref<'g, 'a>(guard: &'g mut Self::Guard<'a>) -> &'g mut ItemSlot;
}

impl<'b> BorrowItemSlot for &'b mut ItemSlot {
    type Guard<'a> = &'a mut ItemSlot
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        &mut **self
    }

    fn deref<'g, 'a>(mut guard: &'g mut &'a mut ItemSlot) -> &'g mut ItemSlot {
        &mut **guard
    }
}

impl<'b> BorrowItemSlot for &'b RefCell<ItemSlot> {
    type Guard<'a> = cell::RefMut<'a, ItemSlot>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        RefCell::borrow_mut(&**self)
    }

    fn deref<'g, 'a>(mut guard: &'g mut cell::RefMut<'a, ItemSlot>) -> &'g mut ItemSlot {
        &mut **guard
    }
}

impl<'b> BorrowItemSlot for Rc<RefCell<&'b mut ItemSlot>> {
    type Guard<'a> = cell::RefMut<'a, &'b mut ItemSlot>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        RefCell::borrow_mut(&**self)
    }

    fn deref<'g, 'a>(mut guard: &'g mut cell::RefMut<'a, &'b mut ItemSlot>) -> &'g mut ItemSlot {
        &mut ***guard
    }
}

impl<
    'a,
    I1: IntoIterator + Debug,
    I2: IntoIterator + Debug,
    C: ItemSlotClickLogic + Debug,
> GuiNode<'a> for ItemGridSized<'a, I1, I2, C>
where
    <I1 as IntoIterator>::Item: BorrowItemSlot,
    <I2 as IntoIterator>::Item: ItemSlotGuiStateGeneral<'a, <I1 as IntoIterator>::Item>,
{
    fn blocks_cursor(&self, ctx: GuiSpatialContext) -> bool {
        let &ItemGridSized { ref inner, scale } = self;
        let size = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config).size;
        ctx.cursor_in_area(0.0, size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let ItemGridSized { inner, scale } = self;
        
        // layout calcs
        let layout = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config);

        let mut draw_cursor_over_state = None;

        // render slots
        let mut slots = inner.slots.into_iter();
        let mut slots_state = inner.slots_state.into_iter();

        for y in 0..inner.grid_size.h {
            for x in 0..inner.grid_size.w {
                let xy = Vec2 { x, y };

                let mut borrow_slot = slots.next()
                    .expect("ItemGrid slots produced None when expected Some");
                let slot_state = slots_state.next()
                    .expect("ItemGrid slots_state produced None when expected Some");

                //let mut slot_guard = borrow_slot.borrow();
                //let slot = <<I1 as IntoIterator>::Item as BorrowItemSlot<'_>>::deref(&mut slot_guard);

                let mut canvas = canvas.reborrow()
                    .translate(xy.map(|n| n as f32) * layout.inner.slot_outer_size);

                // debug background
                if false {
                    canvas.reborrow()
                        .translate(layout.inner.pad_size)
                        .color([1.0, 0.0, 0.0, 0.5])
                        .draw_solid(layout.inner.slot_inner_size);
                }

                let curr_draw_cursor_over_state = slot_state.draw(
                    ctx,
                    &mut canvas,
                    scale,
                    &layout,
                    borrow_slot,
                    inner.items_mesh,
                );

                if layout.cursor_over == Some(xy) {
                    draw_cursor_over_state = Some(curr_draw_cursor_over_state);
                }
            }
        }

        // specifics for moused over slot
        if let Some(xy) = layout.cursor_over {
            <<I2 as IntoIterator>::Item as ItemSlotGuiStateGeneral<_>>::draw_cursor_over(
                draw_cursor_over_state.unwrap(),
                ctx,
                canvas,
                scale,
                xy,
                &layout,
            );
        }
    }

    fn on_cursor_click(self, ctx: GuiSpatialContext, hits: bool, button: MouseButton) {
        let ItemGridSized { inner, scale } = self;
        
        // layout calculation
        let cursor_over = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config).cursor_over;

        // calculate which slot clicked, or return
        if !hits { return }
        let xy = match cursor_over {
            Some(xy) => xy,
            None => return,
        };

        // convert to index and get actual slot
        let i = xy.y as usize * inner.grid_size.w as usize + xy.x as usize;
        let mut borrow_slot = inner.slots.into_iter().nth(i)
            .expect("ItemGrid slots produced None when expected Some");

        let mut slot_guard = borrow_slot.borrow();
        let slot = <<I1 as IntoIterator>::Item as BorrowItemSlot>::deref(&mut slot_guard);
        
        inner.click_logic.on_click(slot, button, ctx.game());
    }
}


// ==== chat stuff ====

const CHAT_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0 - 0x6f as f32 / 0xde as f32];


#[derive(Debug)]
struct GuiChat {
    lines: VecDeque<GuiChatLine>,
}

#[derive(Debug)]
struct GuiChatLine {
    text_block: GuiTextBlock<true>,
    added: Duration, // TODO: make some sort of epoch time newtype?
}

impl GuiChat {
    pub fn new() -> Self {
        GuiChat {
            lines: VecDeque::new(),
        }
    }

    pub fn add_line(&mut self, line: String, ctx: &GuiGlobalContext) {
        self.lines.push_back(GuiChatLine {
            text_block: GuiTextBlock::new(&GuiTextBlockConfig {
                text: &line,
                font: ctx.assets.font,
                logical_font_size: 16.0,
                color: hex_color(0xfbfbfbff),
                h_align: HAlign::Left,
                v_align: VAlign::Top,
            }),
            added: ctx.time_since_epoch,
        });
    }

    fn gui<'a>(&'a mut self, limit: bool) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        let lines = if limit {
            self.lines.range_mut(self.lines.len().saturating_sub(10)..)
        } else {
            self.lines.range_mut(0..)
        };

        logical_width(664.0,
            v_stack(0.0,
                lines.map(|chat_line| {
                    let line_gui = before_after(
                        (
                            solid(CHAT_BACKGROUND),
                        ),
                        v_pad(2.0, 2.0,
                            h_margin(8.0, 8.0,
                                &mut chat_line.text_block
                            )
                        ),
                        (),
                    );
                    if limit {
                        GuiEither::A(fade(chat_line.added + Duration::from_secs(10), 1.0,
                            line_gui
                        ))
                    } else {
                        GuiEither::B(line_gui)
                    }
                })
                .collect::<Vec<_>>()
            )
        )
    }
}

fn fade<'a, W, H, I>(
    fade_at: Duration,
    fade_for_secs: f32,
    inner: I,
) -> impl GuiBlock<'a, W, H>
where
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
{
    Fade {
        fade_at,
        fade_for_secs,
        inner,
    }
}

struct Fade<I> {
    fade_at: Duration,
    fade_for_secs: f32,
    inner: I,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
> GuiBlock<'a, W, H> for Fade<I> {
    type Sized = FadeSized<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized) {
        let alpha = if ctx.time_since_epoch > self.fade_at {
            let secs_faded_for = (ctx.time_since_epoch - self.fade_at).as_secs_f32();
            if secs_faded_for < self.fade_for_secs {
                1.0 - secs_faded_for / self.fade_for_secs
            } else {
                0.0
            }
        } else {
            1.0
        };


        let (w_out, h_out, inner_sized) = self.inner.size(ctx, w_in, h_in, scale);
        (w_out, h_out, FadeSized {
            alpha,
            inner: inner_sized,
        })
    }
}

struct FadeSized<I> {
    alpha: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for FadeSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let mut visitor = visitor.reborrow()
            .color([1.0, 1.0, 1.0, self.alpha]);
        self.inner.visit_nodes(&mut visitor, forward);
    }
}


fn make_chat_input_text_block(text: &str, blinker: bool, ctx: &GuiGlobalContext) -> GuiTextBlock<true> {
    let mut text = format!("saying: {}", text);
    if blinker {
        text.push('_');
    }

    GuiTextBlock::new(&GuiTextBlockConfig {
        text: &text,
        font: ctx.assets.font,
        logical_font_size: 16.0,
        color: hex_color(0xfbfbfbff),
        h_align: HAlign::Left,
        v_align: VAlign::Top,
    })
}


// ==== mob meshes ===

/// Utility for making mob body part meshes.
#[derive(Debug)]
struct MobMesher<G> {
    gpu_vec_ctx: G,
    tex_size: Extent2<u32>,
}

impl<G> MobMesher<G>
where
    G: Deref,
    G::Target: GpuVecContext
{
    pub fn make_part(
        &self,
        offset: impl Into<Vec2<u32>>,
        extent: impl Into<Extent3<u32>>,
        origin_frac: impl Into<Vec3<f32>>,
    ) -> Mesh {
        // convert stuff into vectors
        let offset = offset.into();
        let extent = Vec3::<u32>::from(extent.into());
        let origin_frac = origin_frac.into();

        // convert stuff into floats
        let tex_size = self.tex_size.map(|n| n as f32);
        let offset = offset.map(|n| n as f32);
        let extent = extent.map(|n| n as f32);

        // compose mesh from faces
        let mut mesh = MeshData::new();
        let scaled_extent = Vec3::from(extent);
        let origin_adjust = -(extent * origin_frac);
        for face in FACES {
            let (face_start, face_extents) = face.quad_start_extents();
            let pos_start = face_start
                .to_poles()
                .zip(scaled_extent)
                .map(|(pole, n)| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => n,
                }) + origin_adjust;
            let [pos_ext_1, pos_ext_2] = face_extents
                .map(|ext_face| {
                    let (ext_axis, ext_pole) = ext_face.to_axis_pole();
                    let n = PerAxis::from(scaled_extent)[ext_axis] * ext_pole.to_int() as f32;
                    ext_axis.to_vec(n)
                });
            let tex_start =
                (offset + Vec2::from(match face {
                    Face::PosX => [0.0, extent.z],
                    Face::NegX => [extent.z + extent.x, extent.z],
                    Face::PosY => [extent.z, 0.0],
                    Face::NegY => [extent.z + extent.x, 0.0],
                    Face::PosZ => [extent.z, extent.z],
                    Face::NegZ => [extent.z * 2.0 + extent.x, extent.z],
                })) / tex_size;
            let tex_extent = Vec2::from(face
                .to_axis()
                .other_axes()
                .map(
                    |axis| PerAxis::from(extent)[axis]
                )) / tex_size;

            mesh.add_quad(&Quad {
                pos_start,
                pos_ext_1: pos_ext_1.into(),
                pos_ext_2: pos_ext_2.into(),
                tex_start,
                tex_extent: tex_extent.into(),
                vert_colors: [Rgba::white(); 4],
                tex_index: 0,
            });
        }

        // upload
        mesh.upload(&*self.gpu_vec_ctx)
    }
}

#[derive(Debug)]
struct CharMesh {
    head: Mesh,
    torso: Mesh,
    leg: Mesh,
    arm: Mesh,
}

impl CharMesh {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let char_mesher = MobMesher {
            gpu_vec_ctx: ctx.renderer.borrow(),
            tex_size: [64, 32].into(),
        };
        CharMesh {
            head: char_mesher.make_part([0, 0], [8, 8, 8], [0.5, 0.5, 0.5]),
            torso: char_mesher.make_part([16, 16], [8, 12, 4], [0.5, 0.0, 0.5]),
            leg: char_mesher.make_part([0, 16], [4, 12, 4], [0.5, 10.0 / 12.0, 0.5]),
            arm: char_mesher.make_part([40, 16], [4, 12, 4], [0.5, 10.0 / 12.0, 0.5]),
        }
    }

    pub fn draw<'a>(
        &'a self,
        canvas: &mut Canvas3<'a, '_>,
        assets: &'a Assets,
        head_pitch: f32,
        pointing: bool,
    ) {
        let mut canvas = canvas.reborrow()
            .scale(PLAYER_HEIGHT / 32.0);
        canvas.reborrow()
            .translate([0.0, 12.0, 0.0])
            .draw_mesh(&self.torso, &assets.mob_char);
        canvas.reborrow()
            .translate([0.0, 28.0, 0.0])
            .rotate(Quaternion::rotation_x(-head_pitch))
            .draw_mesh(&self.head, &assets.mob_char);
        canvas.reborrow()
            .translate([-2.0, 10.0, 0.0])
            .draw_mesh(&self.leg, &assets.mob_char);
        canvas.reborrow()
            .translate([2.0, 10.0, 0.0])
            .draw_mesh(&self.leg, &assets.mob_char);
        canvas.reborrow()
            .translate([-6.0, 22.0, 0.0])
            .draw_mesh(&self.arm, &assets.mob_char);
        let mut arm_pitch = 0.0;
        if pointing {
            arm_pitch -= head_pitch;
            arm_pitch -= PI / 2.0;
        }
        canvas.reborrow()
            .translate([6.0, 22.0, 0.0])
            .rotate(Quaternion::rotation_x(arm_pitch))
            .draw_mesh(&self.arm, &assets.mob_char);
    }
}


// ==== other stuff ====

/// GUI block for rendering the vignette.
#[derive(Debug)]
struct Vignette;

impl<'a> GuiNode<'a> for SimpleGuiBlock<Vignette> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        if !ctx.global.pressed_keys_semantic.contains(&VirtualKeyCode::F1) {
            canvas.reborrow()
                .color([1.0, 1.0, 1.0, 1.0 - 0x5b as f32 / 0x7f as f32])
                .draw_image(
                    &ctx.assets().vignette,
                    0,
                    self.size,
                );
        }
    }
}

/// GUI block for rendering the crosshair.
#[derive(Debug)]
struct Crosshair;

impl<'a> GuiNode<'a> for SimpleGuiBlock<Crosshair> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        canvas.reborrow()
            .scale(self.size)
            .draw(DrawObj2::Invert(DrawInvert {
                image: ctx.assets().hud_crosshair.clone(),
                tex_index: 0,
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
            }));
    }
}
