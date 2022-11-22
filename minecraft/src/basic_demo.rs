
use crate::{
    chunk_mesh::ChunkMesh,
    game_data::*,
    gui::{
        blocks::simple_gui_block::{
            SimpleGuiBlock,
            simple_blocks_cursor_impl,
        },
        *,
    },
};
use graphics::{
    Renderer,
    frame_content::Canvas2,
};
use chunk_data::*;
use mesh_data::{
    MeshData,
    Quad,
};
use std::{
    mem::replace,
    fs,
};
//use rand::seq::SliceRandom;
use rand_chacha::ChaCha20Rng;
use vek::*;
use rand::prelude::*;


#[derive(Debug)]
pub struct BasicDemo {
    chunks: LoadedChunks,

    tile_blocks: PerChunk<ChunkBlocks>,
    chunk_meshes: PerChunk<ChunkMesh>,

    xml_dump_requested: bool,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut BasicDemo> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>)
    {
        let world = self.inner;

        let mut canvas = canvas.reborrow()
            .begin_3d_perspective(
                self.size,
                [5.0, 5.0, -20.0],
                Quaternion::identity(),
                f32::to_radians(90.0),
            );
        for (cc, ci) in world.chunks.iter() {
            canvas.reborrow()
                .translate((cc * CHUNK_EXTENT).map(|n| n as f32))
                .draw_mesh(
                    world.chunk_meshes.get(cc, ci).mesh(),
                    &ctx.resources().blocks,
                );
        }

        if replace(&mut world.xml_dump_requested, false) {
            info!("initiating xml dump of frame to framedump.xml...");
            fs::write("framedump.xml", canvas.target.to_pseudo_xml())
                .expect("xml dump failed");
            info!("...completed");
        }
    }

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        _button: MouseButton,
    ) {
        if !hits { return };

        ctx.global.capture_mouse();
    }
}

impl BasicDemo {
    pub fn new(
        game: &GameData,
        renderer: &Renderer,
    ) -> Self {
        //let mut rng = thread_rng();
        let mut rng = ChaCha20Rng::from_seed([0; 32]);

        let mut chunks = LoadedChunks::new();

        let mut tile_blocks = PerChunk::new();
        let mut chunk_meshes = PerChunk::new();

        for x in -1..=0 {
            for y in -1..=0 {
                for z in -1..=0 {
                    let cc = Vec3 { x, y, z };
                    let ci = chunks.add(cc);

                    let mut chunk_tile_blocks = ChunkBlocks::new(&game.blocks);
                    for lti in 0..=MAX_LTI {
                        let bid =
                            [
                                AIR,
                                AIR,
                                AIR,
                                AIR,
                                AIR,
                                AIR,
                                game.bid_stone,
                                game.bid_dirt,
                                game.bid_brick,
                            ]
                            .choose(&mut rng)
                            .copied()
                            .unwrap();    
                        chunk_tile_blocks.set(lti, bid, ());
                    }

                    tile_blocks.add(cc, ci, chunk_tile_blocks);
                    chunk_meshes.add(cc, ci, ChunkMesh::new(renderer));
                }
            }
        }


        let mut mesh_buf = MeshData::new();

        for (cc, ci, getter) in chunks.iter_with_getters() {
            let chunk_mesh = chunk_meshes.get_mut(cc, ci);

            for lti in 0..=MAX_LTI {
                let bid = tile_blocks.get(cc, ci).get(lti);
                let mesh_logic = game
                    .block_mesh_logics
                    .get(bid)
                    .unwrap_or(&BlockMeshLogic::Simple(0));
                let ltc = lti_to_ltc(lti);
                let gtc = cc_ltc_to_gtc(cc, ltc);

                match mesh_logic {
                    &BlockMeshLogic::Invisible => (),
                    &BlockMeshLogic::Simple(tex_index) => {
                        for face in FACES {
                            let gtc2 = gtc + face.to_vec();
                            let obscured = getter
                                .gtc_get(gtc2)
                                .and_then(|tile| {
                                    let bid2 = tile.get(&tile_blocks).get();
                                    game.block_obscures.get(bid2)
                                })
                                .map(|obscures| obscures[-face])
                                .unwrap_or(false);
                            if !obscured {
                                let (
                                    rel_pos_start,
                                    pos_ext_1,
                                    pos_ext_2,
                                ) = match face {
                                    Face::PosX => ([1, 0, 0], [0, 1,  0], [ 0, 0,  1]),
                                    Face::NegX => ([0, 0, 1], [0, 1,  0], [ 0, 0, -1]),
                                    Face::PosY => ([0, 1, 0], [0, 0,  1], [ 1, 0,  0]),
                                    Face::NegY => ([0, 0, 1], [0, 0, -1], [ 1, 0,  0]),
                                    Face::PosZ => ([1, 0, 1], [0, 1,  0], [-1, 0,  0]),
                                    Face::NegZ => ([0, 0, 0], [0, 1,  0], [ 1, 0,  0]),
                                };
                                mesh_buf
                                    .add_quad(&Quad {
                                        pos_start: (ltc + Vec3::from(rel_pos_start)).map(|n| n as f32),
                                        pos_ext_1: Extent3::from(pos_ext_1).map(|n: i32| n as f32),
                                        pos_ext_2: Extent3::from(pos_ext_2).map(|n: i32| n as f32),
                                        tex_start: 0.0.into(),
                                        tex_extent: 1.0.into(),
                                        vert_colors: [Rgba::white(); 4],
                                        tex_index,
                                    });
                            }
                        }
                    }
                }

                chunk_mesh.set_tile_submesh(lti, &mesh_buf);
                mesh_buf.clear();
            }
        }

        BasicDemo {
            chunks,

            tile_blocks,
            chunk_meshes,

            xml_dump_requested: false,
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

impl GuiStateFrame for BasicDemo {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, _elapsed: f32) {
        for (cc, ci) in self.chunks.iter() {
            self.chunk_meshes
                .get_mut(cc, ci)
                .patch(&ctx.global().renderer.borrow());
        }
    }

    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {
        if key == VirtualKeyCode::Escape {
            ctx.global().pop_state_frame();
        } else if key == VirtualKeyCode::X {
            self.xml_dump_requested = true;
        }
    }
}
