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
            $camel:ident $lower:ident $conn_key:ident $per_conn:ident $transition:ident,
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
        pub struct Connections {
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
            $lower: Slab<()>,
        )*}

        impl StateConns {
            fn get(&mut self, state: ConnState) -> &mut Slab<()> {
                match state {$(
                    ConnState::$camel => &mut self.$lower,
                )*}
            }
        }

        impl Connections {
            /// Construct without any connections.
            pub fn new() -> Self {
                Connections {
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

                let state_idx = self.state_conns.$start_lower.insert(());
                let all_idx = self.all_conns.insert(AllConnsEntry {
                    state: ConnState::$start_camel,
                    state_idx,
                    ctr,
                });
                debug_assert_eq!(
                    raw_key,
                    all_idx,
                    "Connections.insert did not follow slab pattern",
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
                    "Connections.remove something or other mismatch",
                );

                self.state_conns.get(state).remove(state_idx);
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
                /// This should be followed by corresponding removal from all
                /// `Per`-(previous state)-`Conn` structures with the old key
                /// then corresponding insertion into all
                /// `Per`-(new state)`-Conn` structures with the new key.
                ///
                /// Doesn't have to perturb `PerAnyConn` structures.
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
                        concat!("Connections.", stringify!($transition), " old state idx mismatch"),
                    );
                    self.state_conns.get(old_state).remove(old_state_idx);
                    let new_state_idx = self.state_conns.$lower.insert(());
                    self.all_conns[all_idx].state = ConnState::$camel;
                    $conn_key(ConnKeyInner {
                        all_idx,
                        state_idx: new_state_idx,
                        ctr,
                    })
                }
            )*
        }


        // ==== keys ====


        /// Key for a connection within `Connections` that's in any state.
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

        $(
            impl From<$conn_key> for AnyConnKey {
                fn from(conn_key: $conn_key) -> AnyConnKey {
                    AnyConnKey::$camel(conn_key)
                }
            }


            /// Key for a connection within `Connections` that's in this state.
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
        /// Should be updated in synchrony with `Connections`.
        #[derive(Debug, Clone)]
        pub struct PerAnyConn<T>(Slab<(usize, T)>);

        impl<T> PerAnyConn<T> {
            /// Construct empty.
            pub fn new() -> Self {
                PerAnyConn(Slab::new())
            }

            /// Insert a new element into this structure following an operation
            /// on `Connections` which's documentation says to do so.
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
            /// operation on `Connections` which's documentation says to do so.
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
            pub struct $per_conn<T>(Slab<(usize, T)>);

            impl<T> $per_conn<T> {
                /// Construct empty.
                pub fn new() -> Self {
                    $per_conn(Slab::new())
                }

                /// Insert a new element into this structure following an operation
                /// on `Connections` which's documentation says to do so.
                pub fn insert(&mut self, key: $conn_key, elem: T) {
                    debug_assert!(
                        self.0.get(key.0.state_idx).is_none(),
                        concat!(stringify!($per_conn), "insert on non-empty index"),
                    );
                    let state_idx2 = self.0.insert((key.0.ctr, elem));
                    debug_assert_eq!(
                        key.0.state_idx, state_idx2,
                        concat!(stringify!($per_conn), "insert indexes did not follow a slab pattern"),
                    );
                }

                /// Remove an existing element from this structure following an
                /// operation on `Connections` which's documentation says to do so.
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
    Uninit uninit UninitConnKey PerUninitConn transition_to_uninit,
    Client client ClientConnKey PerClientConn transition_to_client,
);
