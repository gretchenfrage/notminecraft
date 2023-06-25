
use crate::{
    asset::consts::*,
	game_data::{
		GameData,
		blocks::door::{
			DoorMeta,
			DoorPart,
			DoorDir,
		},
	},
	singleplayer::{
		put_block,
		physics::looking_at::LookingAt,
		block_update_queue::BlockUpdateQueue,
	},
};
use chunk_data::{
	AIR,
	TileKey,
	PerChunk,
	ChunkBlocks,
	Face,
	Getter,
};
use mesh_data::{
	MeshData,
	Quad,
};
use std::f32::consts::PI;
use vek::*;


pub fn mesh_door_tile(
    mesh_buf: &mut MeshData,
    tile: TileKey,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) {
	let DoorMeta {
        part,
        dir,
    } = tile.get(tile_blocks).meta(game.bid_door);
    let tex_index = match part {
        DoorPart::Upper => BTI_DOOR_UPPER,
        DoorPart::Lower => BTI_DOOR_LOWER,
    };
    let (pos_start, pos_ext_1, pos_ext_2) = match dir {
        DoorDir::PosX => ([1, 0, 0], [0, 1,  0], [ 0, 0,  1]),
        DoorDir::NegX => ([0, 0, 1], [0, 1,  0], [ 0, 0, -1]),
        DoorDir::PosZ => ([1, 0, 1], [0, 1,  0], [-1, 0,  0]),
        DoorDir::NegZ => ([0, 0, 0], [0, 1,  0], [ 1, 0,  0]),
    };

    let pos_start = Vec3::from(pos_start)
        .map(|n: i32| n as f32);
    let pos_ext_1 = Extent3::from(pos_ext_1)
        .map(|n: i32| n as f32);
    let pos_ext_2 = Extent3::from(pos_ext_2)
        .map(|n: i32| n as f32);
    
    mesh_buf
        .add_quad(&Quad {
            pos_start,
            pos_ext_1,
            pos_ext_2,
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index,
        });
}

pub fn on_place_door(
	cam_yaw: f32,
	tile2: TileKey,
	getter: &Getter,
	tile_blocks: &mut PerChunk<ChunkBlocks>,
	block_updates: &mut BlockUpdateQueue,
	game: &GameData,
) {
	let yaw = ((cam_yaw
        % (2.0 * PI))
        + (2.0 * PI))
        % (2.0 * PI);
    let dir =
        if yaw < 0.25 * PI {
            DoorDir::NegZ
        } else if yaw < 0.75 * PI {
            DoorDir::PosX
        } else if yaw < 1.25 * PI {
            DoorDir::PosZ
        } else if yaw < 1.75 * PI {
            DoorDir::NegX // TODO directions???
        } else if yaw <= 2.0 * PI {
            DoorDir::NegZ
        } else {
            unreachable!()
        };
    let gtc3 = tile2.gtc() + Face::PosY.to_vec();
    if let Some(tile3) = getter.gtc_get(gtc3) {
        let bid3 = tile3
            .get(&*tile_blocks)
            .get();
        let can_place_over = game
            .blocks_can_place_over
            .get(bid3)
            .clone();
        if can_place_over {
            put_block(
                tile2,
                &getter,
                game.bid_door,
                DoorMeta {
                    part: DoorPart::Lower,
                    dir,
                },
                tile_blocks,
                block_updates,
            );
            put_block(
                tile3,
                &getter,
                game.bid_door,
                DoorMeta {
                    part: DoorPart::Upper,
                    dir,
                },
                tile_blocks,
                block_updates,
            );
        }
    }
}

pub fn on_break_door(
	looking_at: LookingAt,
	getter: &Getter,
	tile_blocks: &mut PerChunk<ChunkBlocks>,
	block_updates: &mut BlockUpdateQueue,
	game: &GameData,
) {
	let &DoorMeta { part, .. } = looking_at
        .tile
        .get(&*tile_blocks)
        .meta(game.bid_door);
    let also_break_dir = match part {
        DoorPart::Upper => Face::NegY,
        DoorPart::Lower => Face::PosY,
    };

    let gtc2 =
        looking_at.tile.gtc()
        + also_break_dir.to_vec();
    if let Some(tile2) = getter.gtc_get(gtc2) {
        let bid2 = tile2
            .get(&*tile_blocks)
            .get();
        if bid2 == game.bid_door {
            put_block(
                tile2,
                &getter,
                AIR,
                (),
                tile_blocks,
                block_updates,
            );
        }
    }
}
