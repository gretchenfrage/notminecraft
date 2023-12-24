
use crate::util_abort_handle::AbortHandle;
use crossbeam::{
    queue::SegQueue,
    sync::Parker,
};
use std::{
    thread,
    sync::{
        Arc,
        atomic::{
            AtomicU32,
            Ordering,
        },
    },
};


/// Priority level. Variants decrease in priority.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(usize)]
pub enum WorkPriority {
    /// Work for client.
    Client = 0,
    /// Work for server.
    Server = 1,
}

// number of priority levels
const LEVELS: usize = 2;


/// Work stealing thread pool with priority levels.
pub struct ThreadPool(Arc<State>);

// shared state
struct State {
    // per thread, per priority level, job queue
    thread_queues: Vec<[SegQueue<Job>; LEVELS]>,
    // for worker threads to sleep when there's no more work
    parker: Parker,
    // rotating index for which thread to submit work to
    insert_to: AtomicU32,
    // counter of how may ThreadPool handles remain. worker threads shut down once all work is
    // completed and this is 0, thus implying there will be no more work.
    alive: AtomicU32,
}

// job sent to worker thread
struct Job {
    aborted: AbortHandle,
    work: Box<dyn FnOnce(AbortHandle) + Send + 'static>,
}

impl ThreadPool {
    /// Construct, spawning threads.
    pub fn new() -> Self {
        let cpus = num_cpus::get();
        let state = Arc::new(State {
            thread_queues: vec![Default::default(); cpus],
            parker: Parker::new(),
            insert_to: AtomicU32::new(0),
            alive: AtomicU32::new(1),
        });
        for q in 0..cpus {
            let state = Arc::clone(&state);
            thread::spawn(move || thread_body(q, state));
        }
        ThreadPool(state)
    }

    /// Submit a job to be done, with the given priority level and abort handle.
    ///
    /// Higher priority jobs are executed before lower priority jobs. The abort handle is checked
    /// before executing and if aborted the job is discarded. If the work is done, it gets passed
    /// the provided abort handle.
    pub fn submit<F>(&self, priority: WorkPriority, aborted: AbortHandle, work: F)
    where
        F: FnOnce(AbortHandle) + Send + 'static,
    {
        let q = self.0.insert_to.fetch_add(1, Ordering::Relaxed);
        let q = q as usize % self.thread_queues.0.len();
        self.0.thread_queues[q][priority as usize].push(Job {
            aborted,
            work: Box::new(work) as _,
        });
        self.0.unparker().unpark();
    }
}

fn thread_body(q: usize, state: Arc<State>) {
    let my_queues = &state.thread_queues[q];
    'outer: loop {
        // loop for trying to process work for our own queue
        'my_queues: loop {
            // do whatever job can be found at the best priority
            for queue in my_queues {
                if let Some(job) = state.thread_queues[q][p].pop() {
                    do_job(job);
                    continue 'my_queues;
                }
            }
            // if now jobs can be found in our own queues, break the loop
            break 'my_queues;
        }
        // loop for trying to process work from neighbors queues
        for offset in 1..state.thread_queues.len() {
            // look through neighbors increasingly "to the right" (wrapping)
            let q2 = (q + offset) % state.thread_queues.len()
            // and try to find a job at the best priority
            for queue in &state.thread_queues[q2] {
                if let Some(job) = state.thread_queues[q][p].pop() {
                    do_job(job);
                    continue 'outer;
                }
            }
        }
        // and if none can be found anywhere, see if the pool is just dead
        if state.alive.load(Ordering::SeqCst) == 0 {
            // if it is, die, but make sure to maintain a chain reaction of sleeping threads waking
            // each other up so they all notice the pool is dead and shut off
            self.0.parker.unparker().unpark();
            return;
        }
        // elsewise, we're probably just empty, so park until waken up
        state.parker.park();
    }
}

fn do_job(job: Job) {
    if !job.aborted.is_aborted() {
        (job.work)(job.aborted());
    }
}

impl Clone for ThreadPool {
    fn clone(&self) -> Self {
        self.0.alive.fetch_add(1, Ordering::SeqCst);
        ThreadPool(Arc::clone(&self.0))
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        let alive = self.0.alive.fetch_sub(1, Ordering::SeqCst);
        if alive == 1 {
            self.0.parker.unparker().unpark();
        }
    }
}
