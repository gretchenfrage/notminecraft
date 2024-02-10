//! Asynchronously generating graphical meshes for chunks.

use crate::{
    util_abort_handle::*,
    game_data::*,
    client::{
        channel::*,
        mesh_tile::mesh_tile,
        client_loaded_chunks::ClientLoadedChunks,
        *,
    },
    thread_pool::*,
};
use graphics::{
    frame_content::Mesh,
    AsyncGpuVecContext,
};
use mesh_data::*;
use chunk_data::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::spawn,
    mem::replace,
    ops::Range,
};
use vek::*;
use crossbeam::channel::{
    unbounded,
    Sender,
    Receiver,
};


/// Manages client-side chunk meshes.
pub struct ChunkMeshMgr {
    // handle to the thread pool, for initial meshing jobs
    thread_pool: ThreadPool,
    // shared state for initial meshing jobs
    init_mesh_job_ctx: Arc<InitMeshJobCtx>,
    // for sending requests to patching thread
    patch_thread_send_req: Sender<PatchJob>,
    // for receiving responses from patching thread
    patch_thread_recv_res: Receiver<PatchJob>,
    // number of non-aborted jobs currently pending through patching thread
    patching_count: u64,
    // for killing the patching thread
    patch_thread_kill: Arc<AtomicBool>,
    // mesh state for each chunk
    chunk_mesh_state: PerChunk<MeshState>,
    // for each tile, whether it's marked as dirty
    chunk_tile_dirty: PerChunk<PerTileBool>,
    // for each chunk, list of dirty tiles in it
    chunk_dirty_tiles: PerChunk<Vec<u16>>,
    // list of chunks in MeshState::Meshed with dirty tiles
    dirty_meshed_chunks: Vec<(Vec3<i64>, usize)>,
    // for each chunk in MeshState::Meshed with dirty tiles, its index in dirty_meshed_chunks
    chunk_dirty_meshed_idx: PerChunk<Option<usize>>,

    // just a reusable buffer
    mesh_buf: MeshData,
}

/// A chunk's mesh and additional state for changing per-tile submeshes.
#[derive(Default, Debug)]
pub struct ChunkMesh {
    /// The GPU mesh. May lag behind changes applied to differ.
    pub mesh: Mesh,
    /// Tracks per-tile submeshes. Tile submeshes are set in differ, then differ compiles all
    /// changes into patch which is uploaded to GPU and applied to mesh.
    pub differ: MeshDiffer,
    /// For each tile with a non-empty submesh, its submesh key in differ.
    pub tile_submesh_key: PerTileOption<u16>,
}

// state a chunk's mesh can be in
enum MeshState {
    // chunk is still being meshed for the first time in the thread pool
    Meshing(AbortGuard),
    // chunk's mesh exists and is owned here
    Meshed(ChunkMesh),
    // chunk's mesh is currently being patched in the patching thread
    Patching(AbortGuard),
}

// shared state for initial meshing jobs in the thread pool
struct InitMeshJobCtx {
    game: Arc<GameData>,
    client_send: ClientSender,
    gpu_vec_ctx: AsyncGpuVecContext,
}

// message sent through the patching thread.
//
// the patching thread receives this, does the patch, and then sends it back. 
struct PatchJob {
    // tracking whether this request was aborted. if aborted, sending back is not necessary.
    aborted: Option<AbortHandle>,
    cc: Vec3<i64>,
    ci: usize,
    // chunk mesh to patch.
    chunk_mesh: ChunkMesh,
}

