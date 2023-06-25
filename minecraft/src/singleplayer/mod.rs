
mod blocks;
mod block_update_queue;
mod chunk_loader;
mod movement;
mod tile_meshing;
mod physics;

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
    physics::looking_at::compute_looking_at,
};
use crate::{
    game_data::{
        GameData,
        BlockBreakLogic,
    },
    chunk_mesh::ChunkMesh,
    gui::{
        blocks::{
            simple_gui_block::{
                SimpleGuiBlock,
                simple_blocks_cursor_impl,
            },
            *,
            mc::*,
        },
        *,
    },
    util::{
        number_key::num_row_key,
        array::{
            array_each,
            array_const_slice,
        },
    },
    item::slots::ItemSlot,
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
    f32::consts::PI,
    iter,
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

    reach: f32,

    //hotbar_items: [Option<HotbarItem>; 9],
    inventory_slots: Box<[ItemSlot; 36]>,
    hotbar_selected: usize,

    inventory_open: bool,

    evil_animation: f32,
 
    _debug_cube_mesh: Mesh,
    //_human_mesh: Mesh,

    time_since_step_sound: f32,

    particles: Vec<Particle>,
}

#[derive(Debug, Copy, Clone)]
struct Particle {
    pos: Vec3<f32>,
    vel: Vec3<f32>,
    age: f32,
}

/*
fn simple_hud_mesh(tex_index: usize, renderer: &Renderer) -> Mesh {
    let mut mesh_buf = MeshData::new();
    let shade = 0.5;
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [1.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [0.0, 0.0, 1.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 1.0, 0.0].into(),
            pos_ext_1: [0.0, 0.0, 1.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index,
        });
    mesh_buf.upload(renderer)
}

fn door_hud_mesh(renderer: &Renderer) -> Mesh {
    let mut mesh_buf = MeshData::new();
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.25, 0.0, 0.5].into(),
            pos_ext_1: [0.0, 0.5, 0.0].into(),
            pos_ext_2: [0.5, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index: 10,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.25, 0.5, 0.5].into(),
            pos_ext_1: [0.0, 0.5, 0.0].into(),
            pos_ext_2: [0.5, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index: 9,
        });
    mesh_buf.upload(renderer)
}
*/
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
/*
macro_rules! game_gui {
    (
        $logical_size:expr,
        $background_img:expr,
        $( ($elem_pos:expr, $elem:expr) ),*$(,)?
    )=>{

    };
}*/

