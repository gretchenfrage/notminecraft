//! Safe memory-optimized system for storing block and block metadata for each
//! tile in a chunk.


use super::{
    per_tile::PerTile,
    coord::{
        NUM_LTIS,
        MAX_LTI,
    },
};
use std::{
    sync::Arc,
    any::{
        TypeId,
        type_name,
    },
    marker::PhantomData,
    mem::{
        size_of,
        needs_drop,
    },
    ptr::drop_in_place,
    mem::MaybeUninit,
    iter::from_fn,
    fmt::{
        self,
        Debug,
        Formatter,
    },
    panic::{
        catch_unwind,
        resume_unwind,
        AssertUnwindSafe,
    },
};


pub const AIR: BlockId<()> = BlockId::new(RawBlockId(0));


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawBlockId(pub u16);


pub struct BlockId<M> {
    pub bid: RawBlockId,
    _p: PhantomData<M>,
}

impl<M> BlockId<M> {
    pub const fn new(bid: RawBlockId) -> Self {
        BlockId {
            bid,
            _p: PhantomData,
        }
    }
}

impl<M> Copy for BlockId<M> {}

impl<M> Clone for BlockId<M> {
    fn clone(&self) -> Self {
        BlockId {
            bid: self.bid,
            _p: PhantomData,
        }   
    }
}

impl<M> Debug for BlockId<M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_tuple("BlockId")
            .field(&self.bid.0)
            .finish()
    }
}

impl<M> PartialEq<RawBlockId> for BlockId<M> {
    fn eq(&self, rhs: &RawBlockId) -> bool {
        self.bid == *rhs
    }
}

impl<M> PartialEq<BlockId<M>> for RawBlockId {
    fn eq(&self, rhs: &BlockId<M>) -> bool {
        *self == rhs.bid
    }
}


#[derive(Debug, Clone)]
pub struct BlockRegistry {
    items: Vec<RegisteredBlock>,
}

#[derive(Copy, Clone)]
struct RegisteredBlock {
    meta_type_id: TypeId,
    meta_type_name: &'static str,
    meta_layout: MetaLayout,
    debug_fmt_meta: unsafe fn(*const u8, &mut Formatter) -> fmt::Result,
}

#[derive(Copy, Clone)]
enum MetaLayout {
    InPlace {
        drop_in_place: Option<unsafe fn(*mut u8)>,   
    },
    OutOfPlace {
        drop_free: unsafe fn(*mut u8),
    },
}

impl Debug for RegisteredBlock {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_struct("RegisteredBlock")
            .field("meta_type_name", &self.meta_type_name)
            .field("meta_layout", &self.meta_layout)
            .finish_non_exhaustive()
    }
}

impl Debug for MetaLayout {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MetaLayout::InPlace {
                ..
            } => f.write_str("MetaLayout::InPlace { .. }"),
            MetaLayout::OutOfPlace {
                ..
            } => f.write_str("MetaLayout::OutOfPlace { .. }"),
        }
    }
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut blocks = BlockRegistry {
            items: Vec::new(),
        };
        let air = blocks.register::<()>();
        debug_assert!(air.bid == AIR.bid);
        blocks
    }

    pub fn register<M>(&mut self) -> BlockId<M>
    where
        M: Debug + Send + Sync + 'static,
    {
        assert!(self.items.len() <= u16::MAX as usize, "too many blocks");
        let bid = self.items.len() as u16;

        unsafe fn cast_drop_in_place<M>(ptr: *mut u8) {
            drop_in_place(ptr as *mut M);
        }

        unsafe fn cast_drop_free<M>(ptr: *mut u8) {
            drop(Box::from_raw(ptr as *mut M))
        }

        unsafe fn cast_debug_fmt_meta<M: Debug>(
            ptr: *const u8,
            f: &mut Formatter,
        ) -> fmt::Result {
            Debug::fmt(
                &*(ptr as *const M),
                f,
            )
        }

        let meta_layout =
            if size_of::<M>() <= 2 {
                let drop_in_place =
                    if needs_drop::<M>() {
                        Some(cast_drop_in_place::<M> as unsafe fn(*mut u8))
                    } else { None };
                MetaLayout::InPlace { drop_in_place }
            } else {
                MetaLayout::OutOfPlace {
                    drop_free: cast_drop_free::<M>,
                }
            };

        self.items.push(RegisteredBlock {
            meta_type_id: TypeId::of::<M>(),
            meta_type_name: type_name::<M>(),
            meta_layout,
            debug_fmt_meta: cast_debug_fmt_meta::<M>,
        });

        BlockId::new(RawBlockId(bid))
    }

    pub fn freeze(self) -> Arc<Self> {
        Arc::new(self)
    }
}


