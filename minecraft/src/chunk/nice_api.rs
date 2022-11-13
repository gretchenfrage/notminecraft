
use crate::chunk::{
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

    pub fn set<T>(
        self,
        val: <<T as CiGet>::Output as LtiSet>::Input,
        per_chunk: T,
    )
    where
        T: CiGet,
        <T as CiGet>::Output: LtiSet,
    {
        per_chunk.get(self.cc, self.ci).set(self.lti, val);
    }
}


pub trait CiGet {
    type Output;

    fn get(self, cc: Vec3<i64>, ci: usize) -> Self::Output;
}

pub trait LtiGet {
    type Output;

    fn get(self, lti: u16) -> Self::Output;
}

pub trait LtiSet {
    type Input;

    fn set(self, lti: u16, val: Self::Input);
}

impl<'a, T: 'a, C: LtiGet<Output=&'a mut T>> LtiSet for C {
    type Input = T;

    fn set(self, lti: u16, val: Self::Input) {
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
