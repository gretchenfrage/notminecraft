//! Helpers for managing the different states connections can be in and the
//! different index spaces that involves.

use std::ops::{
    Index,
    IndexMut,
};
use slab::Slab;


macro_rules! connection_states {
    (
        $start_camel:ident $start_lower:ident $start_conn_key:ident,
        $(
            $camel:ident $lower:ident $conn_key:ident $per_conn:ident $transition:ident $iter:ident $new_mapped_per:ident,
        )*
    )=>{
        /// State a connection can be in.
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum ConnState {$(
            $camel,
        )*}

        /// Tracks the set of connections that exist and which state they're
        /// in, and manages the various index spaces involved.
        #[derive(Debug)]
        pub struct ConnStates {
            all_conns: Slab<AllConnsEntry>,
            state_conns: StateConns,
            next_ctr: usize,
        }

        #[derive(Debug, Copy, Clone)]
        struct AllConnsEntry {
            state: ConnState,
            state_idx: usize,
            ctr: usize,
        }

        #[derive(Debug)]
        struct StateConns {$(
            // backlinks state idx -> all idx
            $lower: Slab<usize>,
        )*}

        impl StateConns {
            fn get(&mut self, state: ConnState) -> &mut Slab<usize> {
                match state {$(
                    ConnState::$camel => &mut self.$lower,
                )*}
            }
        }

        impl ConnStates {
            /// Construct without any connections.
            pub fn new() -> Self {
                ConnStates {
                    all_conns: Slab::new(),
                    state_conns: StateConns {$(
                        $lower: Slab::new(),
                    )*},
                    next_ctr: 0,
                }
            }

            /// Insert a new connection into the data structure by raw key,
            /// initializing it in the starting state and assigning it a key
            /// in that state.
            ///
            /// This should be followed by corresponding insertion into all
            /// `PerAnyConn` and all `Per`-(start state)-`Conn` structures.
            ///
            /// Raw key insertions and removals are expected to follow a slab
            /// pattern.
            pub fn insert(&mut self, raw_key: usize) -> $start_conn_key {
                let ctr = self.next_ctr;
                self.next_ctr = self.next_ctr.wrapping_add(1);

                let state_idx = self.state_conns.$start_lower.insert(self.all_conns.vacant_key());
                let all_idx = self.all_conns.insert(AllConnsEntry {
                    state: ConnState::$start_camel,
                    state_idx,
                    ctr,
                });
                debug_assert_eq!(
                    raw_key,
                    all_idx,
                    "ConnStates.insert did not follow slab pattern",
                );

                $start_conn_key(ConnKeyInner { all_idx, state_idx, ctr })
            }

            /// Remove an existing connection from the data structure, freeing
            /// up its assigned key for later use.
            ///
            /// This should be followed by corresponding insertion from all
            /// `PerAnyConn` structures and all `Per`-(current state)-`Conn`
            /// structures.
            ///
            /// Raw key insertions and removals are expected to follow a slab
            /// pattern.
            pub fn remove(&mut self, key: impl Into<AnyConnKey>) {
                let key = key.into();
                let state = key.state();
                let ConnKeyInner { all_idx, state_idx, ctr } = key.inner();

                let AllConnsEntry {
                    state: state2,
                    state_idx: state_idx2,
                    ctr: ctr2,
                } = self.all_conns.remove(all_idx);
                debug_assert_eq!(
                    (state, state_idx, ctr),
                    (state2, state_idx2, ctr2),
                    "ConnStates.remove something or other mismatch",
                );

                let all_idx2 = self.state_conns.get(state).remove(state_idx);
                debug_assert_eq!(all_idx, all_idx2);
            }

            /// Translate from raw key to typed key.
            pub fn lookup(&self, raw_key: usize) -> AnyConnKey {
                let AllConnsEntry { state, state_idx, ctr } = self.all_conns[raw_key];
                let inner = ConnKeyInner { all_idx: raw_key, state_idx, ctr };
                match state {$(
                    ConnState::$camel => $conn_key(inner).into(),
                )*}
            }

            $(
                /// Transition an existing connection from its previous state
                /// into this new state, returning its state-changed key.
                ///
                /// This should be preceded by corresponding removal from all
                /// `Per`-(previous state)-`Conn` structures with the old key
                /// and followed with corresponding insertion into all
                /// `Per`-(new state)`-Conn` structures with the new key.
                ///
                /// Doesn't have to perturb `PerAnyConn` structures.
                #[allow(unused)]
                pub fn $transition(&mut self, key: impl Into<AnyConnKey>) -> $conn_key {
                    let key = key.into();
                    let old_state = key.state();
                    let ConnKeyInner {
                        all_idx,
                        state_idx: old_state_idx,
                        ctr,
                    } = key.inner();
                    debug_assert_eq!(
                        old_state_idx, self.all_conns[all_idx].state_idx,
                        concat!("ConnStates.", stringify!($transition), " old state idx mismatch"),
                    );
                    let all_idx2 = self.state_conns.get(old_state).remove(old_state_idx);
                    debug_assert_eq!(all_idx, all_idx2);
                    let new_state_idx = self.state_conns.$lower.insert(all_idx);
                    self.all_conns[all_idx].state = ConnState::$camel;
                    self.all_conns[all_idx].state_idx = new_state_idx;
                    $conn_key(ConnKeyInner {
                        all_idx,
                        state_idx: new_state_idx,
                        ctr,
                    })
                }
            )*

            /// Iterate through all connections.
            #[allow(unused)]
            pub fn iter_any<'a>(&'a self) -> impl Iterator<Item=AnyConnKey> + 'a {
                self.all_conns.iter()
                    .map(|(
                        all_idx,
                        &AllConnsEntry { state, state_idx, ctr },
                    )| match state {$(
                        ConnState::$camel => AnyConnKey::$camel($conn_key(
                            ConnKeyInner { all_idx, state_idx, ctr }
                        )),
                    )*})
            }

            $(
                /// Iterate through all connections in this state.
                #[allow(unused)]
                pub fn $iter<'a>(&'a self) -> impl Iterator<Item=$conn_key> + 'a {
                    self.state_conns.$lower.iter()
                        .map(|(state_idx, &all_idx)| $conn_key(ConnKeyInner {
                            all_idx,
                            state_idx,
                            ctr: self.all_conns[all_idx].ctr,
                        }))
                }
            )*

            /// Map all connections to construct a `PerAnyConn` which is new
            /// yet synchronized with self. 
            #[allow(unused)]
            pub fn new_mapped_per_any<T, F>(&self, mut f: F) -> PerAnyConn<T>
            where
                F: FnMut(AnyConnKey) -> T,
            {
                PerAnyConn(self.all_conns.new_mapped(
                    |all_idx, &AllConnsEntry { state, state_idx, ctr }|
                    match state {$(
                        ConnState::$camel => (
                            ctr,
                            f(AnyConnKey::$camel($conn_key(
                                ConnKeyInner { all_idx, state_idx, ctr }
                            ))),
                        ),
                    )*}
                ))
            }

            $(
                /// Construct a new `Per`-state-`Conn` with its structure
                /// pre-synchronized with self. Entries are populated as
                /// `None`.
                #[allow(unused)]
                pub fn $new_mapped_per<T, F>(&self, mut f: F) -> $per_conn<T>
                where
                    F: FnMut($conn_key) -> T,
                {
                    $per_conn(self.state_conns.$lower.new_mapped(
                        |state_idx, &all_idx| {
                            let ctr = self.all_conns[all_idx].ctr;
                            (
                                ctr,
                                f($conn_key(ConnKeyInner { all_idx, state_idx, ctr })),
                            )
                        }
                    ))
                }
            )*
        }


        // ==== keys ====


        /// Key for a connection within `ConnStates` that's in any state.
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum AnyConnKey {$(
            $camel($conn_key),
        )*}

        impl AnyConnKey {
            pub fn state(self) -> ConnState {
                match self {$(
                    AnyConnKey::$camel(_) => ConnState::$camel,
                )*}
            }

            fn inner(self) -> ConnKeyInner {
                match self {$(
                    AnyConnKey::$camel($conn_key(inner)) => inner,
                )*}
            }
        }

        /// Error for (state)-`ConnKey` as `TryFrom<AnyConnKey>`.
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct WrongConnState {
            pub required: ConnState,
            pub actual: ConnState,
        }

        $(
            impl From<$conn_key> for AnyConnKey {
                fn from(conn_key: $conn_key) -> AnyConnKey {
                    AnyConnKey::$camel(conn_key)
                }
            }

            impl TryFrom<AnyConnKey> for $conn_key {
                type Error = WrongConnState;

                fn try_from(key: AnyConnKey) -> Result<Self, WrongConnState> {
                    if let AnyConnKey::$camel(key) = key {
                        Ok(key)
                    } else {
                        Err(WrongConnState {
                            required: ConnState::$camel,
                            actual: key.state(),
                        })
                    }
                }
            }

            /// Key for a connection within `ConnStates` that's in this state.
            #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
            pub struct $conn_key(ConnKeyInner);
        )*


        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        struct ConnKeyInner {
            all_idx: usize,
            state_idx: usize,
            ctr: usize,
        }


        // ==== "per" ====


        /// Storage of `T` for each current connection regardless of state.
        /// Should be updated in synchrony with `ConnStates`.
        #[derive(Debug, Clone)]
        pub struct PerAnyConn<T>(Slab<(usize, T)>);

        impl<T> PerAnyConn<T> {
            /// Construct empty.
            pub fn new() -> Self {
                PerAnyConn(Slab::new())
            }

            /// Insert a new element into this structure following an operation
            /// on `ConnStates` which's documentation says to do so.
            pub fn insert(&mut self, key: impl Into<AnyConnKey>, elem: T) {
                let ConnKeyInner { all_idx, ctr, .. } = key.into().inner();
                debug_assert!(
                    self.0.get(all_idx).is_none(),
                    "PerAnyConn.insert on non-empty index",
                );
                let all_idx2 = self.0.insert((ctr, elem));
                debug_assert_eq!(
                    all_idx, all_idx2,
                    "PerAnyConn.insert indexes did not follow a slab pattern",
                );
            }

            /// Remove an existing element from this structure following an
            /// operation on `ConnStates` which's documentation says to do so.
            pub fn remove(&mut self, key: impl Into<AnyConnKey>) -> T {
                let ConnKeyInner { all_idx, ctr, .. } = key.into().inner();
                let (ctr2, elem) = self.0.remove(all_idx);
                debug_assert_eq!(
                    ctr, ctr2,
                    "PerAnyConn.remove ctr mismatch",
                );
                elem
            }
        }

        impl<T, K: Into<AnyConnKey>> Index<K> for PerAnyConn<T> {
            type Output = T;

            fn index(&self, key: K) -> &T {
                let ConnKeyInner { all_idx, ctr, .. } = key.into().inner();
                let &(ctr2, ref t) = &self.0[all_idx];
                debug_assert_eq!(ctr, ctr2, "PerAnyConn ctr mismatch");
                t
            }
        }

        impl<T, K: Into<AnyConnKey>> IndexMut<K> for PerAnyConn<T> {
            fn index_mut(&mut self, key: K) -> &mut T {
                let ConnKeyInner { all_idx, ctr, .. } = key.into().inner();
                let &mut (ctr2, ref mut t) = &mut self.0[all_idx];
                debug_assert_eq!(ctr, ctr2, "PerAnyConn ctr mismatch");
                t
            }
        }

        $(
            #[derive(Debug, Clone)]
            #[allow(unused)]
            pub struct $per_conn<T>(Slab<(usize, T)>);

            #[allow(unused)]
            impl<T> $per_conn<T> {
                /// Construct empty.
                pub fn new() -> Self {
                    $per_conn(Slab::new())
                }

                /// Insert a new element into this structure following an operation
                /// on `ConnStates` which's documentation says to do so.
                pub fn insert(&mut self, key: $conn_key, elem: T) {
                    debug_assert!(
                        self.0.get(key.0.state_idx).is_none(),
                        concat!(stringify!($per_conn), ".insert on non-empty index"),
                    );
                    let state_idx2 = self.0.insert((key.0.ctr, elem));
                    debug_assert_eq!(
                        key.0.state_idx, state_idx2,
                        concat!(stringify!($per_conn), "insert indexes did not follow a slab pattern"),
                    );
                }

                /// Remove an existing element from this structure following an
                /// operation on `ConnStates` which's documentation says to do so.
                pub fn remove(&mut self, key: $conn_key) -> T {
                    let (ctr2, elem) = self.0.remove(key.0.state_idx);
                    debug_assert_eq!(
                        key.0.ctr, ctr2,
                        concat!(stringify!($per_conn), "remove ctr mismatch"),
                    );
                    elem
                }
            }

            impl<T> Index<$conn_key> for $per_conn<T> {
                type Output = T;

                fn index(&self, key: $conn_key) -> &T {
                    let &(ctr2, ref t) = &self.0[key.0.state_idx];
                    debug_assert_eq!(key.0.ctr, ctr2, concat!(stringify!($per_conn), " ctr mismatch"));
                    t
                }
            }

            impl<T> IndexMut<$conn_key> for $per_conn<T> {
                fn index_mut(&mut self, key: $conn_key) -> &mut T {
                    let &mut (ctr2, ref mut t) = &mut self.0[key.0.state_idx];
                    debug_assert_eq!(key.0.ctr, ctr2, concat!(stringify!($per_conn), " ctr mismatch"));
                    t
                }
            }
        )*
    };
}

connection_states!(
    // the default state:
    Uninit uninit UninitConnKey,

    // list of states:
    Uninit uninit UninitConnKey PerUninitConn transition_to_uninit iter_uninit new_mapped_per_uninit,
    Client client ClientConnKey PerClientConn transition_to_client iter_client new_mapped_per_client,
    Closed closed ClosedConnKey PerClosedConn transition_to_closed iter_closed new_mapped_per_closed,
);