impl ChunkMeshMgr {
    /// Construct.
    pub fn new(
        game: Arc<GameData>,
        client_send: ClientSender,
        thread_pool: ThreadPool,
        gpu_vec_ctx: AsyncGpuVecContext,
    ) -> Self {
        let (patch_thread_send_req, patch_thread_recv_req) = unbounded();
        let (patch_thread_send_res, patch_thread_recv_res) = unbounded();
        let patch_thread_kill_1 = Arc::new(AtomicBool::new(false));
        let patch_thread_kill_2 = Arc::clone(&patch_thread_kill_1);
        let gpu_vec_ctx_2 = gpu_vec_ctx.clone();
        spawn(move || patch_thread_body(
            patch_thread_recv_req,
            patch_thread_send_res,
            patch_thread_kill_1,
            gpu_vec_ctx_2,
        ));
        ChunkMeshMgr {
            thread_pool,
            init_mesh_job_ctx: Arc::new(InitMeshJobCtx {
                game,
                client_send,
                gpu_vec_ctx,
            }),
            patch_thread_send_req,
            patch_thread_recv_res,
            patching_count: 0,
            patch_thread_kill: patch_thread_kill_2,
            chunk_mesh_state: Default::default(),
            chunk_tile_dirty: Default::default(),
            chunk_dirty_tiles: Default::default(),
            dirty_meshed_chunks: Default::default(),
            chunk_dirty_meshed_idx: Default::default(),
            mesh_buf: Default::default(),
        }
    }

    /// Call upon a chunk being added to the world. May interally call `self.mark_dirty()`.
    pub fn add_chunk(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
    ) {
        // install into world
        let aborted_1 = AbortGuard::new();
        let aborted_2 = aborted_1.new_handle();
        self.chunk_mesh_state.add(cc, ci, MeshState::Meshing(aborted_1));
        self.chunk_tile_dirty.add(cc, ci, PerTileBool::new());
        self.chunk_dirty_tiles.add(cc, ci, Vec::new());
        self.chunk_dirty_meshed_idx.add(cc, ci, None);
        
        // trigger relevant meshing
        self.trigger_init_mesh(cc, ci, chunks, tile_blocks, aborted_2);
        self.mark_chunk_adj_dirty(cc, ci, chunks);
    }

    /// Call upon the given chunk being removed from the world.
    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        let mesh_state = self.chunk_mesh_state.remove(cc, ci);
        if matches!(mesh_state, MeshState::Patching(_)) {
            self.patching_count -= 1;
        }
        drop(mesh_state);

        self.chunk_tile_dirty.remove(cc, ci);
        self.chunk_dirty_tiles.remove(cc, ci);

