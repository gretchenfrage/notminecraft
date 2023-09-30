
use crate::util::array::ArrayBuilder;
use std::{
    any::Any,
    sync::Arc,
    marker::PhantomData,
    thread::spawn,
};
use parking_lot::Mutex;
use slab::Slab;
use crossbeam_channel::{
    Sender,
    Receiver,
    unbounded,
};

pub const PRIORITY_LEVELS: usize = 2;

/// Pool of threads for processing moderately heavy-weight jobs.
#[derive(Debug, Clone)]
pub struct ThreadPool {
    state: Arc<ThreadPoolState>,
}

/// See `ThreadPool.create_domain`.
#[derive(Debug, Clone)]
pub struct ThreadPoolDomain<T> {
    state: Arc<ThreadPoolState>,
    domain_idx: usize,
    domain_ctr: u128,
    _p: PhantomData<T>,
}

#[derive(Debug)]
struct ThreadPoolState {
    index_state: Mutex<DomainIndexState>,
    send_any_token: Sender<()>,
    send_anys: [Sender<MsgToAny>; PRIORITY_LEVELS],
    send_alls: Vec<Sender<MsgToAll>>,
}

#[derive(Debug)]
struct DomainIndexState {
    assigner: Slab<u128>,
    counter: u128,
}

/// Message that is sent to be picked up be all threads.
enum MsgToAll {
    AddDomain {
        domain_idx: usize,
        domain_ctr: u128,
        state: Box<dyn Any + Send>,
    },
    RemoveDomain {
        domain_idx: usize,
        domain_ctr: u128,
    },  
}

/// Message that is sent to be picked up by any one thread.
struct MsgToAny {
    domain_idx: usize,
    domain_ctr: u128,
    job: Box<dyn FnOnce(&mut dyn Any) + Send>,
}


fn thread_body(
    recv_any_token: Receiver<()>,
    recv_anys: [Receiver<MsgToAny>; PRIORITY_LEVELS],
    recv_all: Receiver<MsgToAll>,
) {
    let mut domains = Slab::new();
    while recv_any_token.recv().is_ok() {
        while let Ok(msg_all) = recv_all.try_recv() {
            match msg_all {
                MsgToAll::AddDomain { domain_idx, domain_ctr, state } => {
                    let domain_idx2 = domains.insert((state, domain_ctr));
                    debug_assert_eq!(domain_idx, domain_idx2);
                }
                MsgToAll::RemoveDomain { domain_idx, domain_ctr } => {
                    let (_, domain_ctr2) = domains.remove(domain_idx);
                    debug_assert_eq!(domain_ctr, domain_ctr2);
                }
            }
        }
        let MsgToAny { domain_idx, domain_ctr, job } = recv_anys.iter()
            .rev()
            .find_map(|recv_any| recv_any.try_recv().ok())
            .unwrap();
        if let Some((ref mut domain, _)) = domains
            .get_mut(domain_idx)
            .filter(|&&mut (_, domain_ctr2)| domain_ctr == domain_ctr2)
        {
            job(domain.as_mut());
        }
    }
}

impl ThreadPool {
    /// Spawn a new thread pool with as many threads as there are CPUs.
    pub fn new() -> Self {
        let (send_any_token, recv_any_token) = unbounded();
        let mut send_anys = ArrayBuilder::new();
        let mut recv_anys = ArrayBuilder::new();
        for _ in 0..PRIORITY_LEVELS {
            let (send_any, recv_any) = unbounded();
            send_anys.push(send_any);
            recv_anys.push(recv_any);
        }
        let recv_anys = recv_anys.build();
        let send_alls = (0..num_cpus::get())
            .map(|_| {
                let recv_any_token = recv_any_token.clone();
                let recv_anys = recv_anys.clone();
                let (send_all, recv_all) = unbounded();
                spawn(move || thread_body(recv_any_token, recv_anys, recv_all));
                send_all
            })
            .collect();
        ThreadPool {
            state: Arc::new(ThreadPoolState {
                index_state: Mutex::new(DomainIndexState {
                    assigner: Slab::new(),
                    counter: 0,
                }),
                send_any_token,
                send_anys: send_anys.build(),
                send_alls,
            }),
        }
    }

    /// Create a new "domain" in this thread pool. Jobs are not submitted to
    /// the thread pool directly but rather to a domain. The given state will
    /// be cloned and sent to each thread, and tasks submitted to the domain
    /// then have the ability to access that state when being processed. If the
    /// domain is dropped, then the corresponding pieces of state on the thread
    /// will be dropped and pending tasks submitted on that domain may be
    /// dropped without being ran (which is only for performance, as relying on
    /// that for behavior is probably impossible without race conditions).
    pub fn create_domain<F, T>(&self, mut create_state: F) -> ThreadPoolDomain<T>
    where
        F: FnMut() -> T,
        T: Send + 'static
    {
        let mut guard = self.state.index_state.lock();
        let domain_ctr = guard.counter;
        guard.counter = guard.counter.checked_add(1).unwrap();
        let domain_idx = guard.assigner.insert(domain_ctr);
        for send_all in &self.state.send_alls {
            send_all.send(MsgToAll::AddDomain {
                domain_idx,
                domain_ctr,
                state: Box::new(create_state()),
            }).unwrap();
        }
        drop(guard);

        ThreadPoolDomain {
            state: self.state.clone(),
            domain_idx,
            domain_ctr,
            _p: PhantomData,
        }
    }
}

impl<T: 'static> ThreadPoolDomain<T> {
    /// Submit a task to be run on some thread. Please consider catching
    /// panics.
    pub fn submit<F: FnOnce(&mut T) + Send + 'static>(&self, job: F, priority: usize) {
        self.state.send_anys[priority].send(MsgToAny {
            domain_idx: self.domain_idx,
            domain_ctr: self.domain_ctr,
            job: Box::new(move |state| job(state.downcast_mut().unwrap())),
        }).unwrap();
        self.state.send_any_token.send(()).unwrap();
    }
}

impl<T> Drop for ThreadPoolDomain<T> {
    fn drop(&mut self) {
        let mut guard = self.state.index_state.lock();
        let domain_ctr2 = guard.assigner.remove(self.domain_idx);
        debug_assert_eq!(self.domain_ctr, domain_ctr2);
        for send_all in &self.state.send_alls {
            send_all.send(MsgToAll::RemoveDomain {
                domain_idx: self.domain_idx,
                domain_ctr: self.domain_ctr,
            }).unwrap();
        }
    }
}
