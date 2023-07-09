//! Trait for types which statically tell you the schema by which they'll
//! serde, and implementations for common types.


use crate::schema::*;
use std::{
    collections::{
        BinaryHeap,
        BTreeSet,
        HashSet,
        LinkedList,
        VecDeque,
        BTreeMap,
        HashMap,
    },
    ops::{
        Range,
        RangeInclusive,
        Bound,
    },
    borrow::Cow,
    any::type_name,
    fmt::{self, Debug, Formatter},
};


/// Type which know what `Schema` its `serde`s with.
pub trait KnownSchema {
    fn schema(parent_stack: RecurseStack) -> Schema;
}

#[derive(Copy, Clone)]
pub struct RecurseStack<'a>(Option<Node<'a>>);

#[derive(Copy, Clone)]
struct Node<'a> {
    val: Option<fn(RecurseStack) -> Schema>,
    dbg: &'static str,
    next: &'a Option<Node<'a>>,
}

impl<'a> Default for RecurseStack<'a> {
    fn default() -> Self {
        RecurseStack::new()
    }
}

impl<'a> RecurseStack<'a> {
    pub fn new() -> Self {
        RecurseStack(None)
    }

    pub fn with_type_layer<'b, T: KnownSchema + ?Sized>(&'b self) -> RecurseStack<'b> {
        RecurseStack(Some(Node {
            val: Some(T::schema),
            dbg: type_name::<T>(),
            next: &self.0,
        }))
    }

    pub fn with_none_layer<'b>(&'b self) -> RecurseStack<'b> {
        RecurseStack(Some(Node {
            val: None,
            dbg: "",
            next: &self.0,
        }))
    }

    pub fn find_type<T: KnownSchema>(&self) -> Option<usize> {
        let mut opt_curr = &self.0;
        let mut level = 0;

        while let Some(curr) = opt_curr {
            if let Some(curr_val) = curr.val {
                if curr_val as usize == T::schema as usize {
                    return Some(level);
                }
            }
            opt_curr = curr.next;
            level += 1;
        }

        None
    }

    pub fn parent_recurse<T: KnownSchema>(&self) -> Option<Schema> {
        self.find_type::<T>()
            .map(|n| Schema::Recurse(n + 1))
    }
}

impl<'a> Debug for RecurseStack<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        let mut opt_curr = &self.0;
        while let &Some(curr) = opt_curr {
            list.entry(&curr.dbg);
            opt_curr = curr.next;
        }
        list.finish()
    }
}

macro_rules! scalars_known_schema {
    ($($t:tt,)*)=>{$(
        impl KnownSchema for $t {
            fn schema(_: RecurseStack) -> Schema {
                schema!($t)
            }
        }
    )*};
}

scalars_known_schema!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    char,
    bool,
);

impl KnownSchema for usize {
    fn schema(_: RecurseStack) -> Schema {
        schema!(u64)
    }
}

impl KnownSchema for isize {
    fn schema(_: RecurseStack) -> Schema {
        schema!(i64)
    }
}

impl KnownSchema for str {
    fn schema(_: RecurseStack) -> Schema {
        schema!(str)
    }

}

impl KnownSchema for String {
    fn schema(_: RecurseStack) -> Schema {
        schema!(str)
    }
}

impl KnownSchema for () {
    fn schema(_: RecurseStack) -> Schema {
        schema!(unit)
    }
}

impl<T: KnownSchema> KnownSchema for Option<T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(option(%T::schema(stack)))
    }
}

macro_rules! seqs_known_schema {
    ($($c:ident,)*)=>{$(
        impl<T: KnownSchema> KnownSchema for $c<T> {
            fn schema(parent_stack: RecurseStack) -> Schema {
                let stack = parent_stack.with_type_layer::<Self>();
                schema!(seq(varlen)(%T::schema(stack)))
            }
        }
    )*};
}

