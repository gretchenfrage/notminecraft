//! Types and data structures for handling items. A bit more relaxed tha
//! blocks.
//!
//! Similarly to blocks, there is a registered range of items, each with
//! an item ID and an associated metadata type. Items are typically stored
//! as item stacks, which includes the item ID and type-erased metadata
//! instance, as well as other components such as count and durability.

pub mod erased;


use std::{
    marker::PhantomData,
    fmt::{self, Formatter, Debug},
    num::NonZeroU8,
};

pub use self::erased::ItemMeta;


/// Raw item ID, analogous to raw block ID.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawItemId(pub u16);

impl<M> From<ItemId<M>> for RawItemId {
    fn from(iid: ItemId<M>) -> Self {
        iid.iid
    }
}


/// Item ID with phantom parameter for metadata type, analogous to block ID.
pub struct ItemId<M> {
    pub iid: RawItemId,
    _p: PhantomData<M>,
}

impl<M> ItemId<M> {
    pub const fn new(iid: RawItemId) -> Self {
        ItemId {
            iid,
            _p: PhantomData,
        }
    }
}

impl<M> Copy for ItemId<M> {}

impl<M> Clone for ItemId<M> {
    fn clone(&self) -> Self {
        ItemId {
            iid: self.iid,
            _p: PhantomData,
        }
    }
}

impl<M> Debug for ItemId<M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_tuple("ItemId")
            .field(&self.iid.0)
            .finish()
    }
}

impl<M> PartialEq<RawItemId> for ItemId<M> {
    fn eq(&self, rhs: &RawItemId) -> bool {
        self.iid == *rhs
    }
}

impl<M> PartialEq<ItemId<M>> for RawItemId {
    fn eq(&self, rhs: &ItemId<M>) -> bool {
        *self == rhs.iid
    }
}


/// Assigns item IDs, analogously to block registry.
#[derive(Debug, Clone)]
pub struct ItemRegistry {
    next: usize,
}

impl ItemRegistry {
    pub fn new() -> Self {
        ItemRegistry { next: 0 }
    }

    pub fn register<M>(&mut self) -> ItemId<M> {
        ItemId::new(self.register_raw())
    }

    pub fn register_raw(&mut self) -> RawItemId {
        assert!(self.next <= u16::MAX as usize, "too many items");
        let iid = self.next as u16;
        self.next += 1;
        RawItemId(iid)
    }

    pub fn iter(&self) -> impl Iterator<Item=RawItemId> {
        (0..self.next).map(|n| RawItemId(n as u16))
    }
}


/// Self-contained structure for a stack of items, kinda analogous to a tile
/// block.
#[derive(Debug, Clone)]
pub struct ItemStack {
    /// Item ID.
    pub iid: RawItemId,
    /// Item metadata.
    pub meta: ItemMeta,
    /// Item count in this stack. Should be kept between 1 and the item's max
    /// stack count.
    pub count: NonZeroU8,
    /// Damage level of item. Should be kept between 0 and the item's max
    /// damage level.
    pub damage: u16,
}

impl ItemStack {
    /// Construct with a count of 1 and a damage of 0.
    pub fn new<M>(iid: ItemId<M>, meta: M) -> Self
    where
        M: Debug + Clone + PartialEq + Send + Sync + 'static,
    {
        ItemStack {
            iid: iid.iid,
            meta: ItemMeta::new(meta),
            count: 1.try_into().unwrap(),
            damage: 0,
        }
    }

    /// Cast metadata.
    pub fn meta<M: 'static>(&self, iid: ItemId<M>) -> &M {
        assert_eq!(self.iid, iid);
        self.meta.cast()
    }

    /// Cast metadata mutably.
    pub fn meta_mut<M: 'static>(&mut self, iid: ItemId<M>) -> &mut M {
        assert_eq!(self.iid, iid);
        self.meta.cast_mut()
    }
}

pub type ItemSlot = Option<ItemStack>;
