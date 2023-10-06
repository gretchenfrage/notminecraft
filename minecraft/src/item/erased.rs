//! Erased item metadata.
//!
//! Basically a `Box<dyn Any + Debug + Clone + PartialEq + Send + Sync + 'static>` but:
//!
//! - Formatting, cloning, and equality checks actually work.
//! - The inner type `()` is represented as a null pointer.

use std::{
    any::{
        TypeId,
        type_name,
    },
    fmt::{self, Formatter, Debug},
};


unsafe trait MetaTrait {
    fn type_info(&self) -> (TypeId, &'static str);

    fn debug_fmt(&self, f: &mut Formatter) -> fmt::Result;

    fn clone(&self) -> ItemMeta;

    fn eq(&self, other: &dyn MetaTrait) -> bool;
}

unsafe impl<T: Debug + Clone + PartialEq + Send + Sync + 'static> MetaTrait for T {
    fn type_info(&self) -> (TypeId, &'static str) {
        (TypeId::of::<T>(), type_name::<T>())
    }

    fn debug_fmt(&self, f: &mut Formatter) -> fmt::Result {
        <T as Debug>::fmt(self, f)
    }

    fn clone(&self) -> ItemMeta {
        ItemMeta::new(<T as Clone>::clone(self))
    }

    fn eq(&self, other: &dyn MetaTrait) -> bool {
        let (other_tid, other_type_name) = other.type_info();

        if other_tid == TypeId::of::<T>() {
            unsafe {
                <T as PartialEq>::eq(self, &*(other as *const dyn MetaTrait as *const T))
            }
        } else {
            warn!(
                lhs_type_name=%type_name::<T>(),
                rhs_type_name=%other_type_name,
                "equality comparison between item metadata of different types",
            );
            false
        }
    }
}


pub struct ItemMeta(Option<Box<dyn MetaTrait + Send + Sync>>);

impl<M: Debug + Clone + PartialEq + Send + Sync + 'static> From<Box<M>> for ItemMeta {
    fn from(b: Box<M>) -> Self {
        ItemMeta(Some(b as _))
    }
}

impl ItemMeta {
    pub fn new<M>(meta: M) -> Self
    where
        M: Debug + Clone + PartialEq + Send + Sync + 'static,
    {
        if TypeId::of::<M>() == TypeId::of::<()>() {
            ItemMeta(None)
        } else {
            ItemMeta(Some(Box::new(meta) as _))
        }
    }

    fn cast_assert<M: 'static>(&self) {
        if let Some(ref inner) = self.0 {
            let (have_tid, have_name) = inner.type_info();
            assert!(
                have_tid == TypeId::of::<M>(),
                "item meta cast {} to {}",
                have_name,
                type_name::<M>(),
            );
        } else {
            assert!(
                TypeId::of::<M>() == TypeId::of::<()>(),
                "item cast () to {}",
                type_name::<M>(),
            )
        }
    }

    pub fn cast<M: 'static>(&self) -> &M {
        self.cast_assert::<M>();
        unsafe {
            if let Some(ref inner) = self.0 {
                &*(&**inner as *const dyn MetaTrait as *const M)
            } else {
                &*(&() as *const () as *const M)
            }
        }
    }

    pub fn cast_mut<M: 'static>(&mut self) -> &mut M {
        self.cast_assert::<M>();
        unsafe {
            if let Some(ref mut inner) = self.0 {
                &mut *(&mut **inner as *mut dyn MetaTrait as *mut M)
            } else {
                &mut *(&mut () as *mut () as *mut M)
            }
        }
    }

    pub fn cast_into<M: 'static>(self) -> Box<M> {
        self.cast_assert::<M>();
        unsafe {
            if let Some(inner) = self.0 {
                Box::from_raw(Box::into_raw(inner) as *mut dyn MetaTrait as *mut M)
            } else {
                Box::from_raw(Box::into_raw(Box::new(())) as *mut () as *mut M)
            }
        }
    }

    pub fn is<M: 'static>(&self) -> bool {
        if let Some(ref inner) = self.0 {
            let (have_tid, _) = inner.type_info();
            have_tid == TypeId::of::<M>()
        } else {
            TypeId::of::<M>() == TypeId::of::<()>()
        }
    }

    pub fn try_cast<M: 'static>(&self) -> Option<&M> {
        if self.is::<M>() {
            Some(unsafe {
                if let Some(ref inner) = self.0 {
                    &*(&**inner as *const dyn MetaTrait as *const M)
                } else {
                    &*(&() as *const () as *const M)
                }
            })
        } else {
            None
        }
    }

    pub fn try_cast_mut<M: 'static>(&mut self) -> Option<&mut M> {
        if self.is::<M>() {
            Some(unsafe {
                if let Some(ref mut inner) = self.0 {
                    &mut *(&mut **inner as *mut dyn MetaTrait as *mut M)
                } else {
                    &mut *(&mut () as *mut () as *mut M)
                }
            })
        } else {
            None
        }
    }

    pub fn try_cast_into<M: 'static>(self) -> Result<Box<M>, Self> {
        if self.is::<M>() {
            Ok(unsafe {
                if let Some(inner) = self.0 {
                    Box::from_raw(Box::into_raw(inner) as *mut dyn MetaTrait as *mut M)
                } else {
                    Box::from_raw(Box::into_raw(Box::new(())) as *mut () as *mut M)
                }
            })
        } else {
            Err(self)
        }
    }
}

impl Debug for ItemMeta {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(ref inner) = self.0 {
            inner.debug_fmt(f)
        } else {
            f.write_str("()")
        }
    }
}

impl Clone for ItemMeta {
    fn clone(&self) -> Self {
        if let Some(ref inner) = self.0 {
            (**inner).clone()
        } else {
            ItemMeta::new(())
        }
    }
}

impl PartialEq for ItemMeta {
    fn eq(&self, other: &ItemMeta) -> bool {
        match (self.0.as_ref(), other.0.as_ref()) {
            (Some(a), Some(b)) => a.eq(&**b),
            (None, None) => true,
            _ => false,
        }
    }
}
