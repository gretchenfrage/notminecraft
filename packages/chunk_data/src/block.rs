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
        forget,
    },
    ptr::{
        self,
        drop_in_place,
    },
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
    ops::{
        Deref,
        DerefMut,
    },
};


/// The "air" block, which is hard-guaranteed to be registered.
///
/// When `BlockRegistry` is constructed is automatically self-registers this
/// block, with block ID 0 and meta type `()`. Code can and does rely on this
/// for soundness. The reason for this is that it makes `ChunkBlocks`
/// construction nicer and easier.
pub const AIR: BlockId<()> = BlockId::new(RawBlockId(0));

/// Block ID, dissociated from metadata type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawBlockId(pub u16);

impl<M> From<BlockId<M>> for RawBlockId {
    fn from(bid: BlockId<M>) -> Self {
        bid.bid
    }
}

/// Block ID, with phantom type parameter `M` denoting its metadata type.
///
/// Just a wrapper around a `RawBlockId` and a `PhantomData<M>`. Associating
/// the metadata type is just to help avoid accidental logic errors, it is
/// unrelated to soundness guarantees, which are acheived with runtime
/// assertions.
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


/// Registry of block IDs and their meta types.
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
    // stored directly in the u16
    InPlace {
        drop_in_place: Option<unsafe fn(*mut u8)>,
    },
    // the u16 is an index into the array of usize
    // the data is stored directly in the usize
    SemiInPlace {
        drop_in_place: Option<unsafe fn(*mut u8)>,
    },
    // the u16 is an index into the array of usize
    // the usize is a pointer to the heap
    // the data is stored in the heap allocation
    OutOfPlace {
        drop_free: unsafe fn(*mut u8),
    },
}

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

fn drop_in_place_if_needed<M>() -> Option<unsafe fn(*mut u8)> {
    if needs_drop::<M>() {
        Some(cast_drop_in_place::<M> as unsafe fn(*mut u8))
    } else {
        None
    }
}

impl MetaLayout {
    fn new<M: 'static>() -> Self {
        if size_of::<M>() <= 2 {
            MetaLayout::InPlace { drop_in_place: drop_in_place_if_needed::<M>() }
        } else if size_of::<M>() <= size_of::<usize>() {
            MetaLayout::SemiInPlace { drop_in_place: drop_in_place_if_needed::<M>() }
        } else {
            MetaLayout::OutOfPlace {
                drop_free: cast_drop_free::<M>,
            }
        }
    }
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
            MetaLayout::SemiInPlace {
                ..
            } => f.write_str("MetaLayout::SemiInPlace { .. }"),
            MetaLayout::OutOfPlace {
                ..
            } => f.write_str("MetaLayout::OutOfPlace { .. }"),
        }
    }
}

/// Raw block id + erased block meta.
///
/// Nothing special, just a struct. It's implied that they go together.
#[derive(Debug)]
pub struct ErasedBidMeta {
    pub bid: RawBlockId,
    pub meta: ErasedBlockMeta,
}

impl ErasedBidMeta {
    /// Constructor that helps avoids using the wrong metadata type.
    pub fn new<M>(bid: BlockId<M>, meta: M) -> Self
    where
        M: Debug + Send + Sync + 'static,
    {
        ErasedBidMeta {
            bid: bid.bid,
            meta: ErasedBlockMeta::new(meta),
        }
    }
}

/// A single type-erased self-contained block metadata.
///
/// Because of this being fully self-contained, each instance of this struct
/// has more memory overhead than each tile within a `ChunkBlocks`. That said,
/// this does still have `ChunkBlocks`'s behavior of avoiding heap allocations
/// if the meta type is small enough, so it's not super bad--it just might not
/// be optimal to store a big array of these.
pub struct ErasedBlockMeta {
    type_id: TypeId,
    type_name: &'static str,
    layout: MetaLayout,
    debug_fmt: unsafe fn(*const u8, &mut Formatter) -> fmt::Result,
    data: MaybeUninit<usize>,
}

impl Debug for ErasedBlockMeta {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        unsafe {
            // TODO: is this wrong?
            (self.debug_fmt)(self.data.as_ptr() as *const u8, f)
        }
    }
}

