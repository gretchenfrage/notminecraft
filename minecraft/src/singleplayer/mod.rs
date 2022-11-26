
mod block_update_queue;
mod chunk_loader;
mod movement;
mod tile_meshing;
mod looking_at;

use self::{
    block_update_queue::BlockUpdateQueue,
    chunk_loader::{
        ChunkLoader,
        ReadyChunk,
    },
    movement::{
        KeyBindings,
        MovementController,
    },
    tile_meshing::mesh_tile,
    looking_at::compute_looking_at,
};
use crate::{
    game_data::GameData,
    chunk_mesh::ChunkMesh,
    gui::{
        blocks::simple_gui_block::{
            SimpleGuiBlock,
            simple_blocks_cursor_impl,
        },
        GuiStateFrame,
        DimParentSets,
        GuiVisitor,
        GuiVisitorTarget,
        GuiWindowContext,
        GuiSpatialContext,
        GuiBlock,
        SizedGuiBlock,
        GuiNode,
        MouseButton,
        VirtualKeyCode,
        impl_visit_nodes,
    },
};
use chunk_data::{
    FACES,
    EDGES,
    FACES_EDGES_CORNERS,
    CHUNK_EXTENT,
    AIR,
    LoadedChunks,
    PerChunk,
    TileKey,
    ChunkBlocks,
    Getter,
    BlockId,
    lti_to_ltc,
    cc_ltc_to_gtc,
};
use mesh_data::{
    MeshData,
    Quad,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        Mesh,
    },
};
use std::{
    ops::Range,
    sync::Arc,
};
use vek::*;


#[derive(Debug)]
pub struct Singleplayer {
    bindings: KeyBindings,
    movement: MovementController,

    chunks: LoadedChunks,
    
    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<ChunkMesh>,
    
    block_updates: BlockUpdateQueue,
    chunk_loader: ChunkLoader,

    _debug_cube_mesh: Mesh,
    //_human_mesh: Mesh,
}

fn insert_chunk(
    chunk: ReadyChunk,
    chunks: &mut LoadedChunks,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    tile_meshes: &mut PerChunk<ChunkMesh>,
    block_updates: &mut BlockUpdateQueue,
) {
    // deconstruct
    let ReadyChunk {
        cc,
        chunk_tile_blocks,
        chunk_tile_meshes,
    } = chunk;

    trace!(?cc, "inserting chunk");

    // insert
    let ci = chunks.add(cc);

    tile_blocks.add(cc, ci, chunk_tile_blocks);
    tile_meshes.add(cc, ci, chunk_tile_meshes);

    block_updates.add_chunk(cc, ci);

    // enqueue updates for all involved tiles
    let getter = chunks.getter();

    for fec in FACES_EDGES_CORNERS {
        let ranges: Vec3<Range<i64>> = fec
            .to_vec()
            .zip(CHUNK_EXTENT)
            .map(|(sign, extent)| match sign {
                -1 => 0..1,
                0 => 0..extent,
                1 => extent - 1..extent,
                _ => unreachable!(),
            });

        for x in ranges.x {
            for y in ranges.y.clone() {
                for z in ranges.z.clone() {
                    let gtc = cc * CHUNK_EXTENT + Vec3 { x, y, z };
                    block_updates.enqueue(gtc, &getter);
                }
            }
        }
    }

    for face in FACES {
        let ranges: Vec3<Range<i64>> = face
            .to_vec()
            .zip(CHUNK_EXTENT)
            .map(|(sign, extent)| match sign {
                -1 => -1..0,
                0 => 0..extent,
                1 => extent..extent + 1,
                _ => unreachable!(),
            });

        for x in ranges.x {
            for y in ranges.y.clone() {
                for z in ranges.z.clone() {
                    let gtc = cc * CHUNK_EXTENT + Vec3 { x, y, z };
                    block_updates.enqueue(gtc, &getter);
                }
            }
        }
    }
}

