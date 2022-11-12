
use crate::chunk::{
    CiGet,
    LtiGet,
    LtiSet,
    block::{
        ChunkBlocks,
        RawBlockId,
        BlockId,
    },
    coord::{
        gtc_get_cc,
        gtc_get_lti,
    },
    per_tile_sparse::PerTileSparse,
};
use std::{
    fmt::Debug,
    collections::hash_map::{
        self as hmap,
        HashMap,
    },
    cell::Cell,
    convert::identity,
};
use slab::Slab;
use vek::*;


const NULL_IDX: u32 = !0;

const NUM_NEIGHBORS: usize = 26;

const NIDX_TO_DIFF: [[i64; 3]; NUM_NEIGHBORS] = [
    [ 1, 0, 0], //  0
    [-1, 0, 0], //  1
    [ 0, 1, 0], //  2
    [ 0,-1, 0], //  3
    [ 0, 0, 1], //  4
    [ 0, 0,-1], //  5

    [ 0, 1, 1], //  6
    [ 0,-1,-1], //  7
    [ 1, 0, 1], //  8
    [-1, 0,-1], //  9
    [ 1, 1, 0], // 10
    [-1,-1, 0], // 11
    [ 0,-1, 1], // 12
    [ 0, 1,-1], // 13
    [-1, 0, 1], // 14
    [ 1, 0,-1], // 15
    [-1, 1, 0], // 16
    [ 1,-1, 0], // 17

    [ 1, 1, 1], // 18
    [-1,-1,-1], // 19
    [-1, 1, 1], // 20
    [ 1,-1,-1], // 21
    [ 1,-1, 1], // 22
    [-1, 1,-1], // 23
    [ 1, 1,-1], // 24
    [-1,-1, 1], // 25
];

const DIFF_TO_NIDX: [[[usize; 3]; 3]; 3] =
    [
        [
            [19,11,25],
            [ 9, 1,14],
            [23,16,20],
        ],
        [
            [ 7, 3,12],
            [ 5,!0, 4],
            [13, 2, 6],
        ],
        [
            [21,17,22],
            [15, 0, 8],
            [24,10,18],
        ],
    ];

const NIDX_REV: [usize; NUM_NEIGHBORS] = 
    [
        1,
        0,
        3,
        2,
        5,
        4,
        7,
        6,
        9,
        8,
        11,
        10,
        13,
        12,
        15,
        14,
        17,
        16,
        19,
        18,
        21,
        20,
        23,
        22,
        25,
        24,
    ];


#[derive(Debug, Clone)]
pub struct LoadedChunks {
    hmap: HashMap<Vec3<i64>, u32>,
    slab: Slab<[u32; NUM_NEIGHBORS]>,
}

impl LoadedChunks {
    pub fn new() -> Self {
        LoadedChunks {
            hmap: HashMap::new(),
            slab: Slab::new(),
        }
    }

    pub fn getter(&self) -> Getter {
        Getter {
            chunks: self,
            cache: Default::default(),
        }
    }

    pub fn add(&mut self, cc: Vec3<i64>) -> usize {
        // validate and anticipate idx
        let hmap_entry =
            match self.hmap.entry(cc) {
                hmap::Entry::Vacant(vacant) => vacant,
                hmap::Entry::Occupied(_) => panic!("chunk already loaded"),
            };

        assert!(
            self.slab.vacant_key() < NULL_IDX as usize,
            "too many loaded chunks",
        );
        let idx = self.slab.vacant_key() as u32;

        // insert idx into hmap
        hmap_entry.insert(idx);

        // link neighbors both ways
        let mut neighbors = [NULL_IDX; NUM_NEIGHBORS];
        for nidx in 0..NUM_NEIGHBORS {
            let diff = Vec3::from(NIDX_TO_DIFF[nidx]);

            if let Some(idx2) = self.hmap.get(&(cc + diff)).copied() {
                let nidx2 = NIDX_REV[nidx];

                // link both ways
                neighbors[nidx] = idx2;
                self.slab[idx2 as usize][nidx2] = idx;
            }
        }

        // insert neighbors into slab
        self.slab.insert(neighbors);

        // done
        idx as usize
    }

