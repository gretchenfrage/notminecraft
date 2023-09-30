
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
use super::message::*;
use crate::{
    asset::Assets,
    block_update_queue::BlockUpdateQueue,
    chunk_mesh::ChunkMesh,
    gui::prelude::*,
    util::sparse_vec::SparseVec,
    physics::prelude::*,
    util::{
        hex_color::hex_color,
        secs_rem::secs_rem,
    },
    settings::Settings,
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
    cell::RefCell,
    collections::VecDeque,
    time::{
        Instant,
        Duration,
    },
    mem::{
        take,
        replace,
    },
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

    chat: GuiChat,

    day_night_time: f32,
    white_pixel: GpuImageArray,
    stars: Mesh,
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
    pub fn connect(address: &str, ctx: &GuiGlobalContext) -> Self {
        let connection = Connection::connect(address, ctx.tokio, ctx.game);
        Self::new(connection, ctx)
    }

    pub fn new(mut connection: Connection, ctx: &GuiGlobalContext) -> Self {
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

        ctx.capture_mouse();

        Client {
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

            chat: GuiChat::new(),

            day_night_time: 0.1,
            white_pixel,
            stars,
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        const MENU_DARKENED_BACKGROUND_ALPHA: f32 = 1.0 - 0x2a as f32 / 0x97 as f32;

        let mut chat = Some(&mut self.chat);
        let menu_gui = self.menu_stack.iter_mut().rev().next()
            .map(|open_menu| layer((
                if open_menu.has_darkened_background() {
                    Some(solid([0.0, 0.0, 0.0, MENU_DARKENED_BACKGROUND_ALPHA]))
                } else { None },
                open_menu.gui(&mut self.menu_resources, &mut chat, ctx),
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
        let down::Close {} = msg;
        bail!("server closed connection");
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
                    let gtc = looking_at.tile.gtc() + looking_at.face
                        .map(|face| face.to_vec())
                        .unwrap_or(0.into());
                    getter.gtc_get(gtc).map(|tile| (
                        tile,
                        ErasedBidMeta::new(ctx.global().game.content_stone.bid_stone, ()),
                        true,
                    ))
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
                //self.menu_stack.pop();
                //if self.menu_stack.is_empty() {
                //    ctx.global().capture_mouse();
                //}
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
                    //self.chat.add_line(format!("<me> {}", text), ctx.global());
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

    bob_animation: f32,
    third_person: bool,

    chunks: &'a LoadedChunks,
    tile_blocks: &'a PerChunk<ChunkBlocks>,
    tile_meshes: &'a mut PerChunk<MaybePendingChunkMesh>,

    char_mesh: &'a CharMesh,
    char_name_layed_out: &'a LayedOutTextBlock,

    my_client_key: Option<usize>,
    client_char_state: &'a SparseVec<CharState>,
    client_char_name_layed_out: &'a SparseVec<LayedOutTextBlock>,

    day_night_time: f32,
    stars: &'a Mesh,
    white_pixel: &'a GpuImageArray,
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
        let exit_menu_button = menu_button("Back to game")
            .build(ctx.assets);
        let exit_game_button = menu_button("Save and quit to title")
            .build(ctx.assets);
        let options_button = menu_button(&ctx.assets.lang.menu_options)
            .build(ctx.assets);
        let options_fog_button = OptionsOnOffButton::new("Fog");
        let options_day_night_button = OptionsOnOffButton::new("Day Night");
        let options_load_dist_outline_button = OptionsOnOffButton::new("Load Distance Outline");
        let options_chunk_outline_button = OptionsOnOffButton::new("Chunk Outline");
        let options_done_button = menu_button(&ctx.assets.lang.gui_done)
            .build(ctx.assets);
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
}

impl Menu {
    fn gui<'a>(
        &'a mut self,
        resources: &'a mut MenuResources,
        chat: &mut Option<&'a mut GuiChat>,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        match self {
            &mut Menu::EscMenu => GuiEither::A(GuiEither::A(align(0.5,
                logical_size([400.0, 320.0],
                    v_align(0.0,
                        v_stack(0.0, (
                            &mut resources.esc_menu_title_text,
                            logical_height(72.0, gap()),
                            resources.exit_menu_button.gui(on_exit_menu_click(&resources.effect_queue)),
                            logical_height(8.0, gap()),
                            resources.exit_game_button.gui(on_exit_game_click),
                            logical_height(56.0, gap()),
                            resources.options_button.gui(on_options_click(&resources.effect_queue)),
                        ))
                    )
                )
            ))),
            &mut Menu::Inventory => GuiEither::A(GuiEither::B(align(0.5,
                game_gui!(
                    [176, 166],
                    &ctx.assets().gui_inventory,
                    []
                )
            ))),
            &mut Menu::ChatInput {
                ref mut text_block,
                ..
            } => GuiEither::B(GuiEither::A(v_align(1.0,
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
            ))),
            &mut Menu::Settings => GuiEither::B(GuiEither::B(
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
            )),
        }
    }

    fn exitable_via_inventory_button(&self) -> bool {
        match self {
            &Menu::EscMenu => false,
            &Menu::Inventory => true,
            &Menu::ChatInput { .. } => false,
            &Menu::Settings => false,
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