fn do_block_update(
    tile: TileKey,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    tile_meshes: &mut PerChunk<ChunkMesh>,
    game: &GameData,
    mesh_buf: &mut MeshData,
) {
    // re-mesh
    mesh_buf.clear();

    mesh_tile(
        mesh_buf,
        tile,
        getter,
        tile_blocks,
        game,
    );

    let rel_to = lti_to_ltc(tile.lti).map(|n| n as f32);
    for vertex in &mut mesh_buf.vertices {
        vertex.pos += rel_to;
    }

    tile.set(tile_meshes, mesh_buf);
}

fn put_block<M: 'static>(
    tile: TileKey,
    getter: &Getter,
    bid: BlockId<M>,
    meta: M,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    block_updates: &mut BlockUpdateQueue,
) {
    tile.get(tile_blocks).set(bid, meta);
    let gtc = cc_ltc_to_gtc(tile.cc, lti_to_ltc(tile.lti));
    block_updates.enqueue(gtc, getter);
    for face in FACES {
        block_updates.enqueue(gtc + face.to_vec(), getter);
    }
}

impl Singleplayer {
    pub fn new(game: &Arc<GameData>, renderer: &Renderer) -> Self {
        let chunk_loader = ChunkLoader::new(game, renderer);

        let view_dist = 12;

        let mut to_request = Vec::new();
        for x in -view_dist..view_dist {
            for y in 0..2 {
                for z in -view_dist..view_dist {
                    to_request.push(Vec3 { x, y, z });
                    //chunk_loader.request(Vec3::new(x, y, z));
                }
            }
        }
        to_request.sort_by_key(|cc| cc.x * cc.x + cc.z * cc.z );
        for cc in to_request {
            chunk_loader.request(cc);
        }

        let mut debug_cube_mesh = MeshData::new();
        for (pos_start, pos_ext_1, pos_ext_2) in [
            // front (-z)
            ([0, 0, 0], [0, 1, 0], [1, 0, 0]),
            // back (+z)
            ([1, 0, 1], [0, 1, 0], [-1, 0, 0]),
            // left (-x)
            ([0, 0, 1], [0, 1, 0], [0, 0, -1]),
            // right (+x)
            ([1, 0, 0], [0, 1, 0], [0, 0, 1]),
            // top (+y)
            ([0, 1, 0], [0, 0, 1], [1, 0, 0]),
            // bottom (-y)
            ([0, 0, 1], [0, 0, -1], [1, 0, 0]),   
        ] {
            debug_cube_mesh.add_quad(&Quad {
                pos_start: Vec3::from(pos_start).map(|n: i32| n as f32),
                pos_ext_1: Extent3::from(pos_ext_1).map(|n: i32| n as f32),
                pos_ext_2: Extent3::from(pos_ext_2).map(|n: i32| n as f32),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [Rgba::white(); 4],
                tex_index: 0,
            });
        }
        let debug_cube_mesh = debug_cube_mesh.upload(renderer);

        Singleplayer {
            bindings: KeyBindings::default(),
            movement: MovementController::default(),

            chunks: LoadedChunks::new(),

            tile_blocks: PerChunk::new(),
            tile_meshes: PerChunk::new(),

            block_updates: BlockUpdateQueue::new(),
            chunk_loader,

            _debug_cube_mesh: debug_cube_mesh,
        }
    }

    fn gui<'a>(
        &'a mut self,
        _: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        self
    }
}

impl GuiStateFrame for Singleplayer {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
        // insert chunks that are ready to be loaded
        // this may generate block updates
        /*while*/ if let Some(chunk) = self.chunk_loader.poll_ready() {
            insert_chunk(
                chunk,
                &mut self.chunks,
                &mut self.tile_blocks,
                &mut self.tile_meshes,
                &mut self.block_updates,
            );
        }

        // process block updates
        // this may generate mesh diffs
        let mut mesh_buf = MeshData::new();
        let getter = self.chunks.getter();
        while let Some(tile) = self.block_updates.pop() {
            do_block_update(
                tile,
                &getter,
                &self.tile_blocks,
                &mut self.tile_meshes,
                ctx.game(),
                &mut mesh_buf,
            );
        }