pub struct ChunkBlocks {
    registry: Arc<BlockRegistry>,

    bids: PerTile<RawBlockId>,
    in_place: PerTile<MaybeUninit<u16>>,
    out_of_place: Vec<OutOfPlaceElem>,
}

#[derive(Copy, Clone)]
struct OutOfPlaceElem {
    ptr: *mut u8,
    lti: u16,
}

impl ChunkBlocks {
    pub fn new(registry: &Arc<BlockRegistry>) -> Self {
        let registry = Arc::clone(&registry);

        let bids = PerTile::repeat(AIR.bid);
        let in_place = from_fn(
            || unsafe {
                let mut m = MaybeUninit::uninit();
                *(m.as_mut_ptr() as *mut ()) = (); // lol
                Some(m)
            })
            .take(NUM_LTIS)
            .collect();
        let out_of_place = Vec::new();

        ChunkBlocks {
            registry,

            bids,
            in_place,
            out_of_place,
        }
    }

    pub fn get(&self, lti: u16) -> RawBlockId {
        self.bids[lti]
    }

    /// Validate and return whether in-place.
    fn meta_pre_get<M>(&self, lti: u16) -> bool
    where
        M: 'static,
    {
        let bid = self.bids[lti];
        let registered = &self.registry.items[bid.0 as usize];
        assert!(
            registered.meta_type_id == TypeId::of::<M>(),
            "meta type id mismatch, requested={}, present={}",
            type_name::<M>(),
            registered.meta_type_name,
        );
        matches!(registered.meta_layout, MetaLayout::InPlace { .. })
    }

    pub fn raw_meta<M>(&self, lti: u16) -> &M
    where
        M: 'static,
    {
        unsafe {
            let ptr =
                if self.meta_pre_get::<M>(lti) {
                    self.in_place[lti].as_ptr() as *const M
                } else {
                    let vec_idx = self.in_place[lti].assume_init_read() as usize;
                    self.out_of_place
                        .get(vec_idx)
                        .unwrap_unchecked()
                        .ptr
                        as *const M
                };
            &*ptr
        }
    }

    pub fn raw_meta_mut<M>(&mut self, lti: u16) -> &mut M
    where
        M: 'static,
    {
        unsafe {
            let ptr =
                if self.meta_pre_get::<M>(lti) {
                    self.in_place[lti].as_mut_ptr() as *mut M
                } else {
                    let vec_idx = self.in_place[lti].assume_init_read() as usize;
                    self.out_of_place
                        .get(vec_idx)
                        .unwrap_unchecked()
                        .ptr
                        as *mut M
                };
            &mut *ptr
        }
    }

    pub fn meta<M>(&self, bid: BlockId<M>, lti: u16) -> &M
    where
        M: 'static,
    {
        assert_eq!(self.get(lti), bid);
        self.raw_meta(lti)
    }

    pub fn meta_mut<M>(&mut self, bid: BlockId<M>, lti: u16) -> &mut M
    where
        M: 'static,
    {
        assert_eq!(self.get(lti), bid);
        self.raw_meta_mut(lti)
    }

    pub fn try_meta<M>(&self, bid: BlockId<M>, lti: u16) -> Option<&M>
    where
        M: 'static,
    {
        if self.get(lti) == bid {
            Some(self.raw_meta(lti))
        } else {
            None
        }
    }

    pub fn try_meta_mut<M>(
        &mut self,
        bid: BlockId<M>,
        lti: u16,
    ) -> Option<&mut M>
    where
        M: 'static,
    {
        if self.get(lti) == bid {
            Some(self.raw_meta_mut(lti))
        } else {
            None
        }
    }

