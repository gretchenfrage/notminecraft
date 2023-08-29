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
    InPlace {
        drop_in_place: Option<unsafe fn(*mut u8)>,   
    },
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

impl MetaLayout {
    fn new<M: 'static>() -> Self {
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
            MetaLayout::OutOfPlace {
                ..
            } => f.write_str("MetaLayout::OutOfPlace { .. }"),
        }
    }
}

/// A single type-erased self-contained block metadata.
///
/// Because of this being fully self-contained, each instance of this struct
/// consumes a lot more memory than each equivalent tile within a
/// `ChunkBlocks`.
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
    in_place: bool,
    meta: M,
) -> MaybeUninit<usize>
{
    let mut packed = MaybeUninit::uninit();
    if in_place {
        *(packed.as_mut_ptr() as *mut M) = meta;
    } else {
        packed.write(Box::into_raw(Box::new(meta)) as usize);
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
            pack_metadata_value(
                match layout {
                    MetaLayout::InPlace { .. } => true,
                    MetaLayout::OutOfPlace { .. } => false,
                },
                meta,
            )
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
                MetaLayout::InPlace { .. } => {
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
                MetaLayout::InPlace { .. } => {
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
                MetaLayout::InPlace { .. } => {
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
    ptr: *mut u8,
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

    /// Get the block metadata at the given tile, without checking block ID.
    ///
    /// Panics if `M` is not the meta type for the block at that tile.
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

    /// Get the block metadata at the given tile, mutably, without checking
    /// block ID.
    ///
    /// Panics if `M` is not the meta type for the block at that tile.
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

    // assert that setting a tile to the given block with the given
    // metadata type is valid. return whether it's stored in-place.
    fn pre_set(
        &self,
        bid: RawBlockId,
        tid: TypeId,
        type_name: &'static str,
    ) -> bool {
        let registered = &self.registry.items[bid.0 as usize];
        assert!(
            registered.meta_type_id == tid,
            "meta type id mismatch, given={}, requires={}",
            type_name,
            registered.meta_type_name,
        );
        matches!(
            registered.meta_layout,
            MetaLayout::InPlace { .. },
        )
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
        in_place: bool,
        meta: MaybeUninit<usize>,
    ) {
        self.bids[lti] = bid;
        if in_place {
            // in-place
            ptr::copy(
                meta.as_ptr() as *const u16,
                self.in_place[lti].as_mut_ptr(),
                1,
            );
        } else {
            // out of place
            let ptr = meta.assume_init_read() as *mut u8;
            let vec_idx = self.out_of_place.len() as u16;
            self.out_of_place.push(OutOfPlaceElem { ptr, lti });
            self.in_place[lti].write(vec_idx);
        }
    }

    // parameters must be valid as characterized by `pre_set` and `inner_write`.
    unsafe fn inner_set(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        in_place: bool,
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
        self.inner_write(lti, bid, in_place, meta);

        // resume unwinding if destructor panicked
        if let Some(panic) = panic {
            resume_unwind(panic);
        }
    }

    pub fn erased_set(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        meta: ErasedBlockMeta,
    ) {
        let in_place = self.pre_set(bid, meta.type_id, meta.type_name);

        unsafe {
            self.inner_set(lti, bid, in_place, meta.data);
        }
    }

    /// Set the block ID and metadata at the given tile, by `RawBlockId`.
    ///
    /// Panics if `M` is not the meta type for `bid`.
    pub fn raw_set<M>(&mut self, lti: u16, bid: RawBlockId, meta: M)
    where
        M: 'static,
    {
        let in_place = self.pre_set(
            bid,
            TypeId::of::<M>(),
            type_name::<M>(),
        );

        unsafe {
            let value = pack_metadata_value(in_place, meta);
            self.inner_set(lti, bid, in_place, value);
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
        in_place: bool,
        meta: MaybeUninit<usize>,
    ) -> (RawBlockId, ErasedBlockMeta) {
        // take ownership of the existing metadata at that tile
        let pre_bid = self.bids[lti];
        let registered = &self.registry.items[bid.0 as usize];
        let mut pre_meta_data = MaybeUninit::uninit();
        match registered.meta_layout {
            MetaLayout::InPlace { .. } => {
                ptr::copy(
                    self.in_place[lti].as_ptr(),
                    pre_meta_data.as_mut_ptr() as *mut u16,
                    1,
                );
            }
            MetaLayout::OutOfPlace { .. } => {
                let vec_idx = self.in_place[lti].assume_init_read();
                let ptr = self.out_of_place
                    .get(vec_idx as usize)
                    .unwrap_unchecked()
                    .ptr;
                pre_meta_data.write(ptr as usize);
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
        self.inner_write(lti, bid, in_place, meta);

        // done
        (pre_bid, pre_meta)
    }

    pub fn erased_replace(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        meta: ErasedBlockMeta,
    ) -> (RawBlockId, ErasedBlockMeta)
    {
        let in_place = self.pre_set(bid, meta.type_id, meta.type_name);

        unsafe {
            self.inner_replace(lti, bid, in_place, meta.data)
        }
    }

    pub fn raw_replace<M>(
        &mut self,
        lti: u16,
        bid: RawBlockId,
        meta: M,
    ) -> (RawBlockId, ErasedBlockMeta)
    where
        M: 'static,
    {
        let in_place = self.pre_set(
            bid,
            TypeId::of::<M>(),
            type_name::<M>(),
        );

        unsafe {
            let value = pack_metadata_value(in_place, meta);
            self.inner_replace(lti, bid, in_place, value)
        }
    }

    pub fn replace<M>(
        &mut self,
        lti: u16,
        bid: BlockId<M>,
        meta: M,
    ) -> (RawBlockId, ErasedBlockMeta)
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