        // do movement stuff
        self.movement.update(ctx.global(), &self.bindings, elapsed);
    }

    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {
        if key == VirtualKeyCode::Escape {
            ctx.global().pop_state_frame();
        }
    }

    fn on_captured_mouse_move(
        &mut self,
        _: &GuiWindowContext,
        amount: Vec2<f32>,
    ) {
        self.movement.on_captured_mouse_move(amount);
    }

    fn on_captured_mouse_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {
        let getter = self.chunks.getter();
        let looking_at = compute_looking_at(
            self.movement.cam_pos,
            self.movement.cam_dir(),
            100.0,
            &getter,
            &self.tile_blocks,
            ctx.game(),
        );
        if let Some(looking_at) = looking_at {
            match button {
                MouseButton::Left => put_block(
                    looking_at.tile,
                    &getter,
                    AIR,
                    (),
                    &mut self.tile_blocks,
                    &mut self.block_updates,
                ),
                MouseButton::Right => {
                    let tile1 = looking_at.tile;
                    let gtc1 = cc_ltc_to_gtc(tile1.cc, lti_to_ltc(tile1.lti));
                    if let Some(tile2) = looking_at
                        .face
                        .and_then(|face| getter.gtc_get(gtc1 + face.to_vec()))
                    {
                        put_block(
                            tile2,
                            &getter,
                            ctx.game().bid_glass,
                            (),
                            &mut self.tile_blocks,
                            &mut self.block_updates,
                        );
                    }
                }
                _ => (),
            }
        }
    }
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut Singleplayer> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>)
    {
        let state = self.inner;

        canvas.reborrow()
            .color(ctx.resources().sky_day)
            .draw_solid(self.size);

        {
            let view_proj = state.movement.view_proj(self.size);
            let mut canvas = canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj);

            // patch all tile meshes
            for (cc, ci) in state.chunks.iter() {
                state
                    .tile_meshes
                    .get_mut(cc, ci)
                    .patch(&*ctx.global.renderer.borrow());
            }

            // render all tile meshes
            for (cc, ci) in state.chunks.iter() {
                let rel_to = (cc * CHUNK_EXTENT).map(|n| n as f32);
                let mesh = state
                    .tile_meshes
                    .get(cc, ci)
                    .mesh();

                canvas.reborrow()
                    .translate(rel_to)
                    .draw_mesh(mesh, &ctx.resources().blocks);
            }

            // render the outline for the block being looked at
            let getter = state.chunks.getter();
            let cam_dir = state.movement.cam_dir();
            let looking_at = compute_looking_at(
                state.movement.cam_pos,
                cam_dir,
                100.0,
                &getter,
                &state.tile_blocks,
                ctx.game(),
            );
            if let Some(looking_at) = looking_at {
                let mut canvas = canvas.reborrow()
                    .translate(looking_at.tile.gtc().map(|n| n as f32))
                    .translate(-cam_dir * looking_at.dist / 1000.0)
                    .color(Rgba::black());

                for edge in EDGES {
                    let ranges: Vec3<Range<i32>> = edge
                        .to_vec()
                        .map(|n| match n {
                            -1 => 0..1,
                            0 => 0..2,
                            1 => 1..2,
                            _ => unreachable!(),
                        });
                    let mut points = [Vec3::from(0.0); 2];
                    let mut i = 0;
                    for z in ranges.z {
                        for y in ranges.y.clone() {
                            for x in ranges.x.clone() {
                                points[i] = Vec3 { x, y, z }.map(|n| n as f32);
                                i += 1;
                            }
                        }
                    }
                    debug_assert_eq!(i, 2);
                    let [start, end] = points;
                    canvas.reborrow()
                        .draw_line(start, end);
                }
            }
        }

        // render the crosshair
        let crosshair_size = 30.0 * self.scale;
        canvas.reborrow()
            .translate(-crosshair_size / 2.0)
            .translate(self.size / 2.0)
            .draw_image(&ctx.resources().hud_crosshair, crosshair_size);
    }

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        _button: MouseButton,
    ) {
        // capture mouse on click
        if !hits { return };
        ctx.global.capture_mouse();
    }
}