seqs_known_schema!(
    Vec,
    BinaryHeap,
    BTreeSet,
    HashSet,
    LinkedList,
    VecDeque,
);

macro_rules! maps_known_schema {
    ($($c:ident,)*)=>{$(
        impl<K: KnownSchema, V: KnownSchema> KnownSchema for $c<K, V> {
            fn schema(parent_stack: RecurseStack) -> Schema {
                let stack = parent_stack.with_type_layer::<Self>();
                schema!(seq(varlen)(tuple {
                    (%K::schema(stack)),
                    (%V::schema(stack)),
                }))
            }
        }
    )*};
}

maps_known_schema!(
    BTreeMap,
    HashMap,
);

impl<T: KnownSchema, const LEN: usize> KnownSchema for [T; LEN] {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(seq(LEN)(%T::schema(stack)))
    }
}

impl<T: KnownSchema> KnownSchema for [T] {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(seq(varlen)(%T::schema(stack)))
    }
}

macro_rules! tuples_known_schema {
    (@inner $($t:ident),*)=>{
        impl<$($t: KnownSchema),*> KnownSchema for ($($t,)*) {
            fn schema(parent_stack: RecurseStack) -> Schema {
                let stack = parent_stack.with_type_layer::<Self>();
                schema!(tuple {$(
                    (%$t::schema(stack)),
                )*})
            }
        }
    };
    ($a:ident $(, $t:ident)*)=>{
        tuples_known_schema!(@inner $a $(, $t)*);
        tuples_known_schema!($($t),*);
    };
    ()=>{};
}

tuples_known_schema!(A, B, C, D, E, F, G, H, I, J, K);

impl<T: KnownSchema> KnownSchema for Range<T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(struct {
            (begin: %T::schema(stack)),
            (end: %T::schema(stack)),
        })
    }
}

impl<T: KnownSchema> KnownSchema for RangeInclusive<T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(struct {
            (begin: %T::schema(stack)),
            (end: %T::schema(stack)),
        })
    }
}

impl<T: KnownSchema> KnownSchema for Bound<T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        let stack = parent_stack.with_type_layer::<Self>();
        schema!(enum {
            Included(%T::schema(stack)),
            Excluded(%T::schema(stack)),
            Unbounded(unit),
        })
    }
}

impl<'a, T: KnownSchema> KnownSchema for &'a T {
    fn schema(parent_stack: RecurseStack) -> Schema {
        T::schema(parent_stack)
    }
}

impl<'a, T: KnownSchema> KnownSchema for &'a mut T {
    fn schema(parent_stack: RecurseStack) -> Schema {
        T::schema(parent_stack)
    }
}

impl<T: KnownSchema> KnownSchema for Box<T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        T::schema(parent_stack)
    }
}

impl<'a, T: KnownSchema + ToOwned> KnownSchema for Cow<'a, T> {
    fn schema(parent_stack: RecurseStack) -> Schema {
        T::schema(parent_stack)
    }
}

// TODO: unfortunately, there are more

impl KnownSchema for Schema {
    fn schema(_: RecurseStack) -> Schema {
        schema!(enum {
            Scalar(enum {
                U8(unit),
                U16(unit),
                U32(unit),
                U64(unit),
                U128(unit),
                I8(unit),
                I16(unit),
                I32(unit),
                I64(unit),
                I128(unit),
                F32(unit),
                F64(unit),
                Char(unit),
                Bool(unit),
            }),
            Str(unit),
            Bytes(unit),
            Unit(unit),
            Option(recurse(1)),
            Seq(struct {
                (len: option(u64)),
                (inner: recurse(2)),
            }),
            Tuple(seq(varlen)(recurse(2))),
            Struct(seq(varlen)(struct {
                (name: str),
                (inner: recurse(3)),
            })),
            Enum(seq(varlen)(struct {
                (name: str),
                (inner: recurse(3)),
            })),
            Recurse(u64),
        })
    }
}
