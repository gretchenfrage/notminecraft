
mod connection;
mod tile_meshing;

use self::{
    connection::Connection,
    tile_meshing::mesh_tile,
};
use super::message::*;
use crate::{
    block_update_queue::BlockUpdateQueue,
    chunk_mesh::ChunkMesh,
    game_data::GameData,
    gui::{
        *,
        blocks::{
            *,
            simple_gui_block::*,
        },
    },
    util::sparse_vec::SparseVec,
    physics::looking_at::compute_looking_at,
};
use chunk_data::*;
use mesh_data::MeshData;
use graphics::{
    frame_content::*,
    view_proj::ViewProj,
};
use std::{
    sync::Arc,
    ops::Range,
    f32::consts::PI,
};
use tokio::{
    runtime::Handle,
};
use anyhow::{Result, ensure};
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
}

impl Client {
    pub fn new(
        game: &Arc<GameData>,
        rt: &Handle,
    ) -> Self {
        Client {
            connection: Connection::connect("ws://127.0.0.1:35565", rt, game),

            pos: [0.0, 80.0, 0.0].into(),
            pitch: f32::to_radians(-30.0),
            yaw: f32::to_radians(0.0),

            chunks: LoadedChunks::new(),
            ci_reverse_lookup: SparseVec::new(),
            tile_blocks: PerChunk::new(),
            tile_meshes: PerChunk::new(),
            block_updates: BlockUpdateQueue::new(),
        }
    }

    fn gui<'a>(
        &'a mut self,
        _: &'a GuiWindowContext,
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
        ))
    }

    fn on_network_message(&mut self, msg: DownMessage) -> Result<()> {
        match msg {
            DownMessage::LoadChunk(DownMessageLoadChunk {
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

                // enqueue block updates
                let getter = self.chunks.getter();
                for lti in 0..=MAX_LTI {
                    let gtc = cc_ltc_to_gtc(cc, lti_to_ltc(lti));
                    self.block_updates.enqueue(gtc, &getter);
                }

                for fec in FACES_EDGES_CORNERS {
                    let ranges: Vec3<Range<i64>> = fec
                        .to_signs()
                        .zip(CHUNK_EXTENT)
                        .map(|(sign, extent)| match sign {
                            Sign::Neg => 0..1,
                            Sign::Zero => 0..extent,
                            Sign::Pos => extent - 1..extent,
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
            DownMessage::SetTileBlock(DownMessageSetTileBlock {
                ci,
                lti,
                bid
            }) => {
                // modify local world
                self.tile_blocks.get_mut_checkless(ci).raw_set(lti, bid, ());

                // enqueue block updates
                let cc = self.ci_reverse_lookup[ci];
                let getter = self.chunks.getter_pre_cached(cc, ci);
                let gtc = cc_ltc_to_gtc(cc, lti_to_ltc(lti));

                self.block_updates.enqueue(gtc, &getter);
                for face in FACES {
                    self.block_updates.enqueue(gtc + face.to_vec(), &getter);
                }

            }
        }
        Ok(())
    }
}


impl GuiStateFrame for Client {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
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
                * Vec3::new(0.0, 0.0, 1.0), // what? wtf does this part do?
            // reach
            50.0,
            // geometry
            &getter,
            &self.tile_blocks,
            ctx.game(),
        ) {
            match button {
                MouseButton::Left => {
                    self.connection.send(UpMessage::SetTileBlock(UpMessageSetTileBlock {
                        gtc: looking_at.tile.gtc(),
                        bid: AIR.bid,
                    }));
                }
                _ => (),
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
        let mut canvas = canvas.reborrow()
            .scale(size)
            .begin_3d(ViewProj::perspective(
                // position
                inner.pos,
                // rotation
                Quaternion::rotation_x(inner.pitch) * Quaternion::rotation_y(inner.yaw),
                // field of view
                f32::to_radians(120.0),
                // aspect ratio
                size.w / size.h,
            ));

        // chunk tile meshes
        for (cc, ci) in inner.chunks.iter() {
            canvas.reborrow()
                .translate((cc * CHUNK_EXTENT).map(|n| n as f32))
                .draw_mesh(
                    (&*inner.tile_meshes).get(cc, ci).mesh(),
                    &ctx.assets().blocks,
                );
        }
    }
}
