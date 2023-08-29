//! Data structures for efficient storage of chunk data in memory.
//!
//! Basic example:
//!
//! ```
//! use chunk_data::{
//!     BlockRegistry,
//!     BlockId,
//!     LoadedChunks,
//!     PerChunk,
//!     ChunkBlocks,
//!     PerTile,
//! };
//! use vek::*;
//!
//! // initialize game data
//! let mut blocks = BlockRegistry::new();
//! 
//! let bid_stone: BlockId<()> = blocks.register();
//! let bid_switch: BlockId<bool> = blocks.register();
//! let bid_label: BlockId<String> = blocks.register();
//!
//! let blocks = blocks.finalize();
//!
//! // initialize world data
//! let mut chunks = LoadedChunks::new();
//! 
//! let mut tile_blocks: PerChunk<ChunkBlocks> = PerChunk::new();
//! let mut tile_niceness: PerChunk<PerTile<f32>> = PerChunk::new();
//!
//! // load a chunk
//! let cc = Vec3::new(1, 0, 0);
//!
//! {
//!     let ci = chunks.add(cc);
//!     
//!     tile_blocks.add(cc, ci, ChunkBlocks::new(&blocks));
//!     tile_niceness.add(cc, ci, PerTile::default());
//! }
//!
//! // read and write stuff in that chunk a bit
//! let getter = chunks.getter();
//! getter.gtc_get([47, 14, 20]).unwrap().get(&mut tile_blocks).set(bid_stone, ());
//! if let Some(tile) = getter.gtc_get([47, 15, 20]) {
//!     tile.get(&mut tile_blocks).set(bid_switch, true);
//!     tile.set(&mut tile_niceness, 5.0);
//!     *tile.get(&mut tile_niceness) *= 2.0;
//! }
//! getter.gtc_get([47, 16, 20]).unwrap().get(&mut tile_blocks)
//!     .set(
//!         bid_label,
//!         String::from("hello world!"),
//!     );
//! assert_eq!(
//!     format!(
//!         "{:?}",
//!         getter.gtc_get([47, 16, 20]).map(|tile| tile.get(&tile_blocks).meta_debug()),
//!     ),
//!     r#"Some("hello world!")"#,
//! );
//! ```
//!
//! ## tiles, global tile coordinates
//!
//! The world contains a grid of _tiles_ which extends indefinitely in all
//! directions. A tile can be globally identified by a _global tile coordinate_
//! (gtc) a 3-vec of signed integers wherein the physical volume occupied that
//! tile starts at those coordinates and has an extent of <1,1,1>.
//!
//! ## chunks, chunk coordinates
//!
//! Tiles are grouped into _chunks_, which are cuboids of size 32 by 32 tiles
//! and 64 tiles tall. A chunk can be globally identified by a _chunk
//! coordinate_, (cc) a 3-vec of signed integers wherein the physical volume
//! occupied by that chunk starts at <32,64,32> times those coordinates and has
//! an extent of <32,64,32>.
//!
//! ## local tile coordinates
//!
//! Relative to some chunk, a tile within that chunk can be identified by a
//! _local tile coordinate_ (ltc), a 3-vec of integers between <0,0,0> (inclusive)
//! and <32,64,32> (exclusive). Chunks being 32x64x32 tiles means that each
//! chunk contains exactly 2^16 tiles, and furthermore, that the <x,y,z>
//! components of a local tile coordinate can be represented with 5, 6, and 5
//! bits respectively.
//!
//! ## local tile indices
//!
//! We actually represent local tile coordinates as a newtype around a `u16`.
//! If the <x,y,z> coordinates are composed of bits as such:
//!
//! ```
//! <x5|x4|x3|x2|x1, y6|y5|y4|y3|y2|y1, z5|z4|z3|z2|z1>
//! ```
//!
//! We pack them into a `u16` as such:
//!
//! ```
//! z5|y6|y5|x5 | z4|z3|z2|z1 | y4|y3|y2|y1 | x4|x3|x2|x1
//! ```
//!
//! We one packs an ltc and then interprets that as a single int, we call that
//! a _local tile index_ (lti).
//!
//! When one wants to store some value for each tile in a chunk, they can
//! simply create an array of length 2^16, or `0x10000`, and treat the way
//! ltcs bit-pack as the mapping from coordinates to indices. The way the
//! coordinates pack achieves the property that _sub-chunks_ of 16x16x16 tiles
//! are stored together, thus improving cache locality.
//!
//! Furthermore, this system means that, if one simply wants to iterate over
//! each tile in a chunk, instead of having triple-nested loops, one can simply
//! have a single loop from 0 to 0xffff (inclusive) or 0x10000 (exclusive).
//! Furthermore, if one then wants to convert from that index into an ltc, they
//! can simply bit-unpack it.
//!
//! Furthermore, one can perform a (gtc) -> (cc, ltc) conversion by bit-packing
//! the lower 5,6,5 bits of the gtc's <x,y,z> coordinates into the ltc, and
//! right-shifting them by that many bits to form the cc. One can perform a
//! (cc, ltc) -> (gtc) conversion by doing that in reverse.
//!
//! ## loaded chunks, chunk indices
//!
//! The set of hypothetical chunks in the world extends indefinitely in all
//! directions and thus is generated and loaded lazily. As such, a game has a
//! concept of a set of _loaded chunks_, which changes over time. A loaded
//! chunk is sequentially assigned a _chunk index_, which may be reused if and
//! after that chunk is unloaded.


mod coord;
mod axis;
mod loaded;
mod per_chunk;
mod block;
mod per_tile;
mod per_tile_packed;
mod per_tile_sparse;
mod per_tile_option;
mod nice_api;


pub use self::{
    coord::{
        MAX_LTI,
        NUM_LTIS,
        MAX_LTC_X,
        MAX_LTC_Y,
        MAX_LTC_Z,
        CHUNK_EXTENT,
        ltc_to_lti,
        lti_get_x,
        lti_get_y,
        lti_get_z,
        lti_set_x,
        lti_set_y,
        lti_set_z,
        lti_to_ltc,
        gtc_get_cc,
        gtc_get_ltc,
        gtc_get_lti,
        cc_ltc_to_gtc,
    },
    axis::{
        NUM_AXES,
        AXES,
        NUM_POLES,
        POLES,
        NUM_SIGNS,
        SIGNS,
        NUM_FACES,
        FACES,
        NUM_EDGES,
        EDGES,
        NUM_CORNERS,
        CORNERS,
        NUM_FACES_EDGES_CORNERS,
        FACES_EDGES_CORNERS,
        Axis,
        PerAxis,
        Pole,
        PerPole,
        Sign,
        PerSign,
        Face,
        PerFace,
        Edge,
        PerEdge,
        Corner,
        PerCorner,
        FaceEdgeCorner,
        PerFaceEdgeCorner,
    },
    loaded::{
        LoadedChunks,
        Getter,
    },
    per_chunk::PerChunk,
    block::{
        AIR,
        RawBlockId,
        BlockId,
        BlockRegistry,
        ChunkBlocks,
        ErasedBlockMeta,
        MaybeBoxed,
    },
    per_tile::PerTile,
    per_tile_packed::{
        PerTilePacked,
        PerTileU4,
        PerTileU2,
        PerTileU1,
        PerTileBool,
    },
    per_tile_sparse::PerTileSparse,
    per_tile_option::PerTileOption,
    nice_api::{
        TileKey,
        CiGet,
        LtiGet,
        LtiSet,
        TileBlockRead,
        TileBlockWrite,
    },
};
