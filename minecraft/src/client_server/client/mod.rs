
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
    physics::looking_at::compute_looking_at,
};
use chunk_data::*;
use mesh_data::MeshData;
use graphics::prelude::*;
use std::{
    ops::Range,
    f32::consts::PI,
    cell::RefCell,
    collections::VecDeque,
};
use anyhow::{Result, ensure, bail};
use vek::*;


/// GUI state frame for multiplayer game client.
#[derive(Debug)]
pub struct Client {
    connection: Connection,

    pos: Vec3<f32>,
    pitch: f32,
    yaw: f32,

    chunks: LoadedChunks,
    ci_reverse_lookup: SparseVec<Vec3<i64>>,

    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<ChunkMesh>,
    block_updates: BlockUpdateQueue,

    prediction: PredictionManager,

    menu_stack: Vec<Menu>,
    menu_resources: MenuResources,
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
    pub fn new(address: &str, ctx: &GuiGlobalContext) -> Self {
        let mut connection = Connection::connect(address, ctx.tokio, ctx.game);
        connection.send(UpMessage::LogIn(up::LogIn {
            username: get_username(),
        }));
        Client {
            connection,

            pos: [0.0, 80.0, 0.0].into(),
            pitch: f32::to_radians(-30.0),
            yaw: f32::to_radians(0.0),

            chunks: LoadedChunks::new(),
            ci_reverse_lookup: SparseVec::new(),

            tile_blocks: PerChunk::new(),
            tile_meshes: PerChunk::new(),
            block_updates: BlockUpdateQueue::new(),

            prediction: PredictionManager::new(),

            menu_stack: Vec::new(),
            menu_resources: MenuResources::new(ctx.assets),
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        layer((
            WorldGuiBlock {
                pos: self.pos,
                pitch: self.pitch,
                yaw: self.yaw,

                chunks: &self.chunks,
                tile_meshes: &mut self.tile_meshes,
            },
            mouse_capturer(),
            self.menu_stack.iter_mut().rev().next()
                .map(|open_menu| layer((
                    solid([0.0, 0.0, 0.0, 1.0 - 0x2a as f32 / 0x97 as f32]),
                    open_menu.gui(&mut self.menu_resources, ctx),
                ))),
        ))
    }

    fn on_network_message(&mut self, msg: DownMessage) -> Result<()> {
        match msg {
            DownMessage::Initialized(down::Initialized {}) => {
                info!("yippeee! initialized");
            }
            DownMessage::RejectLogIn(down::RejectLogIn {
                message,
            }) => {
                bail!("server rejected log in: {}", message)
            }
            DownMessage::LoadChunk(down::LoadChunk {
                cc,
                ci,
                chunk_tile_blocks,
            }) => {
                // insert into data structures
                ensure!(
                    self.chunks.add(cc) == ci,
                    "DownMessage::load_chunk ci did not correspond to slab behavior"
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
            }
            DownMessage::ApplyEdit(msg) => self.prediction.process_apply_edit_msg(
                msg,
                &self.chunks,
                &self.ci_reverse_lookup,
                &mut self.tile_blocks,
                &mut self.block_updates,
            ),
            DownMessage::Ack(down::Ack { last_processed }) => self.prediction.process_ack(
                last_processed,
                &self.chunks,
                &self.ci_reverse_lookup,
                &mut self.tile_blocks,
                &mut self.block_updates,
            ),
        }
        Ok(())
    }
}


impl GuiStateFrame for Client {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
        // menu stuff
        self.menu_resources.process_effect_queue(&mut self.menu_stack);

        // deal with messages from the server
        loop {
            match self.connection.poll() {
                Ok(Some(msg)) => {
                    if let Err(e) = self.on_network_message(msg) {
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

        // movement
        if ctx.global().focus_level == FocusLevel::MouseCaptured {
            let mut movement = Vec3::from(0.0);
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::W) {
                movement.z += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::S) {
                movement.z -= 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::D) {
                movement.x += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::A) {
                movement.x -= 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::Space) {
                movement.y += 1.0;
            }
            if ctx.global().pressed_keys_semantic.contains(&VirtualKeyCode::LShift) {
                movement.y -= 1.0;
            }

            let xz = Vec2::new(movement.x, movement.z).rotated_z(self.yaw);
            movement.x = xz.x;
            movement.z = xz.y;

            movement *= 5.0;
            movement *= elapsed;
            self.pos += movement;
        }
    }

    fn on_captured_mouse_move(&mut self, _: &GuiWindowContext, amount: Vec2<f32>) {
        let sensitivity = 1.0 / 1600.0;
        
        self.pitch = (self.pitch - amount.y * sensitivity).clamp(-PI / 2.0, PI / 2.0);
        self.yaw = (self.yaw - amount.x * sensitivity) % (PI * 2.0);
    }

    fn on_captured_mouse_click(&mut self, ctx: &GuiWindowContext, button: MouseButton) {
        let getter = self.chunks.getter();
        if let Some(looking_at) = compute_looking_at(
            // position
            self.pos,
            // direction
            Quaternion::rotation_y(-self.yaw)
                * Quaternion::rotation_x(-self.pitch)
                * Vec3::new(0.0, 0.0, 1.0),
            // reach
            50.0,
            // geometry
            &getter,
            &self.tile_blocks,
            ctx.game(),
        ) {
            match button {
                MouseButton::Left => {
                    self.connection.send(up::SetTileBlock {
                        gtc: looking_at.tile.gtc(),
                        bid: AIR.bid,
                    });
                    self.prediction.make_prediction(
                        edit::SetTileBlock {
                            lti: looking_at.tile.lti,
                            bid: AIR.bid,
                        }.into(),
                        looking_at.tile.cc,
                        looking_at.tile.ci,
                        &getter,
                        &self.connection,
                        &mut self.tile_blocks,
                        &mut self.block_updates,
                    );
                }
                _ => (),
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
            }
        } else {
            if key == VirtualKeyCode::Escape {
                self.menu_stack.pop();
                if self.menu_stack.is_empty() {
                    ctx.global().capture_mouse();
                }
            } else {
                self.menu_stack.iter_mut().rev().next().unwrap().on_key_press_semantic(
                    &mut self.menu_resources,
                    ctx,
                    key,
                );
            }
        }
    }
}


/// GUI block that draws the 3D game world from the player's perspective.
#[derive(Debug)]
struct WorldGuiBlock<'a> {
    pos: Vec3<f32>,
    pitch: f32,
    yaw: f32,

    chunks: &'a LoadedChunks,
    tile_meshes: &'a mut PerChunk<ChunkMesh>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let SimpleGuiBlock { inner, size, scale: _ } = self;

        // apply any pending chunk tile mesh patches
        for (cc, ci) in inner.chunks.iter() {
            inner.tile_meshes.get_mut(cc, ci).patch(&*ctx.global.renderer.borrow());
        }

        // sky
        canvas.reborrow()
            .color(ctx.assets().sky_day)
            .draw_solid(size);

        // begin 3D perspective
        let view_proj = ViewProj::perspective(
            // position
            inner.pos,
            // direction
            Quaternion::rotation_x(inner.pitch) * Quaternion::rotation_y(inner.yaw),
            // field of view
            f32::to_radians(120.0),
            // aspect ratio
            aspect_ratio(size),
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
}

impl Menu {
    fn on_key_press_semantic(
        &mut self,
        resources: &mut MenuResources,
        _: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {
        if key == VirtualKeyCode::E && match self {
            &mut Menu::EscMenu => false,
            &mut Menu::Inventory => true,
        } {
            resources.effect_queue.get_mut().push_back(MenuEffect::PopMenu);
        }
    }

    fn gui<'a>(
        &'a mut self,
        resources: &'a mut MenuResources,
        _: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        match self {
            &mut Menu::EscMenu => GuiEither::A(align(0.5,
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
            )),
            &mut Menu::Inventory => GuiEither::B(solid([1.0, 0.0, 0.0, 0.5])),
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
