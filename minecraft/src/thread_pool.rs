// TODO: this no is efficient in terms of concurrency stuff
//       I do hope it's overshadowed by the cost of the tasks themselves, but
//       if not, I may want to optimize this itself more

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
    bounded,
};

pub const PRIORITY_LEVELS: usize = 2;

/// Pool of threads for processing moderately heavy-weight jobs.
#[derive(Debug, Clone)]
pub struct ThreadPool {
    state: Arc<ThreadPoolState>,
}

#[derive(Debug)]
struct ThreadPoolState {
    index_state: Mutex<DomainIndexState>,

    // queue of jobs for each priority level
    send_jobs: [Sender<Job>; PRIORITY_LEVELS],
    // thing in state for each worker
    workers: Vec<WorkerSenders>,
}

#[derive(Debug)]
struct DomainIndexState {
    assigner: Slab<u128>,
    counter: u128,
}

#[derive(Debug)]
struct WorkerSenders {
    // when a job is submitted, it's sent into the job queue for its priority
    // level, and then a token is sent to all workers.
    //
    // when a domain is added or removed, the appropriate Reconfigure message
    // is sent to all workers, and then a token is sent to all works.
    send_token: Sender<()>,
    send_reconfigure: Sender<Reconfigure>,
}

/// Instruction to the worker to reconfigure itself.
enum Reconfigure {
    AddDomain {
        domain_idx: usize,
        domain_ctr: u128,
        state: Box<dyn Any + Send>,
    },
    RemoveDomain {
        domain_idx: usize,
        domain_ctr: u128,
        send_dropped: Sender<()>,
    },
}

/// A job to be done.
struct Job {
    domain_idx: usize,
    domain_ctr: u128,
    job: Box<dyn FnOnce(&mut dyn Any) + Send>,
}

/// See `ThreadPool.create_domain`.
#[derive(Debug, Clone)]
pub struct ThreadPoolDomain<T> {
    state: Arc<ThreadPoolState>,
    domain_idx: usize,
    domain_ctr: u128,
    _p: PhantomData<T>,
}


// body of a worker thread
fn worker_thread(
    recv_jobs: [Receiver<Job>; PRIORITY_LEVELS],
    recv_token: Receiver<()>,
    recv_reconfigure: Receiver<Reconfigure>,
) {
    // domain state and counter
    let mut domains = Slab::new();
    // max domain_ctr we've ever added, even if we've removed it
    let mut max_domain_ctr = 0;
    // jobs for which we haven't yet initialized their necessary domain
    let mut delayed: Vec<Option<Job>> = Vec::new();

    while recv_token.recv().is_ok() {
        if let Ok(reconfigure) = recv_reconfigure.try_recv() {
            // attribute token to reconfigure message
            //
            // it's intentional that we process all reconfigures before
            // processing jobs even if we're essentially reordering out
            // attribution of what triggered these tokens.
            //
            // as for changes to domain idx and domain ctr, these should be
            // nicely serialized by the domain index state mutex.
            match reconfigure {
                Reconfigure::AddDomain { domain_idx, domain_ctr, state } => {
                    // add domain
                    let domain_idx2 = domains.insert((state, domain_ctr));
                    debug_assert_eq!(domain_idx, domain_idx2);
                    // it's ok that this starts as 0 because idk magic
                    // I don't think the race condition can occur in that case
                    debug_assert!(max_domain_ctr == 0 || domain_ctr == max_domain_ctr + 1);
                    max_domain_ctr = domain_ctr;

                    // then see if that allows us to now run any delayed jobs
                    delayed.retain_mut(|job|
                        if job.as_ref().unwrap().domain_ctr > max_domain_ctr {
                            // still can't run it, so retain it
                            true
                        } else {
                            // try and run it now, then discard it
                            run_job_unless_stale(job.take().unwrap(), &mut domains);
                            false
                        }
                    );
                }
                Reconfigure::RemoveDomain { domain_idx, domain_ctr, send_dropped } => {
                    // remove domain
                    let (state, domain_ctr2) = domains.remove(domain_idx);
                    debug_assert_eq!(domain_ctr, domain_ctr2);

                    // drop state then let the other half know we've done so
                    drop(state);
                    let _ = send_dropped.send(());
                }
            }
        } else {
            // attribute token to job entering some priority queue
            // attempt to pull the highest priority job
            if let Some(job) = recv_jobs.iter()
                .rev()
                .find_map(|recv_any| recv_any.try_recv().ok())
            {
                if job.domain_ctr > max_domain_ctr {
                    // there's possible race conditions here where we receive a
                    // job before we receive the AddDomain it needs to run, so
                    // we buffer it for once we can run it
                    delayed.push(Some(job));
                } else {
                    // but otherwise, try and run it now!
                    run_job_unless_stale(job, &mut domains);
                }
            }
            // if we didn't find any we just assume other workers snatched up
            // all the jobs those tokens were about, which is fine
        }
    }
}

