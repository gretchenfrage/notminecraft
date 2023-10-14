
use crate::item::*;
use std::{
    cell::{
        self,
        RefCell,
    },
    rc::Rc,
};


pub trait BorrowItemSlot {
    type Guard<'a>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a>;

    fn deref<'g, 'a>(guard: &'g mut Self::Guard<'a>) -> &'g mut ItemSlot;
}

impl<'b> BorrowItemSlot for &'b mut ItemSlot {
    type Guard<'a> = &'a mut ItemSlot
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        &mut **self
    }

    fn deref<'g, 'a>(guard: &'g mut &'a mut ItemSlot) -> &'g mut ItemSlot {
        &mut **guard
    }
}

impl<'b> BorrowItemSlot for &'b RefCell<ItemSlot> {
    type Guard<'a> = cell::RefMut<'a, ItemSlot>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        RefCell::borrow_mut(&**self)
    }

    fn deref<'g, 'a>(guard: &'g mut cell::RefMut<'a, ItemSlot>) -> &'g mut ItemSlot {
        &mut **guard
    }
}

impl<'b> BorrowItemSlot for Rc<RefCell<&'b mut ItemSlot>> {
    type Guard<'a> = cell::RefMut<'a, &'b mut ItemSlot>
    where
        Self: 'a;

    fn borrow<'a>(&'a mut self) -> Self::Guard<'a> {
        RefCell::borrow_mut(&**self)
    }

    fn deref<'g, 'a>(guard: &'g mut cell::RefMut<'a, &'b mut ItemSlot>) -> &'g mut ItemSlot {
        &mut ***guard
    }
}