impl Singleplayer {
    pub fn new(game: &Arc<GameData>, renderer: &Renderer) -> Self {
        let chunk_loader = ChunkLoader::new(game, renderer);

        let view_dist = 6;

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

            reach: 12.0,
            inventory_open: false,

            /*
            hotbar_items: [
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_stone,
                    hud_mesh: simple_hud_mesh(0, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_dirt,
                    hud_mesh: simple_hud_mesh(1, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_grass,
                    hud_mesh: simple_hud_mesh(2, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_planks,
                    hud_mesh: simple_hud_mesh(4, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_brick,
                    hud_mesh: simple_hud_mesh(5, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_glass,
                    hud_mesh: simple_hud_mesh(6, renderer),
                }),
                Some(HotbarItem::SimpleBlock {
                    bid: game.bid_log,
                    hud_mesh: simple_hud_mesh(7, renderer),
                }),
                Some(HotbarItem::Door {
                    hud_mesh: door_hud_mesh(renderer),
                }),
                None,
            ],*/
            inventory_slots: iter::from_fn(|| Some(ItemSlot::default()))
                .take(36)
                .collect::<Box<[_]>>()
                .try_into().unwrap(),
            hotbar_selected: 0,

            evil_animation: 0.0,

            _debug_cube_mesh: debug_cube_mesh,

            time_since_step_sound: 0.0,
            particles: Vec::new(),
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        let hand_w = ctx.size.w as f32 * 0.3 / ctx.scale;
        let hand_h = hand_w / 2368.0 * 3014.0;
        let _hand_size = Extent2::new(hand_w, hand_h);
        layer((
            WorldGuiBlock {
                movement: &self.movement,
                chunks: &self.chunks,
                tile_meshes: &mut self.tile_meshes,
                tile_blocks: &self.tile_blocks,
                reach: self.reach,
                debug_cube_mesh: &self._debug_cube_mesh,
                particles: &self.particles,
            },
            /*
            align([1.1, 1.1],
                modify(Transform2::translate(hand_size),
                    modify(Transform2::rotate(PI * 0.10 * self.evil_animation.sin()),
                        modify(Transform2::translate(-hand_size),
                            logical_size(hand_size,
                                layer((
                                    align([0.25, 0.25],
                                        logical_size(0.0,
                                            align(0.5,
                                                logical_size(hand_w * 0.75,
                                                    HotbarItemGuiBlock {
                                                        item: &self.hotbar_items[self.hotbar_selected],
                                                    },
                                                ),
                                            ),
                                        ),
                                    ),
                                    &self.evil_jpg,
                                )),
                            ),
                        ),
                    ),
                ),
            ),
            */
            align(0.5,
                logical_size(30.0,
                    &ctx.assets().hud_crosshair,
                ),
            ),
            align([0.5, 1.0],
                logical_size([364.0, 44.0],
                    layer((
                        &ctx.assets().hud_hotbar,
                        align(0.5,
                            logical_height(40.0,
                                h_stack(0.0,
                                    array_each(
                                        array_const_slice::<_, 9>(&*self.inventory_slots, 0)
                                    )
                                        .map(|slot| v_align(0.5,
                                            slot.gui()
                                        ))
                                ),
                            )
                        ),
                        align([self.hotbar_selected as f32 / 8.0, 0.5],
                            logical_size([44.0, 44.0],
                                align(0.5,
                                    logical_size(48.0,
                                        &ctx.assets().hud_hotbar_selected,
                                    )
                                )
                            )
                        ),
                    )),
                ),
            ),
            CaptureMouseGuiBlock,
            if self.inventory_open {
                GuiEither::A(
                    align(0.5,
                        
                        relative(
                            (),
                            logical_size([352.0, 332.0],
                                &ctx.assets().gui_inventory,
                            ),
                            gui_seq_flatten!(
                                array_each(
                                    array_const_slice::<_, { 9 * 3 }>(&*self.inventory_slots, 9)
                                )
                                    .map({
                                        let mut col = 0;
                                        let mut row = 0;
                                        move |slot| {
                                            let trans =
                                                (
                                                    Vec2::from([7.0, 83.0])
                                                    + (
                                                        Extent2::from(18.0)
                                                        * Vec2::from([col, row])
                                                            .map(|n: u32| n as f32)
                                                    )
                                                ) * 2.0;

                                            col += 1;
                                            debug_assert!(col <= 9);
                                            if col == 9 {
                                                col = 0;
                                                row += 1;
                                            }

                                            logical_translate(trans, slot.gui())
                                        }
                                    }),
                                array_each(
                                    array_const_slice::<_, 9>(&*self.inventory_slots, 0)
                                )
                                    .map({
                                        let mut col = 0;
                                        move |slot| {
                                            let trans =
                                            (
                                                Vec2::from([7.0, 141.0])
                                                + (
                                                    Extent2::from(18.0)
                                                    * Vec2::from([col, 0])
                                                        .map(|n: u32| n as f32)
                                                )
                                            ) * 2.0;

                                            col += 1;

                                            logical_translate(trans, slot.gui())
                                        }
                                    })
                            ),
                        )
                        
                    )
                )
            } else {
                GuiEither::B(gap())
            },
        ))
    }
}
/*
#[derive(Debug)]
struct HotbarItemGuiBlock<'a> {
    item: &'a Option<HotbarItem>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<HotbarItemGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>)
    {
        match self.inner.item {
            Some(HotbarItem::SimpleBlock { hud_mesh, .. }) => {
                let view_proj = Mat4::new(
                    1.0, 0.0, 0.0, 0.5,
                    0.0, -1.0, 0.0, 0.5,
                    0.0, 0.0, 0.01, 0.5,
                    0.0, 0.0, 0.0, 1.0,
                );
                canvas.reborrow()
                    .scale(self.size)
                    .begin_3d(view_proj)
                    .scale(0.5)
                    .rotate(Quaternion::rotation_x(-PI / 5.0))
                    .rotate(Quaternion::rotation_y(PI / 4.0))
                    .translate(-0.5)
                    .draw_mesh(hud_mesh, &ctx.assets().blocks);
                /*
                let view_proj =
                    Mat4::<f32>::translation_3d([0.0, 0.0, 0.5])
                    * Mat4::<f32>::scaling_3d([0.5, 0.5, 0.01])
                    * Mat4::<f32>::rotation_z(PI / 4.0)
                    * Mat4::<f32>::rotation_y(PI / 4.0);
                canvas.reborrow()
                    .scale(self.size)
                    .begin_3d(view_proj)
                    .draw_mesh(self.inner.cube_mesh, &ctx.assets().blocks);
                    */
            },
            Some(HotbarItem::Door { hud_mesh }) => {
                let view_proj = Mat4::new(
                    1.0, 0.0, 0.0, 0.5,
                    0.0, -1.0, 0.0, 0.5,
                    0.0, 0.0, 0.01, 0.5,
                    0.0, 0.0, 0.0, 1.0,
                );
                canvas.reborrow()
                    .scale(self.size)
                    .begin_3d(view_proj)
                    .scale(0.75)
                    .rotate(Quaternion::rotation_x(-PI / 5.0))
                    .rotate(Quaternion::rotation_y(PI / 4.0))
                    .translate(-0.5)
                    .draw_mesh(hud_mesh, &ctx.assets().blocks);
            }
            None => (),
        }
    }
}*/

 /*
        let crosshair_size = 30.0 * self.scale;
        canvas.reborrow()
            .translate(-crosshair_size / 2.0)
            .translate(self.size / 2.0)
            .draw_image(&ctx.assets().hud_crosshair, crosshair_size);
            */