        // swap-remove with backlink updating
        let dirty_meshed_idx = self.chunk_dirty_meshed_idx.remove(cc, ci);
        if let Some(dirty_meshed_idx) = dirty_meshed_idx {
            let (cc2, ci2) = self.dirty_meshed_chunks.pop().unwrap();
            if dirty_meshed_idx < self.dirty_meshed_chunks.len() {
                self.dirty_meshed_chunks[dirty_meshed_idx] = (cc2, ci2);
                *self.chunk_dirty_meshed_idx.get_mut(cc2, ci2) = Some(dirty_meshed_idx);
            }
        }
    }

    /// Call upon receiving a chunk meshed event.
    ///
    /// When a chunk is added, it begins in the meshing state. When this is called, it transitions
    /// to the meshed state.
    pub fn on_chunk_meshed(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_mesh: ChunkMesh,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
    ) {
        // install it
        *self.chunk_mesh_state.get_mut(cc, ci) = MeshState::Meshed(chunk_mesh);

        // if it's dirty, just patch it again right away
        if !self.chunk_dirty_tiles.get(cc, ci).is_empty() {
            self.patch_chunk(cc, ci, chunks, tile_blocks);
        }
    }

    /// Call `mark_dirty` on `gtc` and its neighbors if present.
    pub fn mark_adj_dirty(&mut self, getter: &Getter, gtc: Vec3<i64>) {
        for z in gtc.z - 1..=gtc.z + 1 {
            for y in gtc.y - 1..=gtc.y + 1 {
                for x in gtc.x - 1..=gtc.x + 1 {
                    if let Some(tile) = getter.gtc_get(Vec3 { x, y, z }) {
                        self.mark_dirty(tile);
                    }
                }
            }
        }
    }

    /// Call when the given tile's mesh may have changed.
    ///
    /// Marks that tile as dirty. If the chunk is currently in the meshed state, it gets patched
    /// next time flush_dirty is called. If the chunk is currently in some other state, it gets
    /// patched next time it enters the meshed state. When it is patched as such, all its tiles
    /// become clean, but also it enters the patched state and is sent to the patching thread,
    /// until polling the patching thread reveals it to be fully patched and sent back to the main
    /// thread, at which point it transitions back to the meshed state. This may recursively
    /// trigger it to re-enter the patching state as described.
    pub fn mark_dirty(&mut self, tile: TileKey) {
        if !tile.get(&self.chunk_tile_dirty) {
            // mark tile as dirty
            tile.set(&mut self.chunk_tile_dirty, true);
            let dirty_tiles = self.chunk_dirty_tiles.get_mut(tile.cc, tile.ci);
            let chunk_dirtied = dirty_tiles.is_empty();
            dirty_tiles.push(tile.lti);

            // maybe mark chunk as dirty
            if chunk_dirtied
                && matches!(self.chunk_mesh_state.get(tile.cc, tile.ci), &MeshState::Meshed(_))
            {
                let dirty_meshed_idx = self.dirty_meshed_chunks.len();
                self.dirty_meshed_chunks.push((tile.cc, tile.ci));
                *self.chunk_dirty_meshed_idx.get_mut(tile.cc, tile.ci) = Some(dirty_meshed_idx);
            }
        }
    }

    /// Call when it may be the end of a chronologically dense burst of `mark_dirty` calls.
    ///
    /// See mark_dirty for best description. This polls the patch thread, potentially causing some
    /// chunks in the patching state to transition to the meshed state. Then, this loops through
    /// all chunks in the meshed state containing dirty tiles and patches them, causing all their
    /// tiles to become clean but also transitioning them into the patching state.
    pub fn flush_dirty(
        &mut self,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
    ) {
        // poll patch thread
        while let Ok(msg) = self.patch_thread_recv_res.try_recv() {
            let PatchJob { aborted, cc, ci, chunk_mesh } = msg;
            if aborted
                .map(|aborted| !aborted.is_aborted())
                .unwrap_or(true)
            {
                // received non-aborted patched chunk, so internalize it
                *self.chunk_mesh_state.get_mut(cc, ci) = MeshState::Meshed(chunk_mesh);
                self.patching_count -= 1;

                // if it's dirty, just patch it again right away
                if !self.chunk_dirty_tiles.get(cc, ci).is_empty() {
                    self.patch_chunk(cc, ci, chunks, tile_blocks);
                }
            }
        }

        // patch all dirty meshed chunks, and mark them as clean
        while let Some((cc, ci)) = self.dirty_meshed_chunks.pop() {
            self.patch_chunk(cc, ci, chunks, tile_blocks);
            *self.chunk_dirty_meshed_idx.get_mut(cc, ci) = None;
        }
    }

    /// Call (once) before calling `chunk_mesh` (n times).
    ///
    /// This blocks until all jobs going through the patch thread are complete and returned, thus
    /// waiting until all chunks in the patching state transition to the meshed state (including,
    /// possibly, for each such chunk, up to once, upon receiving it, re-transitioning it back into
    /// the patched state and then waiting for it to return again), and causing all (clean) tiles
    /// in such chunks to have their up-to-date submeshes actually reflected in the GPU mesh.
    pub fn synchronize(
        &mut self,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
    ) {
        debug_assert!(
            self.dirty_meshed_chunks.is_empty(),
            "expected usage of synchronize is to call after flush_dirty",
        );

        // block on patching thread until it clears out
        while self.patching_count > 0 {
            let msg = self.patch_thread_recv_res.recv().unwrap();
            let PatchJob { aborted, cc, ci, chunk_mesh } = msg;
            if aborted
                .map(|aborted| !aborted.is_aborted())
                .unwrap_or(true)
            {
                // received non-aborted patched chunk, so internalize it
                *self.chunk_mesh_state.get_mut(cc, ci) = MeshState::Meshed(chunk_mesh);
                self.patching_count -= 1;

                // if it's dirty, just send it right back in to be patched
                if !self.chunk_dirty_tiles.get(cc, ci).is_empty() {
                    self.patch_chunk(cc, ci, chunks, tile_blocks);
                }
            }
        }
    }

    /// Get the renderable mesh for the given chunk if there is one.
    pub fn chunk_mesh(&self, cc: Vec3<i64>, ci: usize) -> Option<&Mesh> {
        debug_assert!(
            self.patching_count == 0,
            "expected usage of chunk_mesh is to call after synchronize",
        );
        debug_assert!(
            self.dirty_meshed_chunks.is_empty(),
            "expected usage of chunk_mesh is to call after synchronize, after flush_dirty",
        );

        match self.chunk_mesh_state.get(cc, ci) {
            &MeshState::Meshed(ref chunk_mesh) => Some(&chunk_mesh.mesh),
            _ => None,
        }
    }

    // internal method to patch a chunk. assumes that the chunk is in the meshed state and has dirty
    // tiles. does not handle removing it from the list of dirty meshed chunks.
    fn patch_chunk(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
    ) {
        // transition state
        let aborted_1 = AbortGuard::new();
        let aborted_2 = aborted_1.new_handle();
        let mesh_state = MeshState::Patching(aborted_1);
        let mesh_state = replace(self.chunk_mesh_state.get_mut(cc, ci), mesh_state);
        let mut chunk_mesh = match mesh_state {
            MeshState::Meshed(chunk_mesh) => chunk_mesh,
            _ => unreachable!(),
        };

        // re-mesh tiles and mark them as clean
        let getter = chunks.getter_pre_cached(cc, ci);
        for lti in self.chunk_dirty_tiles.get_mut(cc, ci).drain(..) {
            self.chunk_tile_dirty.get_mut(cc, ci).set(lti, false);
            debug_assert!(self.mesh_buf.is_empty());
            mesh_tile(
                &mut self.mesh_buf,
                TileKey { cc, ci, lti },
                &self.init_mesh_job_ctx.game,
                &getter,
                tile_blocks,
            );
            self.mesh_buf.translate(lti_to_ltc(lti).map(|n| n as f32));
            if let Some(&submesh_key) = chunk_mesh.tile_submesh_key.get(lti) {
                chunk_mesh.differ.remove_submesh(submesh_key as usize);
            }
            if self.mesh_buf.is_empty() {
                chunk_mesh.tile_submesh_key.set_none(lti);
            } else {
                let submesh_key = chunk_mesh.differ.add_submesh(&self.mesh_buf);
                chunk_mesh.tile_submesh_key.set_some(lti, submesh_key.try_into().unwrap());
                self.mesh_buf.clear();
            }
        }

        // send to patching thread
        let msg = PatchJob { aborted: Some(aborted_2), cc, ci, chunk_mesh };
        let _ = self.patch_thread_send_req.send(msg);
        self.patching_count += 1;
    }

    // internal method to send a job to the thread pool to mesh a new chunk for the first time
    fn trigger_init_mesh(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        chunks: &ClientLoadedChunks,
        tile_blocks: &PerChunk<ChunkBlocks>,
        aborted: AbortHandle,
    ) {
        // mesh all the border tiles while here, without applying the patch
        let mut chunk_mesh = ChunkMesh::default();
        let getter = chunks.getter_pre_cached(cc, ci);
        for ltc in FACES_EDGES_CORNERS.into_iter()
            .flat_map(|fec| permute_ranges(fec
                .to_signs()
                .zip(CHUNK_EXTENT)
                .map(|(sign, ext)| match sign {
                    Sign::Neg => 0..1,
                    Sign::Zero => 1..ext - 1,
                    Sign::Pos => ext - 1..ext,
                })))
        {
            let lti = ltc_to_lti(ltc);
            debug_assert!(self.mesh_buf.is_empty());
            mesh_tile(
                &mut self.mesh_buf,
                TileKey { cc, ci, lti },
                &self.init_mesh_job_ctx.game,
                &getter,
                tile_blocks,
            );
            self.mesh_buf.translate(ltc.map(|n| n as f32));
            if !self.mesh_buf.is_empty() {
                let key = chunk_mesh.differ.add_submesh(&self.mesh_buf);
                chunk_mesh.tile_submesh_key.set_some(lti, key.try_into().unwrap());
                self.mesh_buf.clear();
            }
        }
        // then clone the chunk blocks and send a task to the threadpool to mesh the rest of the
        // tiles then apply the patch
        let chunk_tile_blocks = self.init_mesh_job_ctx.game
            .clone_chunk_blocks(tile_blocks.get(cc, ci));
        let ctx = Arc::clone(&self.init_mesh_job_ctx);
        self.thread_pool.submit(WorkPriority::Client, aborted, move |aborted| {
            // build a fake world
            let mut chunks = LoadedChunks::new();
            let mut tile_blocks = PerChunk::new();
            let ci2 = chunks.add(cc);
            debug_assert_eq!(ci2, 0);
            tile_blocks.add(cc, ci2, chunk_tile_blocks);

            // mesh every tile not on the border
            let mut mesh_buf = MeshData::new();
            let getter = chunks.getter();
            for ltc in (1..CHUNK_EXTENT.z - 1)
                .flat_map(|z| (1..CHUNK_EXTENT.y - 1)
                    .flat_map(move |y| (1..CHUNK_EXTENT.x - 1)
                        .map(move |x| Vec3 { x, y, z })))
            {
                // mesh the tile
                let lti = ltc_to_lti(ltc);
                debug_assert!(mesh_buf.is_empty());
                let tile = TileKey { cc, ci: ci2, lti };
                mesh_tile(&mut mesh_buf, tile, &ctx.game, &getter, &tile_blocks);
                
                // add the tile mesh to the chunk mesh
                if !mesh_buf.is_empty() {
                    mesh_buf.translate(ltc.map(|n| n as f32));
                    let key = chunk_mesh.differ.add_submesh(&mesh_buf);
                    chunk_mesh.tile_submesh_key.set_some(lti, key.try_into().unwrap());
                    mesh_buf.clear();
                }
            }

            // upload to GPU
            let (v_diff, i_diff) = chunk_mesh.differ.diff();
            v_diff.patch(&mut chunk_mesh.mesh.vertices, &ctx.gpu_vec_ctx);
            i_diff.patch(&mut chunk_mesh.mesh.indices, &ctx.gpu_vec_ctx);
            
            // send it back
            let event = ClientEvent::ChunkMeshed { cc, ci, chunk_mesh };
            ctx.client_send.send(event, EventPriority::Other, Some(aborted), None);
        });
    }

    // marks dirty all tiles bordering on the new chunk, not including tiles in that chunk
    fn mark_chunk_adj_dirty(&mut self, cc: Vec3<i64>, ci: usize, chunks: &ClientLoadedChunks) {
        let getter = chunks.getter_pre_cached(cc, ci);
        for face in FACES {
            let cc2 = cc + face.to_vec();
            if let Some(ci2) = getter.get(cc2) {
                for ltc in permute_ranges(face
                    .to_signs()
                    .zip(CHUNK_EXTENT)
                    .map(|(sign, ext)| match sign {
                        Sign::Neg => ext - 1..ext,
                        Sign::Zero => 0..ext,
                        Sign::Pos => 0..1,
                    }))
                {
                    self.mark_dirty(TileKey { cc: cc2, ci: ci2, lti: ltc_to_lti(ltc) });
                }
            }
        }
    }
}