    pub fn remove(&mut self, cc: Vec3<i64>) -> usize {
        // remove idx from hmap
        let idx = self.hmap
            .remove(&cc)
            .expect("chunk not loaded");
        
        // remove neighbors from slab
        let neighbors = self.slab.remove(idx as usize);

        // nullify neighbors' pointers to self
        for nidx in 0..NUM_NEIGHBORS {
            let idx2 = neighbors[nidx];
            if idx2 != NULL_IDX {
                let nidx2 = NIDX_REV[nidx];
                self.slab[idx2 as usize][nidx2] = NULL_IDX;
            }
        }

        // done
        idx as usize
    }

    pub fn iter<'c>(&'c self) -> impl Iterator<Item=(Vec3<i64>, usize)> + 'c {
        self.hmap
            .iter()
            .map(|(&cc, &idx)| (
                cc,
                idx as usize,
            ))
    }

    pub fn iter_with_getters<'c>(
        &'c self,
    ) -> impl Iterator<Item=(Vec3<i64>, usize, Getter<'c>)> + 'c
    {
        self.hmap
            .iter()
            .map(|(&cc, &idx)| (
                cc,
                idx as usize,
                Getter {
                    chunks: self,
                    cache: Cell::new(Some((
                        cc,
                        idx,
                    )))
                }
            ))
    }
}


#[derive(Debug, Clone)]
pub struct Getter<'a> {
    chunks: &'a LoadedChunks,
    cache: Cell<Option<(Vec3<i64>, u32)>>,
}

impl<'a> Getter<'a> {
    pub fn get(&self, cc: Vec3<i64>) -> Option<usize> {
        if let Some((cache_cc, cache_idx)) = self.cache.get() {
            // case 1: is cached
            // 
            // just return
            if cache_cc == cc {
                return Some(cache_idx as usize);
            }

            // case 2: neighbor is cached
            //
            // traverse link, cache if Some, return
            let diff = cc - cache_cc;
            if diff.x >= -1
                && diff.y >= -1
                && diff.z >= -1
                && diff.x <= 1
                && diff.y <= 1
                && diff.z <= 1
            {
                let neighbor_idx = DIFF_TO_NIDX
                    [(diff.x + 1) as usize]
                    [(diff.y + 1) as usize]
                    [(diff.z + 1) as usize];

                let idx = self.chunks.slab
                    [cache_idx as usize]
                    [neighbor_idx];

                return
                    if idx == NULL_IDX { None }
                    else {
                        self.cache.set(Some((cc, idx)));
                        Some(idx as usize)
                    };
            }

        }

        // case 3: not cached
        //
        // hashmap lookup, cache if Some, return
        let idx = self.chunks.hmap
            .get(&cc)
            .copied();
        if let Some(idx) = idx {
            self.cache.set(Some((cc, idx)));
            Some(idx as usize)
        } else {
            None
        }
    }

    // convenience

    pub fn gtc_get<T>(
        &self,
        gtc: Vec3<i64>,
        per_chunk: T,
    ) -> Option<<<T as CiGet>::Output as LtiGet>::Output>
    where
        T: CiGet,
        <T as CiGet>::Output: LtiGet,
    {
        self.get(gtc_get_cc(gtc))
            .map(|ci| per_chunk
                .get(ci)
                .get(gtc_get_lti(gtc)))
    }

    pub fn gtc_flat_get<T, I>(
        &self,
        gtc: Vec3<i64>,
        per_chunk: T,
    ) -> Option<I>
    where
        T: CiGet,
        <T as CiGet>::Output: LtiGet<Output = Option<I>>,
    {
        self.gtc_get(gtc, per_chunk)
            .and_then(identity)
    }

    pub fn gtc_set<T>(
        &self,
        gtc: Vec3<i64>,
        val: <<T as CiGet>::Output as LtiSet>::Input,
        per_chunk: T,
    )
    where
        T: CiGet,
        <T as CiGet>::Output: LtiSet,
    {
        let ci = self
            .get(gtc_get_cc(gtc))
            .expect("tile not loaded");
        per_chunk
            .get(ci)
            .set(gtc_get_lti(gtc), val)
    }

    pub fn gtc_try_set<T>(
        &self,
        gtc: Vec3<i64>,
        val: <<T as CiGet>::Output as LtiSet>::Input,
        per_chunk: T,
    ) -> bool
    where
        T: CiGet,
        <T as CiGet>::Output: LtiSet,
    {
        if let Some(ci) = self.get(gtc_get_cc(gtc)) {
            per_chunk
                .get(ci)
                .set(gtc_get_lti(gtc), val);
            true
        } else {
            false
        }
    }
}
