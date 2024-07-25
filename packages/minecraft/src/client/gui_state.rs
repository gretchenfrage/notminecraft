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
    server::tick_mgr::TICK,
    sync_state_entities::{steve_physics_continuous, steve_physics_discrete},
    message::*,
    physics::prelude::*,
    gui::prelude::*,
    sync_state_entities,
};
use graphics::prelude::*;
use chunk_data::*;
use std::{
    fmt::{self, Formatter, Debug},
    time::{Instant, Duration},
    cmp::min,
    mem::take,
};
use vek::*;
use anyhow::*;


pub const MAX_CATCHUP: Duration = Duration::from_millis(30);


/// Wrapper around client that implements `GuiStateFrame`.
pub struct ClientGuiState(pub Client);

/// Gui block that renders the 3D world.
pub struct WorldGuiBlock<'a> {
    pub chunks: &'a ClientLoadedChunks,
    pub chunk_mesh_mgr: &'a ChunkMeshMgr,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,
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
                    DownMsg::AcceptLogIn(_) => bail!("server protocol violation"),
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
                tile_blocks: &self.0.pre_join.tile_blocks,
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

    fn update(&mut self, ctx: &GuiWindowContext<'_>, elapsed: f32, now: Instant) {
        // do a client game logic tick basically
        trace!("client tick");

        // client-side prediction
        /*
        let tick_catchup = self.0.pre_join.just_finished_tick.take()
            .map(|tick_start| {
                let delta = now.checked_duration_since(tick_start).unwrap_or(Duration::ZERO);
                delta.as_secs_f32()// + elapsed
                //min(delta, MAX_CATCHUP).as_secs_f32()
            });*/
        //let just_finished_tick = self.0.pre_join.just_finished_tick.take();

            /*
        let tdi = self.0.pre_join.next_tick_instant + Duration::from_secs_f32(tick_catchup.unwrap_or(elapsed));
        let tick_dst =
            if now > tdi {
                now.duration_since(tdi).as_secs_f32()
            } else {
                -tdi.duration_since(now).as_secs_f32()
            } - 0.000001;*/

        let tick_just_finished = take(&mut self.0.pre_join.tick_just_finished);

        for (cc, ci, getter) in self.0.pre_join.chunks.iter() {
            let chunk_newly_added = take(self.0.pre_join.chunk_newly_added.get_mut(cc, ci));

            for steve in self.0.pre_join.chunk_steves.get_mut(cc, ci) {
                if tick_just_finished || chunk_newly_added {
                    steve.extra.predicted_cc = cc;
                    steve.extra.predicted_rel_pos = steve.entity.rel_pos;
                    steve.extra.predicted_vel = steve.entity.state.vel;
                }

                let (mut caught_up_to, mut next_catch_up_tick) =
                    if chunk_newly_added {
                        (
                            self.0.pre_join.next_tick_instant - TICK,
                            self.0.pre_join.next_tick_instant,
                        )
                    } else {
                        (
                            self.0.pre_join.caught_up_to,
                            self.0.pre_join.next_catch_up_tick,
                        )
                    };

                //debug!("doing steve loop");
                while caught_up_to < now {
                    if next_catch_up_tick < now {
                        steve_physics_continuous(
                            next_catch_up_tick.duration_since(caught_up_to).as_secs_f32(),
                            cc,
                            &mut steve.extra.predicted_rel_pos,
                            &mut steve.extra.predicted_vel,
                            &getter,
                            &self.0.pre_join.tile_blocks,
                            &self.0.pre_join.game,
                            None,
                        );
                        steve_physics_discrete(
                            cc,
                            &mut steve.extra.predicted_rel_pos,
                            &mut steve.extra.predicted_vel,
                            &getter,
                            &self.0.pre_join.tile_blocks,
                            &self.0.pre_join.game,
                            None,
                        );
                        caught_up_to = next_catch_up_tick;
                        next_catch_up_tick += TICK;
                    } else {
                        steve_physics_continuous(
                            now.duration_since(caught_up_to).as_secs_f32(),
                            cc,
                            &mut steve.extra.predicted_rel_pos,
                            &mut steve.extra.predicted_vel,
                            &getter,
                            &self.0.pre_join.tile_blocks,
                            &self.0.pre_join.game,
                            None,
                        );
                        caught_up_to = now;
                    }
                }

                if let &mut Some((ref mut smoothed_rel_pos, ref mut smooth_vel)) = &mut steve.extra.smoothed_rel_pos_vel {
                    let mut new_smoothed_rel_pos = *smoothed_rel_pos;
                    
                    /*steve_physics_continuous(
                        elapsed,
                        cc,
                        &mut new_smoothed_rel_pos,
                        smooth_vel,
                        &getter,
                        &self.0.pre_join.tile_blocks,
                        &self.0.pre_join.game,
                        None,
                    );*/
                    new_smoothed_rel_pos += *smooth_vel * elapsed;

                    let old_to_new = new_smoothed_rel_pos - *smoothed_rel_pos;
                    let old_to_pred = steve.extra.predicted_rel_pos - *smoothed_rel_pos;

                    if old_to_new != Vec3::from(0.0) && old_to_pred != Vec3::from(0.0) {
                        let dot = old_to_new.dot(old_to_pred);
                        if dot / old_to_pred.magnitude() > old_to_new.magnitude() {
                            *smoothed_rel_pos = new_smoothed_rel_pos;
                        } else {
                            *smoothed_rel_pos += old_to_new * (dot / old_to_new.magnitude());
                        }
                        /*let dot = old_to_new.normalized().dot(old_to_pred.normalized());
                        if dot >= 1.0 {
                            *smoothed_rel_pos = new_smoothed_rel_pos;
                        } else if dot > 0.0 {
                            *smoothed_rel_pos += old_to_new * dot;
                        }*/
                    }

                    *smooth_vel = steve.extra.predicted_vel;

                    /*
                    let old_smoothed_to_pred = steve.extra.predicted_rel_pos - old_smoothed_rel_pos;
                    if !old_smoothed_to_pred.zero() {
                        let delta_smoothed_pos = new_smoothed_rel_pos - old_smoothed_rel_pos;
                        let dot = delta_smoothed_pos.dot(old_smoothed_to_pred);
                        let ideal_dist_along_dsp = dot / old_smoothed_to_pred.magnitude();

                        if ideal_dist_along_dsp > 0.0 {
                            if ideal_dist_along_dsp >= delta_smoothed_pos.magnitude() {
                                steve.extra.smoothed_rel_pos = Some(new_smoothed_rel_pos);
                            } else {
                                steve.extra.smoothed_rel_pos = Some(old_smoothed_rel_pos + ideal_dist_along_dsp)
                            }
                        }
                    }*/


                    /*
                    let a = (new_smoothed_rel_pos - old_smoothed_rel_pos).dot(steve.extra.predicted_rel_pos - old_smoothed_rel_pos);
                    if a > 0.0 {
                        if a * a >= (new_smoothed_rel_pos - old_smoothed_rel_pos).magnitude_squared() * (steve.extra.predicted_rel_pos - old_smoothed_rel_pos).magnitude_squared() {
                            steve.extra.smoothed_rel_pos = Some(new_smoothed_rel_pos);
                        } else {
                            steve.extra.smoothed_rel_pos = Some(old_smoothed_rel_pos + (new_smoothed_rel_pos - old_smoothed_rel_pos) * a / ((steve.extra.predicted_rel_pos - old_smoothed_rel_pos).magnitude() / (new_smoothed_rel_pos - old_smoothed_rel_pos).magnitude()));
                        }
                    }*/
                    


                    /*if smooth_vel.y > -10.0 && !(
                        (steve.extra.smoothed_rel_pos.unwrap() - steve.extra.predicted_rel_pos).magnitude()
                        <=
                        (old_smoothed_rel_pos - steve.extra.predicted_rel_pos).magnitude()
                    ) {
                        dbg!(steve.extra.predicted_rel_pos);
                        dbg!(old_smoothed_rel_pos);
                        dbg!(new_smoothed_rel_pos);
                        dbg!(steve.extra.smoothed_rel_pos);
                        dbg!(new_smoothed_rel_pos - old_smoothed_rel_pos);
                        dbg!(steve.extra.predicted_rel_pos - old_smoothed_rel_pos);
                        dbg!((new_smoothed_rel_pos - old_smoothed_rel_pos).magnitude_squared());
                        dbg!((steve.extra.predicted_rel_pos - old_smoothed_rel_pos).magnitude_squared());
                        dbg!(a);
                        dbg!(a * a);
                        dbg!(a * a >= (new_smoothed_rel_pos - old_smoothed_rel_pos).magnitude_squared() * (steve.extra.predicted_rel_pos - old_smoothed_rel_pos).magnitude_squared());
                        dbg!((old_smoothed_rel_pos - steve.extra.predicted_rel_pos).magnitude());
                        dbg!((steve.extra.smoothed_rel_pos.unwrap() - steve.extra.predicted_rel_pos).magnitude());
                        panic!();
                    }*/
                } else {
                    steve.extra.smoothed_rel_pos_vel = Some((steve.extra.predicted_rel_pos, steve.extra.predicted_vel));
                }

                /*
                if tick_catchup.is_some() {
                    steve.extra.predicted_cc = cc;
                    steve.extra.predicted_rel_pos = steve.entity.rel_pos;
                    steve.extra.predicted_vel = steve.entity.state.vel;
                }

                let (mut caught_up_to, mut next_catch_up_tick) =
                    if just_finished_tick.is_some() {
                        (
                            self.0.pre_join.next_tick_instant - TICK,
                            self.0.pre_join.next_tick_instant,
                        )
                    } else {
                        (
                            now - Duration::from_secs_f32(elapsed),

                    };


                while caught_up_to < now {
                    if now.duration_since(caught_up_to) < TICK {

                    }
                }*/
                /*
                if let Some(just_finished_tick) = just_finished_tick {

                } else {
                    steve_physics_continuous(
                        elapsed
                        cc,
                        &mut steve.extra.predicted_rel_pos,
                        &mut steve.extra.predicted_vel,
                        &getter,
                        &self.0.pre_join.tile_blocks,
                        &self.0.pre_join.game,
                        None,
                    );
                }*/
                // TODO: more complete prediction limiting
                /*do_steve_physics(
                    tick_catchup.unwrap_or(elapsed), 
                    tick_dst,
                    cc,
                    &mut steve.extra.predicted_rel_pos,
                    &mut steve.extra.predicted_vel,
                    &getter,
                    &self.0.pre_join.tile_blocks,
                    &self.0.pre_join.game,
                    None,
                );*/

                use std::io::Write as _;
                std::writeln!(
                    &mut steve.extra.file,
                    "{}, {}, {}, {}, {}, {}",
                    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_micros(),
                    (cc.y * CHUNK_EXTENT.y) as f32 + steve.extra.predicted_rel_pos.y,
                    (cc.y * CHUNK_EXTENT.y) as f32 + steve.entity.rel_pos.y,
                    (cc.y * CHUNK_EXTENT.y) as f32 + steve.extra.smoothed_rel_pos_vel.unwrap().0.y,
                    steve.extra.predicted_vel.y,
                    steve.entity.state.vel.y,
                ).unwrap();
            }

            /*for pig in self.0.pre_join.chunk_pigs.get_mut(cc, ci) {
                if tick_catchup.is_some() {
                    pig.extra.predicted_cc = cc;
                    pig.extra.predicted_rel_pos = pig.entity.rel_pos;
                    pig.extra.predicted_vel = pig.entity.state.vel;
                }
                do_steve_physics(
                    tick_catchup.unwrap_or(elapsed),
                    tick_dst,
                    cc,
                    &mut pig.extra.predicted_rel_pos,
                    &mut pig.extra.predicted_vel,
                    &getter,
                    &self.0.pre_join.tile_blocks,
                    &self.0.pre_join.game,
                    None,
                );
            }*/
        }

        //debug!("doing post-steve loop");
        while self.0.pre_join.caught_up_to < now {
            if self.0.pre_join.next_catch_up_tick < now {
                self.0.pre_join.caught_up_to = self.0.pre_join.next_catch_up_tick;
                self.0.pre_join.next_catch_up_tick += TICK;
            } else {
                self.0.pre_join.caught_up_to = now;
            }
        }

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
        } else if key == KeyCode::KeyP || key == KeyCode::KeyL || key == KeyCode::KeyO {
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
                if key == KeyCode::KeyP || key == KeyCode::KeyL {
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
                } else {
                    self.0.pre_join.connection.send(UpMsg::PlayerMsg(PlayerMsg::SpawnSteve(
                        looking_at.pos
                        //looking_at.tile.gtc().map(|n| n as f32)
                    )));
                }
            }
        } else if key == KeyCode::KeyK {
            self.0.pre_join.connection.send(UpMsg::PlayerMsg(PlayerMsg::ClearSteves));
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
        let rot = Quaternion::rotation_x(-self.inner.pitch) * Quaternion::rotation_y(-self.inner.yaw);
        let dir = rot.conjugate() * Vec3::new(0.0, 0.0, 1.0);
        let vp = ViewProj::perspective(
            self.inner.pos,
            rot,
            f32::to_radians(120.0),
            self.size,
        );
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(vp, Fog::None);
        for (cc, ci, _getter) in self.inner.chunks.iter() {
            let bbox_pos = (cc * CHUNK_EXTENT).map(|n| n as f32);
            let bbox_ext = CHUNK_EXTENT.map(|n| n as f32);

            if vp.is_volume_visible(bbox_pos, bbox_ext.into()) {
                let mut canvas = canvas.reborrow()
                    .translate(bbox_pos);

                if let Some(mesh) = self.inner.chunk_mesh_mgr.chunk_mesh(cc, ci) {
                    canvas.reborrow()
                        .draw_mesh(mesh, &ctx.assets().blocks);
                }
            }
            
            for steve in self.inner.chunk_steves.get(cc, ci) {
                canvas.reborrow()
                    .translate((steve.extra.predicted_cc * CHUNK_EXTENT).map(|n| n as f32))
                    .translate(steve.extra.smoothed_rel_pos_vel.unwrap().0)
                    .color([0.8, 0.8, 0.8, 1.0])
                    .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
            }

            for pig in self.inner.chunk_pigs.get(cc, ci) {
                canvas.reborrow()
                    .translate((pig.extra.predicted_cc * CHUNK_EXTENT).map(|n| n as f32))
                    .translate(pig.extra.predicted_rel_pos)
                    .scale([1.0, 0.5, 1.0])
                    .color(pig.entity.state.color)
                    .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
            }
        }
        let getter = self.inner.chunks.getter();
        if let Some(looking_at) = compute_looking_at(
            self.inner.pos,
            dir,
            50.0,
            &getter,
            self.inner.tile_blocks,
            ctx.game(),
        ) {
            const GAP: f32 = 0.002;

            {
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

            canvas.reborrow()
                .translate(looking_at.pos)
                .color(Rgba::red())
                .scale(0.1)
                .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
            canvas.reborrow()
                .translate(looking_at.tile.gtc().map(|n| n as f32))
                .color(Rgba::green())
                .scale(0.1)
                .draw_mesh(self.inner.steve_mesh, &ctx.assets().blocks);
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