impl Drop for ErasedBlockMeta {
    fn drop(&mut self) {
        unsafe {
            match self.layout {
                MetaLayout::InPlace {
                    drop_in_place,
                } => if let Some(drop_in_place) = drop_in_place {
                    drop_in_place(self.data.as_mut_ptr() as *mut u8);
                }
                MetaLayout::SemiInPlace {
                    drop_in_place,
                } => if let Some(drop_in_place) = drop_in_place {
                    drop_in_place(self.data.as_mut_ptr() as *mut u8);
                }
                MetaLayout::OutOfPlace { drop_free } => {
                    drop_free(self.data.assume_init_read() as *mut u8);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MaybeBoxed<T> {
    Boxed(Box<T>),
    NotBoxed(T),
}

impl<T> Deref for MaybeBoxed<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            &MaybeBoxed::Boxed(ref b) => &**b,
            &MaybeBoxed::NotBoxed(ref v) => v,
        }
    }
}

impl<T> DerefMut for MaybeBoxed<T> {
    fn deref_mut(&mut self) -> &mut T {
        match self {
            &mut MaybeBoxed::Boxed(ref mut b) => &mut **b,
            &mut MaybeBoxed::NotBoxed(ref mut v) => v,
        }
    }
}

impl<T> MaybeBoxed<T> {
    pub fn into_box(self) -> Box<T> {
        match self {
            MaybeBoxed::Boxed(b) => b,
            MaybeBoxed::NotBoxed(v) => Box::new(v),
        }
    }

    pub fn into_val(self) -> T {
        match self {
            MaybeBoxed::Boxed(b) => *b,
            MaybeBoxed::NotBoxed(v) => v,
        }
    }
}

impl<T> From<T> for MaybeBoxed<T> {
    fn from(t: T) -> Self {
        MaybeBoxed::NotBoxed(t)
    }
}

impl<T> From<Box<T>> for MaybeBoxed<T> {
    fn from(t: Box<T>) -> Self {
        MaybeBoxed::Boxed(t)
    }
}

unsafe impl Send for ErasedBlockMeta {}
unsafe impl Sync for ErasedBlockMeta {}

unsafe fn pack_metadata_value<M>(
    layout: MetaLayout,
    meta: M,
) -> MaybeUninit<usize>
{
    let mut packed = MaybeUninit::uninit();
    match layout {
        MetaLayout::InPlace { .. } | MetaLayout::SemiInPlace { .. } => {
            *(packed.as_mut_ptr() as *mut M) = meta; // TODO fix and whatnot
        }
        MetaLayout::OutOfPlace { .. } => {
            packed.write(Box::into_raw(Box::new(meta)) as usize);
        }
    }
    packed
}

impl ErasedBlockMeta {
    pub fn new<M>(meta: M) -> Self
    where
        M: Debug + Send + Sync + 'static,
    {
        let layout = MetaLayout::new::<M>();
        let data = unsafe {
            pack_metadata_value(layout, meta)
        };
        ErasedBlockMeta {
            type_id: TypeId::of::<M>(),
            type_name: type_name::<M>(),
            layout,
            debug_fmt: cast_debug_fmt_meta::<M>,
            data,
        }
    }

    fn cast_assert<M: 'static>(&self) {
        assert!(
            TypeId::of::<M>() == self.type_id,
            "erased block meta cast {} to {}",
            self.type_name,
            type_name::<M>(),
        );
    }

    pub fn cast<M: 'static>(&self) -> &M {
        unsafe {
            self.cast_assert::<M>();
            &*match self.layout {
                MetaLayout::InPlace { .. } | MetaLayout::SemiInPlace { .. } => {
                    self.data.as_ptr() as *const M
                }
                MetaLayout::OutOfPlace { .. } => {
                    self.data.assume_init_read() as *const M
                }
            }
        }
    }

