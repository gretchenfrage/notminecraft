
mod block_update_queue;
mod chunk_loader;
mod movement;
mod tile_meshing;

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
    FACES_EDGES_CORNERS,
    CHUNK_EXTENT,
    LoadedChunks,
    PerChunk,
    TileKey,
    ChunkBlocks,
    Getter,
    lti_to_ltc,
};
use mesh_data::MeshData;
use graphics::{
    Renderer,
    frame_content::Canvas2,
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

    info!(?cc, "inserting chunk");

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

        // TODO
        assert!(
            vertex.pos.x > -100.0
            && vertex.pos.x < 100.0
            && vertex.pos.y > -100.0
            && vertex.pos.y < 100.0
            && vertex.pos.z > -100.0
            && vertex.pos.z < 100.0
        );
    }

    tile.set(tile_meshes, mesh_buf);
}

impl Singleplayer {
    pub fn new(game: &Arc<GameData>, renderer: &Renderer) -> Self {
        let chunk_loader = ChunkLoader::new(game, renderer);

        let view_dist = 10;

        for x in -view_dist..view_dist {
            for z in -view_dist..view_dist {
                chunk_loader.request(Vec3::new(x, 0, z));
            }
        }

        Singleplayer {
            bindings: KeyBindings::default(),
            movement: MovementController::default(),

            chunks: LoadedChunks::new(),

            tile_blocks: PerChunk::new(),
            tile_meshes: PerChunk::new(),

            block_updates: BlockUpdateQueue::new(),
            chunk_loader,
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
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut Singleplayer> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>)
    {
        let state = self.inner;

        canvas.reborrow()
            .color(ctx.resources().sky_day)
            .draw_solid(self.size);

        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(state.movement.view_proj(self.size));

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
