
use crate::{
    TileKey,
    coord::{
        gtc_get_cc,
        gtc_get_lti,
    },
};
use std::{
    fmt::Debug,
    collections::hash_map::{
        self as hmap,
        HashMap,
    },
    cell::Cell,
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


/// Set of loaded chunks.
///
/// Serves 3 purposes:
///
/// - Tracks the set of chunks which are currently loaded.
/// - Assigns each loaded chunk a chunk index (ci), which may be reused if the
///   chunk is unloaded. Guaranteed to assign indexes with precisely the
///   behavior of a `slab::Slab`.
/// - Provides an efficient lookup from chunk coordinate (cc) to chunk index
///   (ci) based on a 3-dimensionally linked hashmap which is exploited for
///   caching, which makes linear access patterns extremely fast. 
#[derive(Debug, Clone)]
pub struct LoadedChunks {
    hmap: HashMap<Vec3<i64>, u32>,
    slab: Slab<[u32; NUM_NEIGHBORS]>,
}

impl LoadedChunks {
    /// Construct a new empty set of loaded chunks.
    pub fn new() -> Self {
        LoadedChunks {
            hmap: HashMap::new(),
            slab: Slab::new(),
        }
    }

    /// Produce a getter, for lookups, which does caching and link-traversal.
    ///
    /// This means that:
    /// - Sequential accesses of the same chunk are cached.
    /// - Sequential accesses of adjacent chunks (including face, edge, and
    ///   corner adjacency) are done with link traversal rather than a full
    ///   hashmap lookup.
    ///
    /// `Getter` implements `Clone`, which can be exploited to improve
    /// performance if one is performing two interleaved access patterns which
    /// individually exhibit locality but which would switch back and forth if
    /// combined.
    pub fn getter(&self) -> Getter {
        Getter {
            chunks: self,
            cache: Default::default(),
        }
    }

    /// Add a new chunk to the set of loaded chunks, and get its assigned chunk
    /// index (ci).
    ///
    /// Panics if already present.
    ///
    /// This should be followed by a corresponding add operation to all
    /// per-chunk world data, with all the generation or loading logic that may
    /// require.
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

    /// Remove a chunk from the set of loaded chunks. Its chunk index may be
    /// reused for following `add` transactions.
    ///
    /// Panics if not present.
    ///
    /// This should be followed by a corresponding remove operation to all
    /// per-chunk world data, with all the cleanup and saving logic that may
    /// require.
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

    /// Iterate through the cc and ci of all loaded chunks.
    pub fn iter<'c>(&'c self) -> impl Iterator<Item=(Vec3<i64>, usize)> + 'c {
        self.hmap
            .iter()
            .map(|(&cc, &idx)| (
                cc,
                idx as usize,
            ))
    }

    /// Iterate through the cc and ci of all loaded chunks, along with
    /// corresponding `Getter`s which have those chunks pre-cached.
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


/// See `LoadedChunks::getter`.
#[derive(Debug, Clone)]
pub struct Getter<'a> {
    chunks: &'a LoadedChunks,
    cache: Cell<Option<(Vec3<i64>, u32)>>,
}

impl<'a> Getter<'a> {
    /// Perform a cc -> ci lookup.
    pub fn get<V>(&self, cc: V) -> Option<usize>
    where
        V: Into<Vec3<i64>>,
    {
        let cc = cc.into();

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

    /// Given a global tile coordinate (gtc), look up the chunk it's in, and
    /// pack everything into a nice `TileKey`. This is part of the nice
    /// chainable API.
    pub fn gtc_get<V>(&self, gtc: V) -> Option<TileKey>
    where
        V: Into<Vec3<i64>>,
    {
        let gtc = gtc.into();

        let cc = gtc_get_cc(gtc);
        self.get(cc)
            .map(|ci| TileKey {
                cc,
                ci,
                lti: gtc_get_lti(gtc),
            })
    }
}
