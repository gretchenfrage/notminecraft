//! Data structures for efficient storage of chunk data in memory.
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


pub mod coord;
pub mod per_tile;
pub mod block;
pub mod loaded;
pub mod per_tile_sparse;
pub mod per_tile_packed;


use self::{
    block::{
        ChunkBlocks,
        RawBlockId,
        BlockId,
    },
    per_tile::PerTile,
    per_tile_sparse::PerTileSparse,
    per_tile_packed::PerTilePacked,
};
use std::fmt::Debug;
use slab::Slab;

/*
pub trait CiLtiGet {
    type Output;

    fn get(self, ci: usize, lti: u16) -> Self::Output;
}

impl<C> CiLtiGet for C
where
    C: CiGet,
    <C as CiGet>::Output: LtiGet,
{
    type Output = <<C as CiGet>::Output as LtiGet>::Output;

    fn get(self, ci: usize, lti: u16) -> Self::Output {

    }
}
*/

pub trait CiGet {
    type Output;

    fn get(self, ci: usize) -> Self::Output;
}

impl<'a, T> CiGet for &'a Slab<T> {
    type Output = &'a T;

    fn get(self, ci: usize) -> Self::Output {
        &self[ci]
    }
}

impl<'a, T> CiGet for &'a mut Slab<T> {
    type Output = &'a mut T;

    fn get(self, ci: usize) -> Self::Output {
        &mut self[ci]
    }
}


pub trait LtiGet {
    type Output;

    fn get(self, lti: u16) -> Self::Output;
}

pub trait LtiSet { // TODO remove trait
    type Input;

    fn set(self, lti: u16, val: Self::Input);
}

impl<'a, T: 'a, C: LtiGet<Output=&'a mut T>> LtiSet for C {
    type Input = T;

    fn set(self, lti: u16, val: Self::Input) {
        *self.get(lti) = val;
    }
}
/*
impl<'a> LtiGet for &'a ChunkBlocks {
    type Output = RawBlockId;

    fn get(self, lti: u16) -> Self::Output {
        ChunkBlocks::get(self, lti)
    }
}
*/
impl<'a, T> LtiGet for &'a PerTile<T> {
    type Output = &'a T;

    fn get(self, lti: u16) -> Self::Output {
        &self[lti]
    }
}

impl<'a, T> LtiGet for &'a mut PerTile<T> {
    type Output = &'a mut T;

    fn get(self, lti: u16) -> Self::Output {
        &mut self[lti]
    }
}

impl<'a, T> LtiGet for &'a PerTileSparse<T> {
    type Output = Option<&'a T>;

    fn get(self, lti: u16) -> Self::Output {
        PerTileSparse::get(self, lti)
    }
}

impl<'a, T> LtiSet for &'a mut PerTileSparse<T> {
    type Input = Option<T>;

    fn set(self, lti: u16, val: Self::Input) {
        PerTileSparse::set(self, lti, val)
    }
}

impl<
    'a,
    const BYTES: usize,
    const MASK: u8,
> LtiGet for &'a PerTilePacked<BYTES, MASK> {
    type Output = u8;

    fn get(self, lti: u16) -> Self::Output {
        PerTilePacked::get(self, lti)
    }
}

impl<
    'a,
    const BYTES: usize,
    const MASK: u8,
> LtiSet for &'a mut PerTilePacked<BYTES, MASK> {
    type Input = u8;

    fn set(self, lti: u16, val: Self::Input) {
        PerTilePacked::set(self, lti, val)
    }
}

impl<'a> LtiGet for &'a ChunkBlocks {
    type Output = TileBlockRead<'a>;

    fn get(self, lti: u16) -> Self::Output {
        TileBlockRead { chunk: self, lti }
    }
}

impl<'a> LtiGet for &'a mut ChunkBlocks {
    type Output = TileBlockWrite<'a>;

    fn get(self, lti: u16) -> Self::Output {
        TileBlockWrite { chunk: self, lti }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TileBlockRead<'a> {
    pub chunk: &'a ChunkBlocks,
    pub lti: u16,
}

impl<'a> TileBlockRead<'a> {
    pub fn get(self) -> RawBlockId {
        self.chunk.get(self.lti)
    }

    pub fn raw_meta<M>(self) -> &'a M
    where
        M: 'static,
    {
        self.chunk.raw_meta::<M>(self.lti)
    }

    pub fn meta<M>(self, bid: BlockId<M>) -> &'a M
    where
        M: 'static,
    {
        self.chunk.meta(bid, self.lti)
    }

    pub fn try_meta<M>(self, bid: BlockId<M>) -> Option<&'a M>
    where
        M: 'static,
    {
        self.chunk.try_meta(bid, self.lti)
    }

    pub fn meta_debug(self) -> impl Debug + 'a {
        self.chunk.meta_debug(self.lti)
    }
}

#[derive(Debug)]
pub struct TileBlockWrite<'a> {
    pub chunk: &'a mut ChunkBlocks,
    pub lti: u16,
}

impl<'a> TileBlockWrite<'a> {
    pub fn reborrow<'a2>(&'a2 mut self) -> TileBlockWrite<'a2> {
        TileBlockWrite {
            chunk: self.chunk,
            lti: self.lti,
        }
    }

    pub fn read(self) -> TileBlockRead<'a> {
        TileBlockRead {
            chunk: self.chunk,
            lti: self.lti,
        }
    }

    pub fn raw_set<M>(&mut self, bid: RawBlockId, meta: M)
    where
        M: 'static,
    {
        self.chunk.raw_set(self.lti, bid, meta);
    }

    pub fn set<M>(&mut self, bid: BlockId<M>, meta: M)
    where
        M: 'static,
    {
        self.chunk.set(self.lti, bid, meta);
    }

    pub fn raw_meta<M>(self) -> &'a mut M
    where
        M: 'static,
    {
        self.chunk.raw_meta_mut::<M>(self.lti)
    }

    pub fn meta<M>(self, bid: BlockId<M>) -> &'a mut M
    where
        M: 'static,
    {
        self.chunk.meta_mut(bid, self.lti)
    }

    pub fn try_meta<M>(self, bid: BlockId<M>) -> Option<&'a mut M>
    where
        M: 'static,
    {
        self.chunk.try_meta_mut(bid, self.lti)
    }
}
