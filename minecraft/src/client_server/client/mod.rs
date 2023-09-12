
pub mod connection;
mod apply_edit;
mod tile_meshing;
mod prediction;

use self::{
    connection::Connection,
    tile_meshing::mesh_tile,
    prediction::PredictionManager,
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
};
use chunk_data::*;
use mesh_data::*;
use graphics::{
    prelude::*,
    frame_content::{
        DrawObj2,
        DrawInvert,
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
    mem::take,
};
use slab::Slab;
use anyhow::{Result, ensure, bail};
use vek::*;


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

    chunks: LoadedChunks,
    ci_reverse_lookup: SparseVec<Vec3<i64>>,

    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<ChunkMesh>,
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
        };

        connection.send(UpMessage::LogIn(up::LogIn {
            username: username.clone(),
            char_state,
        }));

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

        ctx.capture_mouse();


        Client {
            connection,

            char_mesh,
            char_name_layed_out,


            char_state,
            noclip: false,
            vel: 0.0.into(),
            time_since_ground: f32::INFINITY,
            time_since_jumped: f32::INFINITY,
            
            char_state_last_sent: char_state,
            char_state_last_sent_time: Instant::now(),

            bob_animation: 0.0,
            third_person: false,

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
            menu_resources: MenuResources::new(ctx.assets),

            chat: GuiChat::new(),
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
            DownMessage::AcceptLogin(msg) => self.on_network_message_accept_login(msg),
            DownMessage::RejectLogin(msg) => self.on_network_message_reject_login(msg)?,
            DownMessage::AddChunk(msg) => self.on_network_message_add_chunk(msg)?,
            DownMessage::RemoveChunk(msg) => self.on_network_message_remove_chunk(msg)?,
            DownMessage::AddClient(msg) => self.on_network_message_add_client(msg, ctx)?,
            DownMessage::RemoveClient(msg) => self.on_network_message_remove_client(msg),
            DownMessage::ThisIsYou(msg) => self.on_network_message_this_is_you(msg),
            DownMessage::ApplyEdit(msg) => self.on_network_message_apply_edit(msg),
            DownMessage::Ack(msg) => self.on_network_message_ack(msg),
            DownMessage::ChatLine(msg) => self.on_network_message_chat_line(msg, ctx),
            DownMessage::SetCharState(msg) => self.on_network_message_set_char_state(msg),
        }
        Ok(())
    }

    fn on_network_message_accept_login(&mut self, msg: down::AcceptLogin) {
        let down::AcceptLogin {} = msg;
        info!("yippeee! initialized");
    }
    
    fn on_network_message_reject_login(&mut self, msg: down::RejectLogin) -> Result<()> {
        let down::RejectLogin { message } = msg;
        bail!("server rejected log in: {}", message);
    }
    
    fn on_network_message_add_chunk(&mut self, msg: down::AddChunk) -> Result<()> {
        let down::AddChunk { cc, ci, chunk_tile_blocks } = msg;
        debug!(?cc, ?ci, "client adding chunk");

        // insert into data structures
        ensure!(
            self.chunks.add(cc) == ci,
            "AddChunk message ci did not correspond to slab behavior",
        );
        self.ci_reverse_lookup.set(ci, cc);

        self.tile_blocks.add(cc, ci, chunk_tile_blocks);
        self.tile_meshes.add(cc, ci, ChunkMesh::new());
        self.block_updates.add_chunk(cc, ci);

        self.prediction.add_chunk(cc, ci);

        // enqueue block updates
        let getter = self.chunks.getter();
        for lti in 0..=MAX_LTI {
            let gtc = cc_ltc_to_gtc(cc, lti_to_ltc(lti));
            self.block_updates.enqueue(gtc, &getter);
        }

        for face in FACES {
            let ranges: Vec3<Range<i64>> = face
                .to_signs()
                .zip(CHUNK_EXTENT)
                .map(|(sign, extent)| match sign {
                    Sign::Neg => -1..0,
                    Sign::Zero => 0..extent,
                    Sign::Pos => extent..extent + 1,
                });

            for x in ranges.x {
                for y in ranges.y.clone() {
                    for z in ranges.z.clone() {
                        let gtc = cc * CHUNK_EXTENT + Vec3 { x, y, z };
                        self.block_updates.enqueue(gtc, &getter);
                    }
                }
            }
        }

        Ok(())
    }

    fn on_network_message_remove_chunk(&mut self, msg: down::RemoveChunk) -> Result<()> {
        let down::RemoveChunk { cc, ci } = msg;

        ensure!(
            self.chunks.getter().get(cc) == Some(ci),
            "RemoveChunk message cc and ci did not match",
        );

        self.chunks.remove(cc);
        self.ci_reverse_lookup.remove(ci);
        self.tile_blocks.remove(cc, ci);
        self.tile_meshes.remove(cc, ci);
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
    
    fn on_network_message_remove_client(&mut self, msg: down::RemoveClient) {
        let down::RemoveClient { client_key } = msg;
        debug!(?client_key, "client removed");
        self.clients.remove(client_key);
        self.client_username.remove(client_key);
        self.client_char_state.remove(client_key);
        self.client_char_name_layed_out.remove(client_key);
    }
    
    fn on_network_message_this_is_you(&mut self, msg: down::ThisIsYou) {
        let down::ThisIsYou { client_key } = msg;
        debug!(?client_key, "this is you!");
        self.my_client_key = Some(client_key);
    }

    fn on_network_message_apply_edit(&mut self, msg: down::ApplyEdit) {
        self.prediction.process_apply_edit_msg(
            msg,
            &self.chunks,
            &self.ci_reverse_lookup,
            &mut self.tile_blocks,
            &mut self.block_updates,
        )
    }
    
    fn on_network_message_ack(&mut self, msg: down::Ack) {
        let down::Ack { last_processed } = msg;
        self.prediction.process_ack(
            last_processed,
            &self.chunks,
            &self.ci_reverse_lookup,
            &mut self.tile_blocks,
            &mut self.block_updates,
        );
    }
    
    fn on_network_message_chat_line(&mut self, msg: down::ChatLine, ctx: &GuiGlobalContext) {
        let down::ChatLine { line } = msg;
        self.chat.add_line(line, ctx);
    }

    fn on_network_message_set_char_state(&mut self, msg: down::SetCharState) {
        let down::SetCharState { client_key, char_state } = msg;
        let () = self.clients[client_key];
        self.client_char_state[client_key] = char_state;
    }

    fn on_ground(&self) -> bool {
        self.time_since_ground < GROUND_DETECTION_PERIOD
        && self.time_since_jumped > GROUND_DETECTION_PERIOD
    }
}