impl Drop for ChunkMeshMgr {
    fn drop(&mut self) {
        self.patch_thread_kill.store(true, Ordering::SeqCst);
    }
}

// body of the patch thread
fn patch_thread_body(
    recv_req: Receiver<PatchJob>,
    send_res: Sender<PatchJob>,
    patch_thread_kill: Arc<AtomicBool>,
    gpu_vec_ctx: AsyncGpuVecContext,
) {
    while let Ok(mut msg) = recv_req.recv() {
        if patch_thread_kill.load(Ordering::SeqCst) {
            return;
        }
        if msg.aborted.as_ref()
            .map(|aborted| !aborted.is_aborted())
            .unwrap_or(true)
        {
            let (v_diff, i_diff) = msg.chunk_mesh.differ.diff();
            v_diff.patch(&mut msg.chunk_mesh.mesh.vertices, &gpu_vec_ctx);
            i_diff.patch(&mut msg.chunk_mesh.mesh.indices, &gpu_vec_ctx);
            let _ = send_res.send(msg);
        }
    }
}

// helper function
fn permute_ranges(ranges: Vec3<Range<i64>>) -> impl Iterator<Item=Vec3<i64>> {
    ranges.z.flat_map(move |z| {
        let x_range = ranges.x.clone();
        ranges.y.clone()
            .flat_map(move |y| x_range.clone()
                .map(move |x| Vec3 { x, y, z }))
    })
}
