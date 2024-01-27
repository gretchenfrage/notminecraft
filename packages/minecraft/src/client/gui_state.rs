//! Wrapper around client that implements `GuiStateFrame`.

use crate::{
    client::{
        pre_join::process_pre_join_msg,
        client_loaded_chunks::ClientLoadedChunks,
        chunk_mesh_mgr::ChunkMeshMgr,
        *,
    },
    message::*,
    physics::prelude::*,
    gui::prelude::*,
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
                    DownMsg::Ack { .. } => (), // TODO
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
        _ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        layer((
            WorldGuiBlock {
                chunks: &self.0.pre_join.chunks,
                chunk_mesh_mgr: &self.0.pre_join.chunk_mesh_mgr,
                pos: self.0.pos,
                yaw: self.0.yaw,
                pitch: self.0.pitch,
            },
        ))
    }
}

impl GuiStateFrame for ClientGuiState {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext<'_>, elapsed: f32) {
        // do a client game logic tick basically
        trace!("client tick");
        
        // super basic movement logic
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
        _: &GuiWindowContext,
        key: PhysicalKey,
        _: Option<TypingInput>,
    ) {
        trace!(?key, "key press");
        if key == KeyCode::KeyP {
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
                let offset = looking_at.face.map(|face| face.to_vec()).unwrap_or(0.into());
                let gtc = looking_at.tile.gtc() + offset;
                self.0.pre_join.connection.send(UpMsg::PlayerMsg(PlayerMsg::SetTileBlock(
                    PlayerMsgSetTileBlock {
                        gtc,
                        bid_meta: ErasedBidMeta::new(
                            self.0.pre_join.game.content.stone.bid_stone,
                            (),
                        ),
                    }
                )));
            }
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

            if !vp.is_volume_visible(bbox_pos, bbox_ext.into()) {
                continue;
            }

            if let Some(mesh) = self.inner.chunk_mesh_mgr.chunk_mesh(cc, ci) {
                canvas.reborrow()
                    .translate(bbox_pos)
                    .draw_mesh(mesh, &ctx.assets().blocks);
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