impl GuiStateFrame for Singleplayer {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
        self.evil_animation -= elapsed * 15.0;
        self.evil_animation = f32::max(self.evil_animation, 0.0);
        self.time_since_step_sound += elapsed;

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
        self.movement.vel_v -= 18.0 * elapsed;
        self.movement.update(ctx.global(), &self.bindings, elapsed);
        let body_extent = Extent3::new(0.5, 1.98, 0.5);
        let body_offset = Vec3::new(0.25, 1.93, 0.25);
        let mut pos = self.movement.cam_pos - body_offset;
        let mut vel =
            Vec3::new(
                self.movement.vel_h.x,
                self.movement.vel_v,
                self.movement.vel_h.y,
            );

        use physics::{
            do_physics::do_physics,
            collision::aa_box::AaBoxCollisionObject,
            collision::point::PointCollisionObject,
            world_geometry::WorldPhysicsGeometry,
        };

        let did_physics = do_physics(
            elapsed,
            &mut pos,
            &mut vel,
            &AaBoxCollisionObject { ext: body_extent },
            //&PointCollisionObject,
            &WorldPhysicsGeometry {
                getter: &getter,
                tile_blocks: &self.tile_blocks,
                game: ctx.game(),
            },
        );
        self.movement.cam_pos = pos + body_offset;
        self.movement.vel_h.x = vel.x;
        self.movement.vel_v = vel.y;
        self.movement.vel_h.y = vel.z;
        self.movement.on_ground = did_physics.on_ground;

        if did_physics.on_ground && (vel.x != 0.0 || vel.z != 0.0) && self.time_since_step_sound >= 0.5 {
            ctx.sound_player().play(&ctx.assets().grass_step_sound);
            self.time_since_step_sound = 0.0;
        }

