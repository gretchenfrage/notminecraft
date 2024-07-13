//! Wrapper around client that implements `GuiStateFrame`.

use crate::{
    client::{
        process_msg::{
            process_pre_join_msg,
            process_post_join_msg,
        },
        client_loaded_chunks::ClientLoadedChunks,
        chunk_mesh_mgr::ChunkMeshMgr,
        menu_mgr::MenuGuiClientBorrows,
        menu_esc::EscMenu,
        menu_inventory::InventoryMenu,
        *,
    },
    //sync_state_steve,
    message::*,
    physics::prelude::*,
    gui::prelude::*,
    sync_state_entities,
};
use graphics::prelude::*;
use chunk_data::*;
use std::{
    fmt::{self, Formatter, Debug},
    time::Instant,
};
use vek::*;
use anyhow::*;


/// Wrapper around client that implements `GuiStateFrame`.
pub struct ClientGuiState(pub Client);

/// Gui block that renders the 3D world.
pub struct WorldGuiBlock<'a> {
    pub chunks: &'a ClientLoadedChunks,
    pub chunk_mesh_mgr: &'a ChunkMeshMgr,
    pub pos: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub steve_mesh: &'a Mesh,
    pub chunk_steves: &'a PerChunk<Vec<sync_state_entities::ChunkEntityEntry<sync_state_entities::SteveEntityState, sync_state_entities::SteveEntityClientState>>>,
    pub chunk_pigs: &'a PerChunk<Vec<sync_state_entities::ChunkEntityEntry<sync_state_entities::PigEntityState, sync_state_entities::PigEntityClientState>>>,
}

impl ClientGuiState {
    /// Process an asynchronous client event. The client promptly exits on error.
    pub fn process_event(&mut self, event: ClientEvent) -> Result<()> {
        trace!(?event, "client event");
        match event {
            // ignore this (unsure if possible)
            ClientEvent::AbortInit => (),
            // network event
            ClientEvent::Network(event) => match event {
                NetworkEvent::Received(msg) => match msg {
                    DownMsg::AcceptLogIn => bail!("server protocol violation"),
                    DownMsg::PreJoin(msg) => process_pre_join_msg(&mut self.0.pre_join, msg)?,
                    DownMsg::ShouldJoinGame => bail!("server protocol violation"),
                    DownMsg::FinalizeJoinGame(_) => bail!("server protocol violation"),
                    DownMsg::PostJoin(msg) => process_post_join_msg(&mut self.0, msg)?,
                }
                NetworkEvent::Closed(msg) => bail!("server connection closed: {:?}", msg),
            },
            // chunk meshed for the first time
            ClientEvent::ChunkMeshed { cc, ci, chunk_mesh } => {
                self.0.pre_join.connection.send(UpMsg::PreJoin(PreJoinUpMsg::AcceptMoreChunks(1)));
                self.0.pre_join.chunk_mesh_mgr
                    .on_chunk_meshed(
                        cc,
                        ci,
                        chunk_mesh,
                        &self.0.pre_join.chunks,
                        &self.0.pre_join.tile_blocks,
                    );
            }
        }
        Ok(())
    }

    /// Get as a gui block.
    pub fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        layer((
            WorldGuiBlock {
                chunks: &self.0.pre_join.chunks,
                chunk_mesh_mgr: &self.0.pre_join.chunk_mesh_mgr,
                pos: self.0.pos,
                yaw: self.0.yaw,
                pitch: self.0.pitch,
                steve_mesh: &self.0.steve_mesh,
                chunk_steves: &self.0.pre_join.chunk_steves,
                chunk_pigs: &self.0.pre_join.chunk_pigs,
            },
            self.0.menu_mgr.gui(ctx, MenuGuiClientBorrows {
                connection: &self.0.pre_join.connection,
                inventory_slots: &self.0.inventory_slots,
                item_mesh: &self.0.pre_join.item_mesh,
            }),
        ))
    }
}

impl GuiStateFrame for ClientGuiState {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext<'_>, elapsed: f32) {
        // do a client game logic tick basically
        trace!("client tick");
        
        // super basic movement logic
        if !self.0.menu_mgr.is_open_menu() {
            let mut movement = Vec3::new(0.0, 0.0, 0.0);
            if ctx.global().pressed_keys.contains(&KeyCode::KeyW.into()) {
                movement.z += 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::KeyS.into()) {
                movement.z -= 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::KeyD.into()) {
                movement.x += 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::KeyA.into()) {
                movement.x -= 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::Space.into()) {
                movement.y += 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::ShiftLeft.into()) {
                movement.y -= 1.0;
            }
            let rot = Quaternion::rotation_y(self.0.yaw) * Quaternion::rotation_x(self.0.pitch);
            self.0.pos += rot * movement * elapsed * 10.0;

            let mut lookment = Vec2::new(0.0, 0.0);
            if ctx.global().pressed_keys.contains(&KeyCode::ArrowRight.into()) {
                lookment.x += 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::ArrowLeft.into()) {
                lookment.x -= 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::ArrowUp.into()) {
                lookment.y += 1.0;
            }
            if ctx.global().pressed_keys.contains(&KeyCode::ArrowDown.into()) {
                lookment.y -= 1.0;
            }
            lookment *= elapsed * f32::to_radians(45.0);
            self.0.yaw += lookment.x;
            self.0.pitch -= lookment.y;
        }