    /// Drop, deallocate, and remove, if applicable, whatever metadata
    /// currently exists at the given lti. After this runs, the in_place value
    /// at that tile should be considered no longer validly initialized.
    ///
    /// May panic if meta type's destructor panics, but will still otherwise
    /// clean up properly.
    unsafe fn drop_existing_meta<const VEC_REMOVE: bool>(&mut self, lti: u16) {
        let bid = self.bids[lti];
        let registered = &self.registry.items[bid.0 as usize];
        match registered.meta_layout {
            MetaLayout::InPlace { drop_in_place } => {
                // in-place

                // run destructor if one exists
                //
                // if it panics here, simply unwinding immediately is the
                // correct behavior.
                if let Some(drop_in_place) = drop_in_place {
                    drop_in_place(self.in_place[lti].as_mut_ptr() as *mut u8);
                }
            }
            MetaLayout::OutOfPlace { drop_free } => {
                // out of place

                // run destructor and deallocate
                let vec_idx = self.in_place[lti].assume_init_read();
                let ptr = self.out_of_place
                    .get(vec_idx as usize)
                    .unwrap_unchecked()
                    .ptr;

                // if destructor panics, still remove from vector before
                // unwinding
                let panic = catch_unwind(AssertUnwindSafe(|| {
                    drop_free(ptr);
                }))
                    .err();

                // swap-remove from vector and update accordingly
                if VEC_REMOVE {
                    if vec_idx as usize + 1 == self.out_of_place.len() {
                        // trivial case
                        self.out_of_place
                            .pop()
                            .unwrap_unchecked();
                    } else {
                        // actual swap-remove case
                        let replace_with = self.out_of_place
                            .pop()
                            .unwrap_unchecked();
                        *self.out_of_place
                            .get_mut(vec_idx as usize)
                            .unwrap_unchecked() = replace_with;

                        // this part is very important:
                        self.in_place[replace_with.lti].write(vec_idx);
                    }
                }

                // resume unwinding if destructor panicked
                if let Some(panic) = panic {
                    resume_unwind(panic);
                }
            }
        }
    }

    pub fn raw_set<M>(&mut self, lti: u16, bid: RawBlockId, meta: M)
    where
        M: 'static,
    {
        // validate
        let registered = &self.registry.items[bid.0 as usize];
        assert!(
            registered.meta_type_id == TypeId::of::<M>(),
            "meta type id mismatch, given={}, requires={}",
            type_name::<M>(),
            registered.meta_type_name,
        );
        let in_place = matches!(
            registered.meta_layout,
            MetaLayout::InPlace { .. },
        );

        unsafe {
            // clean up the existing metadata at that tile
            //
            // this will panic if they metadata's destructor panics in which case
            // we still need to write the new data before unwinding, or we'll leave
            // this data structure in an not fully initialized state
            let panic = catch_unwind(AssertUnwindSafe(|| {
                self.drop_existing_meta::<true>(lti);
            }))
                .err();

            // write new data
            self.bids[lti] = bid;
            if in_place {
                // in-place
                *(self.in_place[lti].as_mut_ptr() as *mut M) = meta;
            } else {
                // out of place
                let ptr = Box::into_raw(Box::new(meta)) as *mut u8;
                let vec_idx = self.out_of_place.len() as u16;
                self.out_of_place.push(OutOfPlaceElem { ptr, lti });
                self.in_place[lti].write(vec_idx);
            }

            // resume unwinding if destructor panicked
            if let Some(panic) = panic {
                resume_unwind(panic);
            }
        }
    }

    pub fn set<M>(&mut self, lti: u16, bid: BlockId<M>, meta: M)
    where
        M: 'static,
    {
        self.raw_set(lti, bid.bid, meta);
    }

    // replace could be implemented, but seems unnecessary

    pub fn meta_debug<'s>(&'s self, lti: u16) -> impl Debug + 's {
        unsafe {
            let bid = self.bids[lti];
            let registered = &self.registry.items[bid.0 as usize];
            let ptr = match registered.meta_layout {
                MetaLayout::InPlace { .. } => {
                    // in-place
                    self.in_place[lti].as_ptr() as *mut u8
                }
                MetaLayout::OutOfPlace { .. } => {
                    // out of place
                    let vec_idx = self.in_place[lti].assume_init_read();
                    self.out_of_place
                        .get(vec_idx as usize)
                        .unwrap_unchecked()
                        .ptr
                }
            };
            MetaDebug {
                ptr,
                f: registered.debug_fmt_meta,
                _p: PhantomData,
            }
        }
    }
}

struct MetaDebug<'a> {
    ptr: *mut u8,
    f: unsafe fn(*const u8, &mut Formatter) -> fmt::Result,
    _p: PhantomData<&'a ChunkBlocks>,
}

impl<'a> Debug for MetaDebug<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        unsafe {
            (self.f)(self.ptr, f)
        }
    }
} 

impl Debug for ChunkBlocks {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_list()
            .entries((0..=MAX_LTI).map(|lti| self.meta_debug(lti)))
            .finish()
    }
}

impl Drop for ChunkBlocks {
    fn drop(&mut self) {
        unsafe {
            for lti in 0..=MAX_LTI {
                self.drop_existing_meta::<false>(lti);
            }
        }
    }
}