        for i in (0..self.particles.len()).rev() {
            self.particles[i].age += elapsed;
            if self.particles[i].age > 3.0 {
                self.particles.swap_remove(i);
                continue;
            }
            let particle = &mut self.particles[i];
            particle.vel.y -= 18.0 * elapsed;
            do_physics(
                elapsed,
                &mut particle.pos,
                &mut particle.vel,
                &PointCollisionObject,
                &WorldPhysicsGeometry {
                    getter: &getter,
                    tile_blocks: &self.tile_blocks,
                    game: ctx.game(),
                },
            );
        }
    }

    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {
        if key == VirtualKeyCode::Escape {
            ctx.global().pop_state_frame();
        } else if key == VirtualKeyCode::Space {
            if self.movement.on_ground {
                self.movement.vel_v += 6.5;
                self.movement.on_ground = false;
            }
        } else if let Some(n) = num_row_key(key) {
            if n >= 1 && n <= 9 {
                self.hotbar_selected = n as usize - 1;
            }
        } else if key == VirtualKeyCode::E {
            if !self.inventory_open {
                self.inventory_open = true;
                ctx.global().uncapture_mouse();
            } else {
                self.inventory_open = false;
                ctx.global().capture_mouse();
            }
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
        self.evil_animation = PI;

        let getter = self.chunks.getter();
        let looking_at = compute_looking_at(
            self.movement.cam_pos,
            self.movement.cam_dir(),
            self.reach,
            &getter,
            &self.tile_blocks,
            ctx.game(),
        );
        if let Some(looking_at) = looking_at {
            match button {
                MouseButton::Left => {
                    let bid = looking_at.tile.get(&self.tile_blocks).get();
                    let break_logic = ctx.game().blocks_break_logic.get(bid);
                    match break_logic {
                        &BlockBreakLogic::Null => (),
                        &BlockBreakLogic::Door => blocks::door::on_break_door(
                            looking_at,
                            &getter,
                            &mut self.tile_blocks,
                            &mut self.block_updates,
                            ctx.game(),
                        ),
                    }
                    put_block(
                        looking_at.tile,
                        &getter,
                        AIR,
                        (),
                        &mut self.tile_blocks,
                        &mut self.block_updates,
                    );
                    ctx.sound_player().play(&ctx.assets().grass_dig_sound);
                    for i in 0..4 {
                        self.particles.push(Particle {
                            pos: looking_at.tile.gtc().map(|n| n as f32) + i as f32 / 4.0,
                            vel: (i as f32).into(),
                            age: 0.0,
                        });
                    }
                }
                MouseButton::Right => {
                    let tile1 = looking_at.tile;
                    let gtc1 = cc_ltc_to_gtc(tile1.cc, lti_to_ltc(tile1.lti));
                    if let Some(tile2) = looking_at
                        .face
                        .and_then(|face| getter.gtc_get(gtc1 + face.to_vec()))
                    {
                        /*
                        match self.hotbar_items[self.hotbar_selected] {
                            Some(HotbarItem::SimpleBlock { bid, .. }) => {
                                put_block(
                                    tile2,
                                    &getter,
                                    bid,
                                    (),
                                    &mut self.tile_blocks,
                                    &mut self.block_updates,
                                );
                                ctx.sound_player().play(&ctx.assets().grass_dig_sound);
                            }
                            Some(HotbarItem::Door { .. }) => blocks::door::on_place_door(
                                self.movement.cam_yaw,
                                tile2,
                                &getter,
                                &mut self.tile_blocks,
                                &mut self.block_updates,
                                ctx.game(),
                            ),
                            None => (),
                        }
                        */
                    }
                }
                _ => (),
            }
        }
    }
}

#[derive(Debug)]
struct WorldGuiBlock<'a> {
    movement: &'a MovementController,
    chunks: &'a LoadedChunks,
    tile_meshes: &'a mut PerChunk<ChunkMesh>,
    tile_blocks: &'a PerChunk<ChunkBlocks>,
    reach: f32,
    debug_cube_mesh: &'a Mesh,
    particles: &'a Vec<Particle>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>)
    {
        let state = self.inner;

        canvas.reborrow()
            .color(ctx.assets().sky_day)
            .draw_solid(self.size);

        {
            let view_proj = state.movement.view_proj(self.size);
            let mut canvas = canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj);

            for particle in state.particles {
                canvas.reborrow()
                    .translate(particle.pos)
                    .scale(1.0 / 8.0)
                    .rotate(state.movement.cam_rot())
                    .draw_image(
                        &ctx.assets().blocks,
                        1,
                        0.0,
                        0.1,
                    );
            }


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
                    .draw_mesh(mesh, &ctx.assets().blocks);
            }

            // render the outline for the block being looked at
            let getter = state.chunks.getter();
            let cam_dir = state.movement.cam_dir();
            let looking_at = compute_looking_at(
                state.movement.cam_pos,
                cam_dir,
                state.reach,
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

            /*
            let body_extent = Extent3::new(0.5, 1.98, 0.5);
            canvas.reborrow()
                .translate(state.movement.cam_pos)
                .scale(body_extent)
                .translate([-0.5, -1.0, -0.5])
                .color([1.0, 1.0, 1.0, 0.6])
                .draw_mesh(state.debug_cube_mesh, &ctx.assets().blocks);*/
        }

        // render the crosshair
        /*
        let crosshair_size = 30.0 * self.scale;
        canvas.reborrow()
            .translate(-crosshair_size / 2.0)
            .translate(self.size / 2.0)
            .draw_image(&ctx.assets().hud_crosshair, crosshair_size);
            */
    }
}

#[derive(Debug)]
struct CaptureMouseGuiBlock;

impl<'a> GuiNode<'a> for SimpleGuiBlock<CaptureMouseGuiBlock> {
    simple_blocks_cursor_impl!();

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