        // fully synchronize chunk meshes so they're ready to render
        self.0.pre_join.chunk_mesh_mgr.flush_dirty(
            &self.0.pre_join.chunks,
            &self.0.pre_join.tile_blocks,
        );
        self.0.pre_join.chunk_mesh_mgr.synchronize(
            &self.0.pre_join.chunks,
            &self.0.pre_join.tile_blocks,
        );
    }

    fn on_key_press(
        &mut self,
        ctx: &GuiWindowContext,
        key: PhysicalKey,
        typing: Option<TypingInput>,
    ) {
        trace!(?key, "key press");
        if self.0.menu_mgr.is_open_menu() {
            // have menu handle
            self.0.menu_mgr.on_key_press(ctx, key, typing);
            return;
        } else if key == KeyCode::KeyP || key == KeyCode::KeyL {
            let getter = self.0.pre_join.chunks.getter();
            let rot = Quaternion::rotation_y(self.0.yaw) * Quaternion::rotation_x(self.0.pitch);
            let looking_at = compute_looking_at(
                self.0.pos,
                rot * Vec3::new(0.0, 0.0, 1.0),
                50.0,
                &getter,
                &self.0.pre_join.tile_blocks,
                &self.0.pre_join.game,
            );
            if let Some(looking_at) = looking_at {
                let (gtc, bid_meta) =
                    if key == KeyCode::KeyP {
                        (
                            looking_at.tile.gtc() +
                                looking_at.face.map(|face| face.to_vec()).unwrap_or(0.into()),
                            ErasedBidMeta::new(self.0.pre_join.game.content.stone.bid_stone, ()),
                        )
                    } else {
                        (
                            looking_at.tile.gtc(),
                            ErasedBidMeta::new(AIR, ())
                        )
                    };
                self.0.pre_join.connection.send(UpMsg::PlayerMsg(PlayerMsg::SetTileBlock(
                    PlayerMsgSetTileBlock { gtc, bid_meta }
                )));
            }
        } else if key == KeyCode::Escape {
            self.0.menu_mgr.set_menu(EscMenu::new(ctx.global()));
        } else if key == KeyCode::KeyE {
            self.0.menu_mgr.set_menu(InventoryMenu::new(ctx.global()));
        } else if key == KeyCode::KeyT {
            let now = Instant::now();
            self.0.pre_join.connection.send(UpMsg::PlayerMsg(PlayerMsg::ClockDebug(
                self.0.pre_join.connection.rel_time(now)
            )));
        }
    }

    fn poll_user_events(
        &mut self,
        ctx: &GuiWindowContext,
        stop_at: Instant,
        notify: &GuiUserEventNotify,
    ) {
        // process events
        let mut flushed = Instant::now();
        while let Some(event) = self.0.pre_join.client_recv.poll_gui(notify) {
            // process event
            let result = self.process_event(event);
            if let Err(e) = result {
                error!(%e, "error processing event (client closing)");
                ctx.global().pop_state_frame();
                return;
            }

            // time stuff
            let now = Instant::now();
            if now > stop_at {
                // flushing would be redundant because we're about to do a tick
                return;
            } else if now - flushed > ctx.global().frame_duration_target / 10 {
                // flush occasionally to keep data in transit
                trace!("flushing chunk mesh (period elapsed)");
                self.0.pre_join.chunk_mesh_mgr.flush_dirty(
                    &self.0.pre_join.chunks,
                    &self.0.pre_join.tile_blocks,
                );
                flushed = now;
            }
        }

        trace!("flushing chunk mesh (about to block)");
        self.0.pre_join.chunk_mesh_mgr.flush_dirty(
            &self.0.pre_join.chunks,
            &self.0.pre_join.tile_blocks,
        );
    }

    fn process_gui_effects(&mut self, ctx: &GuiWindowContext) {
        self.0.menu_mgr.process_gui_effects(ctx, &self.0.pre_join.connection);
    }
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let vp = ViewProj::perspective(
            self.inner.pos,
            Quaternion::rotation_x(-self.inner.pitch) * Quaternion::rotation_y(-self.inner.yaw),
            f32::to_radians(120.0),
            self.size,
        );
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(vp, Fog::None);
        for (cc, ci, _getter) in self.inner.chunks.iter() {
            let bbox_pos = (cc * CHUNK_EXTENT).map(|n| n as f32);
            let bbox_ext = CHUNK_EXTENT.map(|n| n as f32);

            let mut canvas = canvas.reborrow()
                .translate(bbox_pos);

            if vp.is_volume_visible(bbox_pos, bbox_ext.into()) {
                if let Some(mesh) = self.inner.chunk_mesh_mgr.chunk_mesh(cc, ci) {
                    canvas.reborrow()
                        .draw_mesh(mesh, &ctx.assets().blocks);
                }
            }
            
            for steve in self.inner.chunk_steves.get(cc, ci) {
                canvas.reborrow()
                    .translate(steve.entity.rel_pos)
                    .color([0.8, 0.8, 0.8, 1.0])
                    .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
            }

            for pig in self.inner.chunk_pigs.get(cc, ci) {
                canvas.reborrow()
                    .translate(pig.entity.rel_pos)
                    .scale([1.0, 0.5, 1.0])
                    .color(pig.entity.state.color)
                    .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
            }
        }
    }
}

impl Debug for ClientGuiState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("ClientGuiState(..)")
    }
}

impl<'a> Debug for WorldGuiBlock<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("WorldGuiBlock { .. }")
    }
}