    pub fn cast_mut<M: 'static>(&mut self) -> &mut M {
        unsafe {
            self.cast_assert::<M>();
            &mut *match self.layout {
                MetaLayout::InPlace { .. } | MetaLayout::SemiInPlace { .. } => {
                    self.data.as_mut_ptr() as *mut M
                }
                MetaLayout::OutOfPlace { .. } => {
                    self.data.assume_init_read() as *mut M
                }
            }
        }
    }

    pub fn cast_into<M: 'static>(mut self) -> MaybeBoxed<M> {
        let ret = unsafe {
            self.cast_assert::<M>();
            match self.layout {
                MetaLayout::InPlace { .. } | MetaLayout::SemiInPlace { .. } => {
                    MaybeBoxed::NotBoxed(ptr::read(self.data.as_mut_ptr() as *mut M))
                }
                MetaLayout::OutOfPlace { .. } => {
                    MaybeBoxed::Boxed(Box::from_raw(self.data.assume_init_read() as *mut M))
                }
            }
        };
        forget(self);
        ret
    }

    pub fn is<M: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<M>()
    }

    pub fn try_cast<M: 'static>(&self) -> Option<&M> {
        if self.is::<M>() {
            Some(self.cast::<M>())
        } else {
            None
        }
    }

    pub fn try_cast_mut<M: 'static>(&mut self) -> Option<&mut M> {
        if self.is::<M>() {
            Some(self.cast_mut::<M>())
        } else {
            None
        }
    }

    pub fn try_cast_into<M: 'static>(self) -> Result<MaybeBoxed<M>, Self> {
        if self.is::<M>() {
            Ok(self.cast_into::<M>())
        } else {
            Err(self)
        }
    }
}

impl BlockRegistry {
    /// Construct a new block registry with nothing but `AIR` registered.
    pub fn new() -> Self {
        let mut blocks = BlockRegistry {
            items: Vec::new(),
        };
        let air = blocks.register::<()>();
        debug_assert!(air.bid == AIR.bid);
        blocks
    }

    /// Register a new block with the given meta type, and return its assigned
    /// block ID.
    pub fn register<M>(&mut self) -> BlockId<M>
    where
        M: Debug + Send + Sync + 'static,
    {
        assert!(self.items.len() <= u16::MAX as usize, "too many blocks");
        let bid = self.items.len() as u16;

        let meta_layout = MetaLayout::new::<M>();

        self.items.push(RegisteredBlock {
            meta_type_id: TypeId::of::<M>(),
            meta_type_name: type_name::<M>(),
            meta_layout,
            debug_fmt_meta: cast_debug_fmt_meta::<M>,
        });

        BlockId::new(RawBlockId(bid))
    }

    /// Finalize the block registry by wrapping it in an `Arc`.
    ///
    /// This allows all `ChunkBlocks` instances to efficiently share the block
    /// registry and rely on it to not change.
    pub fn finalize(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn iter(&self) -> impl Iterator<Item=RawBlockId> {
        (0..self.items.len()).map(|n| RawBlockId(n as u16))
    }

    pub fn meta_type_id(&self, bid: impl Into<RawBlockId>) -> TypeId {
        self.items[bid.into().0 as usize].meta_type_id
    }
}


/// Optimized storage for block ID and block metadata for every tile in a
/// chunk.
pub struct ChunkBlocks {
    registry: Arc<BlockRegistry>,

    bids: PerTile<RawBlockId>,
    in_place: PerTile<MaybeUninit<u16>>,
    out_of_place: Vec<OutOfPlaceElem>,
}

#[derive(Copy, Clone)]
struct OutOfPlaceElem {
    val: usize,
    lti: u16,
}

unsafe impl Send for OutOfPlaceElem {}
unsafe impl Sync for OutOfPlaceElem {}

impl ChunkBlocks {
    /// Construct a new `ChunkBlocks` filled with `AIR`.
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

    /// Get the block ID at the given tile.
    pub fn get(&self, lti: u16) -> RawBlockId {
        self.bids[lti]
    }

    /// Validate and return layout.
    fn meta_pre_get<M>(&self, lti: u16) -> MetaLayout
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
        registered.meta_layout
    }

