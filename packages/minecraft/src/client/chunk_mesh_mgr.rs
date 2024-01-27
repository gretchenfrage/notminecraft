//! Asynchronously generating graphical meshes for chunks.


use crate::{
    util_abort_handle::AbortGuard,
    game_data::{
        logic::block_mesh_logic::*,
        *,
    },
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
    GpuVecContext,
    AsyncGpuVecContext,
};
use mesh_data::*;
use chunk_data::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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
    // list of chunks in MeshState::Mesghed with dirty tiles
    dirty_meshed_chunks: Vec<(Vec3<i64>, usize)>,
    // for each chunk in MeshState::meshed with dirty tiles, its index in dirty_meshed_chunks
    chunk_dirty_meshed_idx: PerChunk<Option<usize>>,

    // just a reusable buffer
    mesh_buf: MeshBuf,
}

/// A chunk's mesh and additional state for changing per-tile submeshes.
#[derive(Default)]
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
enum PatchJob {
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

    }

    /// Call upon a chunk being added to the world.
    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize, chunk_tile_blocks: &ChunkBlocks) {
        // install into world
        let aborted_1 = AbortGuard::new();
        let aborted_2 = aborted_1.new_handle();
        self.chunk_mesh_state.add(cc, ci, MeshState::Meshing(aborted_1));
        self.chunk_tile_dirty.add(cc, ci, PerTileBool::new());
        self.chunk_dirty_tiles.add(cc, ci, Vec::new());
        self.chunk_dirty_meshed_idx.add(cc, ci, None);
        
        // submit thread pool request to mesh for first time
        todo!("submit job to mesh chunk");
        // TODO: here, what we want... is...
        //       construct the chunk mesh. for all ltcs on the border of other chunks, mesh them,
        //       putting them in the mesh differ and giving them a submesh key. then, clone this
        //       chunk's chunk tile blocks, and send the job to the thread pool, which will put it
        //       in the fake world, mesh the rest of the tiles, and then upload the whole combined
        //       patch to the GPU, and then send it back to us. when we get it back, that stays
        //       normal logic--if it's good it's good, and if not then it must be like, the dirty
        //       stuff must be re-meshed and patched again.
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
                self.dirty_meshed_chunks[dirty] = (cc2, ci2);
                *self.chunk_dirty_meshed_idx.get_mut(cc2, ci2) = dirty_meshed_idx;
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
        *self.chunk_mesh_state.get_mut(cc, ci) = MeshState::Meshed(cc, ci);

        // if it's dirty, just patch it again right away
        if !self.chunk_dirty_tiles.get(cc, ci).is_empty() {
            self.patch_chunk(cc, ci, chunks, tile_blocks);
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
            if chunk_dirtied {
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
    /// all chunks in the meshed state containing dirty tiles patches them, causing all their tiles
    /// to become clean but also transitioning them into the patching state.
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
        for (cc, ci) in self.dirty_meshed_chunks.drain(..) {
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
            let msg = self.patching_thread_recv_res.recv().unwrap();
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
        let mesh_state = self.chunk_mesh_state.get_mut(cc, ci), MeshState::Patching(aborted_1));
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
                chunk_mesh.differ.remove_submesh(submesh_key);
            }
            if !self.mesh_buf.is_empty() {
                let submesh_key = chunk_mesh.differ.add_submesh(&self.mesh_buf);
                chunk_mesh.tile_submesh_key.set_some(submesh_key);
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
        &self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_tile_blocks: &ChunkBlocks,
        aborted: AbortHandle,
    ) {
        // TODO it would be possible to change logic in ways that avoid this clone
        let chunk_tile_blocks = self.init_mesh_job_ctx.game.clone_chunk_blocks(&chunk_tile_blocks);
        let ctx = Arc::clone(&self.init_mesh_job_ctx);
        self.thread_pool.submit(WorkPriority::Client, aborted, move |aborted| {
            let chunk_mesh = mesh_chunk(cc, chunk_tile_blocks, &ctx);
            let event = ClientEvent::ChunkMeshed { cc, ci, chunk_mesh };
            ctx.client_send.send(event, EventPriority::Other, Some(aborted), None);
        });
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

// mesh a single chunk in isolation
fn mesh_chunk(
    cc: Vec3<i64>,
    chunk_tile_blocks: ChunkBlocks,
    ctx: &Arc<InitMeshJobCtx>,
) -> ChunkMesh {
    // build a fake world
    let mut chunks = LoadedChunks::new();
    let mut tile_blocks = PerChunk::new();
    let ci = chunks.add(cc);
    tile_blocks.add(cc, ci, chunk_tile_blocks);

    // construct the empty chunk mesh
    let mut chunk_mesh = ChunkMesh::default()

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
        mesh_tile(&mut mesh_buf, TileKey { cc, ci, lti }, &ctx.game, &getter, &tile_blocks);
        
        // add the tile mesh to the chunk mesh
        if !mesh_buf.is_empty() {
            mesh_buf.translate(ltc.map(|n| n as f32));
            let key = chunk_mesh.differ.add_submesh(&mesh_buf);
            chunk_mesh.submesh_keys.set_some(lti, key.try_into().unwrap());
            mesh_buf.clear();
        }
    }

    // upload to GPU, done
    chunk_mesh.mesh.patch(&ctx.gpu_vec_ctx);
    chunk_mesh
}

/*
/// State for a chunk in terms of it being meshed. After a chunk is added to the client, the
/// uploading of the chunk's mesh to the GPU is done asynchronously in the background.
pub enum ChunkMeshState {
    /// Chunk is still being meshed.
    Meshing(ChunkMeshing),
    /// Chunk has been meshed.
    Meshed(ChunkMeshed),
}

/// Variant of `ChunkMeshState` for when chunk is still being meshed.
pub struct ChunkMeshing {
    /// Abort guard for the request to mesh the chunk.
    pub aborted: AbortGuard,
    /// Whether each tile potentially had its mesh changed since the request to mesh the chunk was
    /// submitted.
    pub tile_dirty: PerTileBool,
    /// List of tiles set to true in tile_dirty.
    pub dirty_tiles: Vec<u16>,
}

/// Variant of `ChunkMeshState` for when chunk has been meshed.
#[derive(Debug)]
pub struct ChunkMeshed {
    /// The GPU mesh
    pub mesh: Mesh,
    /// The mesh differ, which organizes the internal structure of mesh in a way that allows'
    /// variable-length submeshes to be efficiently added and removed, and tracks the buffered
    /// GPU mesh writes involved in such.
    pub differ: MeshDiffer,
    /// For every tile with a non-empty mesh, its submesh key in the mesh differ.
    pub submesh_keys: PerTileOption<u16>,
    /// Whether there are mesh edits buffered in the mesh differ.
    pub dirty: bool,
}

impl ChunkMeshState {
    /// Recalculate the mesh for a given tile, or mark it as needing recalculation.
    pub fn remesh_tile(&mut self)
}

impl From<ChunkMeshing> for ChunkMeshState {
    fn from(meshing: ChunkMeshing) -> Self {
        ChunkMeshState::Meshing(meshing)
    }
}

impl From<ChunkMeshed> for ChunkMeshState {
    fn from(meshed: ChunkMeshed) -> Self {
        ChunkMeshState::Meshed(meshed)
    }
}

impl ChunkMeshed {
    /// Upload all buffered mesh changes from the CPU to the GPU.
    pub fn patch<G: GpuVecContext>(&mut self, gpu_vec_ctx: &G) {
        let (vertices_diff, indices_diff) = self.differ.diff();
        vertices_diff.patch(&mut self.mesh.vertices, gpu_vec_ctx);
        indices_diff.patch(&mut self.mesh.indices, gpu_vec_ctx);
        self.dirty = false;
    }
}

/// Utility for triggering jobs to mesh chunks.
pub struct ChunkMesher {
    thread_pool: ThreadPool,
    job_ctx: Arc<JobCtx>,
}

struct JobCtx {
    game: Arc<GameData>,
    client_send: ClientSender,
    gpu_vec_ctx: AsyncGpuVecContext,
}

impl ChunkMesher {
    /// Construct.
    pub fn new(
        game: Arc<GameData>,
        client_send: ClientSender,
        thread_pool: ThreadPool,
        gpu_vec_ctx: AsyncGpuVecContext,
    ) -> Self {
        ChunkMesher {
            thread_pool,
            job_ctx: Arc::new(JobCtx {
                game,
                client_send,
                gpu_vec_ctx,
            }),
        }
    }

    /// Submit a job to the thread pool to mesh this chunk and then send it back to the client as a
    /// `ChunkMeshed` asynchronous event.
    pub fn trigger_mesh(
        &self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_tile_blocks: ChunkBlocks,
    ) -> ChunkMeshing {
        let ctx = Arc::clone(&self.job_ctx);
        let aborted = AbortGuard::new();
        self.thread_pool.submit(WorkPriority::Client, aborted.new_handle(), move |aborted| {
            let meshed = mesh_chunk(cc, chunk_tile_blocks, &ctx.game, &ctx.gpu_vec_ctx);
            let event = ClientEvent::ChunkMeshed { cc, ci, meshed };
            ctx.client_send.send(event, EventPriority::Other, Some(aborted), None);
        });
        ChunkMeshing { aborted, tile_dirty: PerTileBool::new(), dirty_tiles: Vec::new() }
    }
}

// mesh a single chunk in isolation
fn mesh_chunk(
    cc: Vec3<i64>,
    chunk_tile_blocks: ChunkBlocks,
    game: &Arc<GameData>,
    gpu_vec_ctx: &AsyncGpuVecContext,
) -> ChunkMeshed {
    // build a fake world
    let mut chunks = LoadedChunks::new();
    let mut tile_blocks = PerChunk::new();
    let ci = chunks.add(cc);
    tile_blocks.add(cc, ci, chunk_tile_blocks);

    // construct the empty ChunkMeshed
    let mut meshed = ChunkMeshed {
        mesh: Mesh::new(),
        differ: MeshDiffer::new(),
        submesh_keys: PerTileOption::new(),
        dirty: false,
    };

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
        mesh_tile(&mut mesh_buf, TileKey { cc, ci, lti }, game, &getter, &tile_blocks);
        
        // add the tile mesh to the chunk mesh
        if !mesh_buf.is_empty() {
            mesh_buf.translate(ltc.map(|n| n as f32));
            let key = meshed.differ.add_submesh(&mesh_buf);
            meshed.submesh_keys.set_some(lti, key.try_into().unwrap());
            meshed.dirty = true;
            mesh_buf.clear();
        }
    }

    // upload to GPU, done
    meshed.patch(gpu_vec_ctx);
    meshed
}

// mesh a single tile in isolation
fn mesh_tile(
    mesh_buf: &mut MeshData,
    tile: TileKey,
    game: &Arc<GameData>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
) {
    let bid1 = tile.get(tile_blocks).get();
    match &game.blocks_mesh_logic[bid1] {
        &BlockMeshLogic::NoMesh => (),
        &BlockMeshLogic::FullCube(BlockMeshLogicFullCube { tex_indices, .. }) => {
            // mesh each face
            let gtc1 = tile.gtc();
            for face in FACES {
                // skip if obscured
                let gtc2 = gtc1 + face.to_vec();
                if getter
                    .gtc_get(gtc2)
                    .map(|tile2| {
                        let bid2 = tile2.get(tile_blocks).get();
                        game.blocks_mesh_logic[bid2].obscures(-face)
                    })
                    .unwrap_or(true)
                {
                    continue;
                }

                // begin meshing
                let (pos_start, pos_exts) = face.quad_start_extents();
                let mut vert_rgbs = [Rgb::white(); 4];

                // ambient occlusion
                for (corner, ext_coefs) in [
                    (0, [Pole::Neg, Pole::Neg]),
                    (1, [Pole::Pos, Pole::Neg]),
                    (2, [Pole::Pos, Pole::Pos]),
                    (3, [Pole::Neg, Pole::Pos]),
                ] {
                    // calculate occlusion level from 0 through 3
                    let sides = [0, 1].map(|i| ext_coefs[i] * pos_exts[i]);
                    let [a, b] = sides.map(|side| getter
                        .gtc_get(gtc2 + side.to_vec())
                        .map(|tile3| {
                            let bid3 = tile3.get(tile_blocks).get();
                            game.blocks_mesh_logic[bid3].obscures(-side) as i32
                        })
                        .unwrap_or(0));
                    let c = getter
                        .gtc_get(gtc2 + sides[0].to_vec() + sides[1].to_vec())
                        .map(|tile3| {
                            let bid3 = tile3.get(tile_blocks).get();
                            sides.into_iter()
                                .all(|side| game.blocks_mesh_logic[bid3].obscures(-side)) as i32
                        })
                        .unwrap_or(0);
                    let ab = a * b;
                    let occlusion = 3 * ab + (1 - ab) * (ab * c);

                    // light accordingly
                    vert_rgbs[corner] *= 1.0 - occlusion as f32 / 3.0 * 0.25;
                }

                // axis lighting
                let axis_lighting = match face {
                    Face::PosY => 0,
                    Face::PosX | Face::NegX => 1,
                    Face::PosZ | Face::NegZ => 2,
                    Face::NegY => 3,
                };
                for vert_rgb in &mut vert_rgbs {
                    *vert_rgb *= 1.0 - axis_lighting as f32 * 0.07;
                }

                // add quad to mesh
                let pos_start = pos_start.to_poles().map(|pole| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => 1.0,
                });
                let [
                    pos_ext_1,
                    pos_ext_2,
                ] = pos_exts.map(|pos_ext| pos_ext.to_vec().map(|n| n as f32));
                let vert_colors = vert_rgbs.map(|rgb| Rgba::from((rgb, 1.0)));
                let quad = Quad {
                    pos_start,
                    pos_ext_1: pos_ext_1.into(),
                    pos_ext_2: pos_ext_2.into(),
                    tex_start: 0.0.into(),
                    tex_extent: 1.0.into(),
                    vert_colors,
                    tex_index: tex_indices[face],
                };
                let flip = vert_rgbs[0].sum() + vert_rgbs[2].sum()
                    < vert_rgbs[1].sum() + vert_rgbs[3].sum();
                let indices =
                    if flip { FLIPPED_QUAD_INDICES }
                    else { QUAD_INDICES };
                mesh_buf.extend(quad.to_vertices(), indices);
            }
        }
    }
}
*/