
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
    util::{
        hex_color::hex_color,
        secs_rem::secs_rem,
    },
};
use chunk_data::*;
use mesh_data::MeshData;
use graphics::prelude::*;
use std::{
    ops::Range,
    f32::consts::PI,
    cell::RefCell,
    collections::VecDeque,
    time::Duration,
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

            chat: GuiChat::new(),
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        let mut chat = Some(&mut self.chat);
        let menu_gui = self.menu_stack.iter_mut().rev().next()
            .map(|open_menu| layer((
                if open_menu.has_darkened_background() {
                    Some(solid([0.0, 0.0, 0.0, 1.0 - 0x2a as f32 / 0x97 as f32]))
                } else { None },
                open_menu.gui(&mut self.menu_resources, &mut chat, ctx),
            )));
        layer((
            WorldGuiBlock {
                pos: self.pos,
                pitch: self.pitch,
                yaw: self.yaw,

                chunks: &self.chunks,
                tile_meshes: &mut self.tile_meshes,
            },
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
            } else if key == VirtualKeyCode::T {
                ctx.global().uncapture_mouse();
                let blinker = secs_rem(ctx.global().time_since_epoch, 2.0 / 3.0) < 1.0 / 3.0;
                self.menu_stack.push(Menu::ChatInput {
                    t_preventer: true,
                    text: String::new(),
                    text_block: make_chat_input_text_block("", blinker, ctx.global()),
                    blinker,
                });
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
                if let Some(&Menu::ChatInput {
                    ref text,
                    ..
                }) = self.menu_stack.iter().rev().next() {
                    self.chat.add_line(format!("<me> {}", text), ctx.global());
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
                        //min_height(20.0, 1.0,
                        v_pad(2.0, 2.0,
                            h_margin(8.0, 8.0,
                                &mut chat_line.text_block
                            )
                        )
                        //),
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