    /// Get the block metadata at the given tile, without checking block ID.
    ///
    /// Panics if `M` is not the meta type for the block at that tile.
    pub fn raw_meta<M>(&self, lti: u16) -> &M
    where
        M: 'static,
    {
        unsafe {
            &*match self.meta_pre_get::<M>(lti) {
                MetaLayout::InPlace { .. } => self.in_place[lti].as_ptr() as *const M,
                MetaLayout::SemiInPlace { .. } => self.out_of_place(lti) as *const usize as *const M,
                MetaLayout::OutOfPlace { .. } => *self.out_of_place(lti) as *const M,
            }
        }
    }

    unsafe fn out_of_place(&self, lti: u16) -> &usize {
        let vec_idx = self.in_place[lti].assume_init_read() as usize;
        &self.out_of_place
            .get(vec_idx)
            .unwrap_unchecked()
            .val
    }

    unsafe fn out_of_place_mut(&mut self, lti: u16) -> &mut usize {
        let vec_idx = self.in_place[lti].assume_init_read() as usize;
        &mut self.out_of_place
            .get_mut(vec_idx)
            .unwrap_unchecked()
            .val
    }

    /// Get the block metadata at the given tile, mutably, without checking
    /// block ID.
    ///
    /// Panics if `M` is not the meta type for the block at that tile.
    pub fn raw_meta_mut<M>(&mut self, lti: u16) -> &mut M
    where
        M: 'static,
    {
        unsafe {
            &mut *match self.meta_pre_get::<M>(lti) {
                MetaLayout::InPlace { .. } => self.in_place[lti].as_ptr() as *mut M,
                MetaLayout::SemiInPlace { .. } => self.out_of_place_mut(lti) as *mut usize as *mut M,
                MetaLayout::OutOfPlace { .. } => *self.out_of_place_mut(lti) as *mut M,
            }
        }
    }

    /// Get the block metadata at the given tile.
    ///
    /// Panics if:
    /// - `bid` is not the block ID at that tile.
    /// - `M` is not the meta type for `bid` (unlikely to occur accidentally).
    pub fn meta<M>(&self, bid: BlockId<M>, lti: u16) -> &M
    where
        M: 'static,
    {
        assert_eq!(self.get(lti), bid);
        self.raw_meta(lti)
    }

    /// Get the block metadata at the given tile, mutably.
    ///
    /// Panics if:
    /// - `bid` is not the block at that tile.
    /// - `M` is not the meta type for `bid` (unlikely to occur accidentally).
    pub fn meta_mut<M>(&mut self, bid: BlockId<M>, lti: u16) -> &mut M
    where
        M: 'static,
    {
        assert_eq!(self.get(lti), bid);
        self.raw_meta_mut(lti)
    }

    /// If the block at that tile is `bid`, get its metadata.
    ///
    /// If `bid` is the block at that tile, panics if `M` is not the meta type
    /// for `bid` (unlikely to occur accidentally).
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

