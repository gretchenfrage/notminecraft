
pub mod erased;
pub mod slots;


use std::{
    marker::PhantomData,
    fmt::{self, Formatter, Debug},
    num::NonZeroU16,
};

pub use self::erased::ItemMeta;


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawItemId(pub u16);

impl<M> From<ItemId<M>> for RawItemId {
    fn from(iid: ItemId<M>) -> Self {
        iid.iid
    }
}

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
}

#[derive(Debug)]
pub struct ItemInstance {
    pub iid: RawItemId,
    pub meta: ItemMeta,
}

impl ItemInstance {
    pub fn new<M>(iid: ItemId<M>, meta: M) -> Self
    where
        M: Debug + Send + Sync + 'static,
    {
        ItemInstance {
            iid: iid.iid,
            meta: ItemMeta::new(meta),
        }
    }

    pub fn meta<M: 'static>(&self, iid: ItemId<M>) -> &M {
        assert_eq!(self.iid, iid);
        self.meta.cast()
    }

    pub fn meta_mut<M: 'static>(&mut self, iid: ItemId<M>) -> &mut M {
        assert_eq!(self.iid, iid);
        self.meta.cast_mut()
    }

    pub fn try_meta<M: 'static>(&self, iid: ItemId<M>) -> Option<&M> {
        if self.iid == iid {
            Some(self.meta.cast())
        } else {
            None
        }
    }

    pub fn try_meta_mut<M: 'static>(&mut self, iid: ItemId<M>) -> Option<&mut M> {
        if self.iid == iid {
            Some(self.meta.cast_mut())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ItemStack {
    pub item: ItemInstance,
    pub count: NonZeroU16,
}

impl ItemStack {
    pub fn one(item: ItemInstance) -> Self {
        ItemStack {
            item,
            count: 1.try_into().unwrap(),
        }
    }
}