impl GuiStateFrame for Client {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
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

        // deal with messages from the server
        loop {
            match self.connection.poll() {
                Ok(Some(msg)) => {
                    if let Err(e) = self.on_network_message(msg, ctx.global()) {
                        error!(%e, "error processing message from server");
                        ctx.global().pop_state_frame();
                        return;
                    }
                },
                Ok(None) => break,
                Err(e) => {
                    error!(%e, "client connection error");
                    ctx.global().pop_state_frame();
                    return;
                }
            }
        }

        // do block updates
        let mut mesh_buf = MeshData::new();
        let getter = self.chunks.getter();
        while let Some(tile) = self.block_updates.pop() {
            // re-mesh
            mesh_buf.clear();
            mesh_tile(
                &mut mesh_buf,
                tile,
                &getter,
                &self.tile_blocks,
                ctx.game(),
            );
            let ltc_f = lti_to_ltc(tile.lti).map(|n| n as f32);
            for vertex in &mut mesh_buf.vertices {
                vertex.pos += ltc_f;
            }
            tile.set(&mut self.tile_meshes, &mesh_buf);
        }

        const WALK_SPEED: f32 = 4.0;
        const WALK_ACCEL: f32 = 50.0;
        const WALK_DECEL: f32 = 30.0;
        const NOCLIP_SPEED: f32 = 7.0;

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
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::Space) && self.on_ground()
            {
                self.vel.y += 9.2;
                self.time_since_jumped = 0.0;
            }
        } else {
            // noclip reset physics variables
            self.vel = 0.0.into();
            self.time_since_jumped = f32::INFINITY;
            self.time_since_ground = f32::INFINITY;

            // noclip movement
            let mut noclip_move = Vec3::new(walking_xz.x, 0.0, walking_xz.y);

            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::Space) {
                noclip_move.y += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::LShift) {
                noclip_move.y -= 1.0;
            }

            self.char_state.pos += noclip_move * NOCLIP_SPEED * elapsed;
        }

        const GRAVITY_ACCEL: f32 = 32.0;
        const FALL_SPEED_DECAY: f32 = 0.98;

        // gravity
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
            if let Some((tile, bid, placing)) = match button {
                MouseButton::Left => Some((looking_at.tile, AIR.bid, false)),
                MouseButton::Right => {
                    let gtc = looking_at.tile.gtc() + looking_at.face
                        .map(|face| face.to_vec())
                        .unwrap_or(0.into());
                    getter.gtc_get(gtc).map(|tile| (
                        tile,
                        ctx.global().game.content_stone.bid_stone.bid,
                        true,
                    ))
                }
                _ => None
            } {
                if placing {
                    const EPSILON: f32 = 0.0001;
                    let (old_bid, old_meta) = tile
                        .get(&mut self.tile_blocks)
                        .replace(BlockId::new(bid), ());
                    let placing_blocked = WorldPhysicsGeometry {
                        getter: &getter,
                        tile_blocks: &self.tile_blocks,
                        game: ctx.game(),
                    }.box_intersects(AaBox {
                        pos: self.char_state.pos - Vec3::from(PLAYER_BOX_POS_ADJUST),
                        ext: PLAYER_BOX_EXT.into(),
                    }.expand(EPSILON));
                    tile
                        .get(&mut self.tile_blocks)
                        .erased_set(old_bid, old_meta);
                    if placing_blocked {
                        return;
                    }
                }

                self.connection.send(up::SetTileBlock {
                    gtc: tile.gtc(),
                    bid,
                });
                self.prediction.make_prediction(
                    edit::SetTileBlock {
                        lti: tile.lti,
                        bid,
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
            if key == VirtualKeyCode::Escape {
                ctx.global().uncapture_mouse();
                self.menu_stack.push(Menu::EscMenu);
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
            }
        } else {
            if key == VirtualKeyCode::Escape
                || (
                    key == VirtualKeyCode::E
                    && self.menu_stack.iter().rev().next().unwrap()
                        .exitable_via_inventory_button()
                )
            {
                self.menu_stack.pop();
                if self.menu_stack.is_empty() {
                    ctx.global().capture_mouse();
                }
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

    bob_animation: f32,
    third_person: bool,

    chunks: &'a LoadedChunks,
    tile_blocks: &'a PerChunk<ChunkBlocks>,
    tile_meshes: &'a mut PerChunk<ChunkMesh>,

    char_mesh: &'a CharMesh,
    char_name_layed_out: &'a LayedOutTextBlock,

    my_client_key: Option<usize>,
    client_char_state: &'a SparseVec<CharState>,
    client_char_name_layed_out: &'a SparseVec<LayedOutTextBlock>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let SimpleGuiBlock { inner, size, scale: _ } = self;

        // apply any pending chunk tile mesh patches
        for (cc, ci) in inner.chunks.iter() {
            inner.tile_meshes.get_mut(cc, ci).patch(&*ctx.global.renderer.borrow());
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

        // sky
        canvas.reborrow()
            .color(ctx.assets().sky_day)
            .draw_solid(size);

        // begin 3D perspective
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
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(view_proj);

        // chunks
        for (cc, ci) in inner.chunks.iter() {
            // frustum culling
            let pos = (cc * CHUNK_EXTENT).map(|n| n as f32);
            if !view_proj.is_volume_visible(pos, CHUNK_EXTENT.map(|n| n as f32).into()) {
                continue;
            }

            // blocks
            canvas.reborrow()
                .translate(pos)
                .draw_mesh(
                    (&*inner.tile_meshes).get(cc, ci).mesh(),
                    &ctx.assets().blocks,
                );
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
    }
}


// ==== menu stuff ====

#[derive(Debug)]
struct MenuResources {
    esc_menu_title_text: GuiTextBlock<true>,
    exit_menu_button: MenuButton,
    exit_game_button: MenuButton,
    options_button: MenuButton,
    
    effect_queue: MenuEffectQueue,
}

impl MenuResources {
    fn new(assets: &Assets) -> Self {
        let esc_menu_title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Game menu",
            font: assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Bottom,
        });
        let exit_menu_button = menu_button("Back to game")
            .build(assets);
        let exit_game_button = menu_button("Save and quit to title")
            .build(assets);
        let options_button = menu_button(&assets.lang.menu_options)
            .build(assets);
        MenuResources {
            esc_menu_title_text,
            exit_menu_button,
            exit_game_button,
            options_button,

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
    }
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
            } => GuiEither::B(v_align(1.0,
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
            )),
        }
    }

    fn exitable_via_inventory_button(&self) -> bool {
        match self {
            &Menu::EscMenu => false,
            &Menu::Inventory => true,
            &Menu::ChatInput { .. } => false,
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

fn on_options_click<'a>(_effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {

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