    /// If the block at that tile is `bid`, get its metadata, mutably.
    ///
    /// If `bid` is the block at that tile, panics if `M` is not the meta type
    /// for `bid` (unlikely to occur accidentally).
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
            MetaLayout::SemiInPlace { drop_in_place } => {
                // semi-in-place
                self.drop_existing_meta_vec_inner::<VEC_REMOVE, _>(lti, |ptr| unsafe {
                    if let Some(drop_in_place) = drop_in_place {
                        drop_in_place(ptr as *mut usize as *mut u8);
                    }
                });
            }
            MetaLayout::OutOfPlace { drop_free } => {
                // out of place
                self.drop_existing_meta_vec_inner::<VEC_REMOVE, _>(lti, |ptr| unsafe {
                    drop_free(*ptr as *mut u8);
                });
            }
        }
    }

    unsafe fn drop_existing_meta_vec_inner<const VEC_REMOVE: bool, C>(
        &mut self,
        lti: u16,
        cleanup: C,
    )
    where
        C: FnOnce(&mut usize),
    {
        // run destructor and deallocate
        let vec_idx = self.in_place[lti].assume_init_read();
        let ptr = &mut self.out_of_place
            .get_mut(vec_idx as usize)
            .unwrap_unchecked()
            .val;

        // if destructor panics, still remove from vector before
        // unwinding
        let panic = catch_unwind(AssertUnwindSafe(|| {
            cleanup(ptr);
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

    // assert that setting a tile to the given block with the given
    // metadata type is valid. return whether it's stored in-place.
    fn pre_set(
        &self,
        bid: RawBlockId,
        tid: TypeId,
        type_name: &'static str,
    ) -> MetaLayout {
        let registered = &self.registry.items[bid.0 as usize];
        assert!(
            registered.meta_type_id == tid,
            "meta type id mismatch, given={}, requires={}",
            type_name,
            registered.meta_type_name,
        );
        registered.meta_layout
    }

    // write the block ID and metadata at the given tile without reading or
    // dropping the existing block ID and metadata at that tile.
    //
    // if in-place, metadata value must be stored at the beginning bytes of
    // `meta`. if out-of-place, `meta` must be the raw boxed metadata pointer.
    unsafe fn inner_write(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        layout: MetaLayout,
        meta: MaybeUninit<usize>,
    ) {
        self.bids[lti] = bid;
        match layout {
            MetaLayout::InPlace { .. } => {
                // in-place
                ptr::copy(
                    meta.as_ptr() as *const u16,
                    self.in_place[lti].as_mut_ptr(),
                    1,
                );
            }
            MetaLayout::SemiInPlace { .. } | MetaLayout::OutOfPlace { .. } => {
                // out of place
                let val = meta.assume_init_read();
                let vec_idx = self.out_of_place.len() as u16;
                self.out_of_place.push(OutOfPlaceElem { val, lti });
                self.in_place[lti].write(vec_idx);
            }
        }
    }

    // parameters must be valid as characterized by `pre_set` and `inner_write`.
    unsafe fn inner_set(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        layout: MetaLayout,
        meta: MaybeUninit<usize>,
    ) {
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
        self.inner_write(lti, bid, layout, meta);

        // resume unwinding if destructor panicked
        if let Some(panic) = panic {
            resume_unwind(panic);
        }
    }

    /// Set the block ID and metadata at the given tile with type-erased
    /// metadata.
    ///
    /// Panics if `bid_meta`'s metadata is the wrong type for its bid
    /// (unlikely to occur accidentally if constructed with
    /// `ErasedBidMeta::new`).
    pub fn erased_set(
        &mut self,
        lti: u16,
        bid_meta: ErasedBidMeta,
    ) {
        let layout = self.pre_set(bid_meta.bid, bid_meta.meta.type_id, bid_meta.meta.type_name);
        unsafe {
            self.inner_set(lti, bid_meta.bid, layout, bid_meta.meta.data);
        }
        forget(bid_meta.meta);
    }

    /// Set the block ID and metadata at the given tile, by `RawBlockId`.
    ///
    /// Panics if `M` is not the meta type for `bid`.
    pub fn raw_set<M>(&mut self, lti: u16, bid: RawBlockId, meta: M)
    where
        M: 'static,
    {
        let layout = self.pre_set(
            bid,
            TypeId::of::<M>(),
            type_name::<M>(),
        );

        unsafe {
            let value = pack_metadata_value(layout, meta);
            self.inner_set(lti, bid, layout, value);
        }
    }

    /// Set the block ID and metadata at the given tile.
    ///
    /// Panics if `M` is not the meta type for `bid` (unlikely to occur
    /// accidentally).
    pub fn set<M>(&mut self, lti: u16, bid: BlockId<M>, meta: M)
    where
        M: 'static,
    {
        self.raw_set(lti, bid.bid, meta);
    }

    // parameters must be valid as characterized by `pre_set` and `inner_write`
    unsafe fn inner_replace(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        layout: MetaLayout,
        meta: MaybeUninit<usize>,
    ) -> ErasedBidMeta {
        // take ownership of the existing metadata at that tile
        let pre_bid = self.bids[lti];
        let registered = &self.registry.items[pre_bid.0 as usize];
        let mut pre_meta_data = MaybeUninit::uninit();
        match registered.meta_layout {
            MetaLayout::InPlace { .. } => {
                ptr::copy(
                    self.in_place[lti].as_ptr(),
                    pre_meta_data.as_mut_ptr() as *mut u16,
                    1,
                );
            }
            MetaLayout::SemiInPlace { .. } | MetaLayout::OutOfPlace { .. } => {
                let vec_idx = self.in_place[lti].assume_init_read();
                let val = self.out_of_place
                    .get_mut(vec_idx as usize)
                    .unwrap_unchecked()
                    .val;
                pre_meta_data.write(val);
                if vec_idx as usize + 1 == self.out_of_place.len() {
                    self.out_of_place
                        .pop()
                        .unwrap_unchecked();
                } else {
                    let replace_with = self.out_of_place
                        .pop()
                        .unwrap_unchecked();
                    *self.out_of_place
                        .get_mut(vec_idx as usize)
                        .unwrap_unchecked() = replace_with;

                    self.in_place[replace_with.lti].write(vec_idx);
                }
            }
        }
        let pre_meta = ErasedBlockMeta {
            type_id: registered.meta_type_id,
            type_name: registered.meta_type_name,
            layout: registered.meta_layout,
            debug_fmt: registered.debug_fmt_meta,
            data: pre_meta_data,
        };

        // write new data
        self.inner_write(lti, bid, layout, meta);

        // done
        ErasedBidMeta {
            bid: pre_bid,
            meta: pre_meta,
        }
    }

    /// Replace the block id and metadata the given tile, returning the old bid
    /// and metadata, both receiving and returning metadata type-erased.
    ///
    /// Panics if `bid_meta`'s metadata is the wrong type for its bid
    /// (unlikely to occur accidentally if constructed with
    /// `ErasedBidMeta::new`).
    pub fn erased_replace(
        &mut self,
        lti: u16,
        bid_meta: ErasedBidMeta
    ) -> ErasedBidMeta
    {
        let layout = self.pre_set(bid_meta.bid, bid_meta.meta.type_id, bid_meta.meta.type_name);
        let result = unsafe {
            self.inner_replace(lti, bid_meta.bid, layout, bid_meta.meta.data)
        };
        forget(bid_meta.meta);
        result
    }

    /// Replace the block id and metadata the given tile, by `RawBlockId`,
    /// returning the old bid and metadata type erased.
    ///
    /// Panics if `M` is not the meta type for `bid`.
    pub fn raw_replace<M>(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        meta: M,
    ) -> ErasedBidMeta
    where
        M: 'static,
    {
        let layout = self.pre_set(
            bid,
            TypeId::of::<M>(),
            type_name::<M>(),
        );

        unsafe {
            let value = pack_metadata_value(layout, meta);
            self.inner_replace(lti, bid, layout, value)
        }
    }

    /// Replace the block id and metadata the given tile, returning the old bid
    /// and metadata type erased.
    ///
    /// Panics if `M` is not the meta type for `bid` (unlikely to occur
    /// accidentally).
    pub fn replace<M>(
        &mut self,
        lti: u16,
        bid: BlockId<M>,
        meta: M,
    ) -> ErasedBidMeta
    where
        M: 'static,
    {
        self.raw_replace(lti, bid.bid, meta)
    }

    /// Way of debug-formatting the block metadata at the given tile without
    /// knowing its type.
    pub fn meta_debug<'s>(&'s self, lti: u16) -> impl Debug + 's {
        unsafe {
            let bid = self.bids[lti];
            let registered = &self.registry.items[bid.0 as usize];
            let ptr = match registered.meta_layout {
                MetaLayout::InPlace { .. } => self.in_place[lti].as_ptr() as *mut u8,
                MetaLayout::SemiInPlace { .. } => self.out_of_place(lti) as *const usize as *const u8,
                MetaLayout::OutOfPlace { .. } => *self.out_of_place(lti) as *const u8,
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
    ptr: *const u8,
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
        f.write_str("ChunkBlocks(..)")
        /* yes, we /can/ debug-format the whole chunk's block data
           it's a bad idea though
        f
            .debug_list()
            .entries((0..=MAX_LTI).map(|lti| self.meta_debug(lti)))
            .finish()
        */
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