fn run_job_unless_stale(job: Job, domains: &mut Slab<(Box<dyn Any + Send>, u128)>) {
    // if the domain ctr doesn't match, assume that this is a
    // stale job for a removed domain, and just discard it
    if let Some((ref mut state, _)) = domains
        .get_mut(job.domain_idx)
        .filter(|&&mut (_, domain_ctr)| domain_ctr == job.domain_ctr)
    {
        // but if we can run the job now, run it!
        (job.job)(&mut **state);
    }
}


impl ThreadPool {
    /// Spawn a new thread pool with as many threads as there are CPUs.
    pub fn new() -> Self {
        let mut send_jobs = ArrayBuilder::new();
        let mut recv_jobs = ArrayBuilder::new();
        for _ in 0..PRIORITY_LEVELS {
            let (send_job, recv_job) = unbounded();
            send_jobs.push(send_job);
            recv_jobs.push(recv_job);
        }
        let send_jobs = send_jobs.build();
        let recv_jobs = recv_jobs.build();

        let workers = (0..num_cpus::get())
            .map(|_| {
                let recv_jobs = recv_jobs.clone();
                let (send_token, recv_token) = unbounded();
                let (send_reconfigure, recv_reconfigure) = unbounded();
                spawn(move || worker_thread(recv_jobs, recv_token, recv_reconfigure));
                WorkerSenders {
                    send_token,
                    send_reconfigure,
                }
            })
            .collect();

        ThreadPool {
            state: Arc::new(ThreadPoolState {
                index_state: Mutex::new(DomainIndexState {
                    assigner: Slab::new(),
                    counter: 0,
                }),
                send_jobs,
                workers,
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

        for worker in &self.state.workers {
            worker.send_reconfigure.send(Reconfigure::AddDomain {
                domain_idx,
                domain_ctr,
                state: Box::new(create_state()),
            }).unwrap();
            worker.send_token.send(()).unwrap();
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
        self.state.send_jobs[priority].send(Job {
            domain_idx: self.domain_idx,
            domain_ctr: self.domain_ctr,
            job: Box::new(move |state| {
                job(state.downcast_mut().unwrap());
            }),
        }).unwrap();

        for worker in &self.state.workers {
            worker.send_token.send(()).unwrap();
        }
    }
}

/// This not only triggers the threads to drop their state for this domain, it
/// actually blocks until they've all done so.
impl<T> Drop for ThreadPoolDomain<T> {
    fn drop(&mut self) {
        let (send_dropped, recv_dropped) = bounded(self.state.workers.len());

        let mut guard = self.state.index_state.lock();

        let domain_ctr2 = guard.assigner.remove(self.domain_idx);
        debug_assert_eq!(self.domain_ctr, domain_ctr2);

        for worker in &self.state.workers {
            worker.send_reconfigure.send(Reconfigure::RemoveDomain {
                domain_idx: self.domain_idx,
                domain_ctr: self.domain_ctr,
                send_dropped: send_dropped.clone(),
            }).unwrap();
            worker.send_token.send(()).unwrap();
        }

        drop(guard);
        
        for _ in 0..self.state.workers.len() {
            recv_dropped.recv().unwrap();
        }
    }
}
