
use crate::{
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
use vek::*;


/// Pre-processed and looked-up key for a tile in a currently loaded chunk.
///
/// This is part of the nice chainable API.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TileKey {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub lti: u16,
}

impl TileKey {
    pub fn get<T>(
        self,
        per_chunk: T,
    ) -> <<T as CiGet>::Output as LtiGet>::Output
    where
        T: CiGet,
        <T as CiGet>::Output: LtiGet,
    {
        per_chunk.get(self.cc, self.ci).get(self.lti)
    }

    pub fn set<T, V>(
        self,
        per_chunk: T,
        val: V,
    )
    where
        T: CiGet,
        <T as CiGet>::Output: LtiSet<V>,
    {
        per_chunk.get(self.cc, self.ci).set(self.lti, val);
    }
}

/// Something gettable per-chunk, for nice chaininable API.
pub trait CiGet {
    type Output;

    fn get(self, cc: Vec3<i64>, ci: usize) -> Self::Output;
}

/// Something gettable per-local tile, for nice chainable API.
pub trait LtiGet {
    type Output;

    fn get(self, lti: u16) -> Self::Output;
}

/// Something settable per-local tile, for nice chainable API.
pub trait LtiSet<T> {
    fn set(self, lti: u16, val: T);
}

impl<'a, T: 'a, C: LtiGet<Output=&'a mut T>> LtiSet<T> for C {
    fn set(self, lti: u16, val: T) {
        *self.get(lti) = val;
    }
}

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

impl<'a, T> LtiSet<Option<T>> for &'a mut PerTileSparse<T> {
    fn set(self, lti: u16, val: Option<T>) {
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
> LtiSet<u8> for &'a mut PerTilePacked<BYTES, MASK> {
    fn set(self, lti: u16, val: u8) {
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

/// Reader for block ID and metadata for a single tile.
///
/// Notably, implements `Copy`.
#[derive(Copy, Clone, Debug)]
pub struct TileBlockRead<'a> {
    pub chunk: &'a ChunkBlocks,
    pub lti: u16,
}

impl<'a> TileBlockRead<'a> {
    /// Get the block ID.
    pub fn get(self) -> RawBlockId {
        self.chunk.get(self.lti)
    }

    /// Get the block metadata, without checking block ID.
    ///
    /// Panics if `M` is not the meta type for the block at this tile.
    pub fn raw_meta<M>(self) -> &'a M
    where
        M: 'static,
    {
        self.chunk.raw_meta::<M>(self.lti)
    }

    /// Get the block metadata.
    ///
    /// Panics if:
    /// - `bid` is not the block ID at this tile.
    /// - `M` is not the meta type for `bid` (unlikely to occur accidentally).
    pub fn meta<M>(self, bid: BlockId<M>) -> &'a M
    where
        M: 'static,
    {
        self.chunk.meta(bid, self.lti)
    }

    /// If the block at this tile is `bid`, get its metadata.
    ///
    /// If `bid` is the block at this tile, panics if `M` is not the meta type
    /// for `bid` (unlikely to occur accidentally).
    pub fn try_meta<M>(self, bid: BlockId<M>) -> Option<&'a M>
    where
        M: 'static,
    {
        self.chunk.try_meta(bid, self.lti)
    }

    /// Way of debug-formatting the block metadata at this tile without knowing
    /// its type.
    pub fn meta_debug(self) -> impl Debug + 'a {
        self.chunk.meta_debug(self.lti)
    }
}

/// Writer for block ID and metadata for a single tile.
#[derive(Debug)]
pub struct TileBlockWrite<'a> {
    pub chunk: &'a mut ChunkBlocks,
    pub lti: u16,
}

impl<'a> TileBlockWrite<'a> {
    /// Convert a `&'a2 mut TileBlockWrite<'_>` to a `TileBlockWrite<'a2>`.
    pub fn reborrow<'a2>(&'a2 mut self) -> TileBlockWrite<'a2> {
        TileBlockWrite {
            chunk: self.chunk,
            lti: self.lti,
        }
    }

    /// Convert from a writer to a reader.
    pub fn read(self) -> TileBlockRead<'a> {
        TileBlockRead {
            chunk: self.chunk,
            lti: self.lti,
        }
    }

    /// Set the block ID and metadata at this tile, by `RawBlockId`.
    ///
    /// Panics if `M` is not the meta type for `bid`.
    pub fn raw_set<M>(&mut self, bid: RawBlockId, meta: M)
    where
        M: 'static,
    {
        self.chunk.raw_set(self.lti, bid, meta);
    }

    /// Set the block ID and metadata at this tile.
    ///
    /// Panics if `M` is not the meta type for `bid` (unlikely to occur
    /// accidentally).
    pub fn set<M>(&mut self, bid: BlockId<M>, meta: M)
    where
        M: 'static,
    {
        self.chunk.set(self.lti, bid, meta);
    }

    /// Get the block metadata, mutably, without checking block ID.
    ///
    /// Panics if `M` is not the meta type for the block at this tile.
    pub fn raw_meta<M>(self) -> &'a mut M
    where
        M: 'static,
    {
        self.chunk.raw_meta_mut::<M>(self.lti)
    }

    /// Get the block metadata, mutably.
    ///
    /// Panics if:
    /// - `bid` is not the block ID at this tile.
    /// - `M` is not the meta type for `bid` (unlikely to occur accidentally).
    pub fn meta<M>(self, bid: BlockId<M>) -> &'a mut M
    where
        M: 'static,
    {
        self.chunk.meta_mut(bid, self.lti)
    }

    /// If the block at this tile is `bid`, get its metadata, mutably.
    ///
    /// If `bid` is the block at this tile, panics if `M` is not the meta type
    /// for `bid` (unlikely to occur accidentally).
    pub fn try_meta<M>(self, bid: BlockId<M>) -> Option<&'a mut M>
    where
        M: 'static,
    {
        self.chunk.try_meta_mut(bid, self.lti)
    }
}
