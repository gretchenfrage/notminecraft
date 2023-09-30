
use chunk_data::*;
use vek::*;


// when combined with other parts of the system, essentially determines the
// total number of chunks that are allowed to be in transit to be added to a
// client at a particular instant.
const DEFAULT_BUDGET: u32 = 20;


/// For a single client, manages the limited rate at which chunks can be added
/// to that client.
pub struct ClientAddChunkManager {
    // number of additional chunks that could be added to the client
    // immediately.
    budget: u32,

    // linked queue of additional chunks to be added to the client when able.
    queue: PerChunk<Option<QueueNode>>,

    // front (popped from) and back (pushed to) of queue, unless empty.
    front_back: Option<(CcCi, CcCi)>,
}

// node in queue
#[derive(Debug, Copy, Clone)]
struct QueueNode {
    // next closer node to front of queue, unless this is the front.
    to_front: Option<CcCi>,
    // next closer node to the back of queue, unless this is the back.
    to_back: Option<CcCi>,
}

type CcCi = (Vec3<i64>, usize);


impl ClientAddChunkManager {
    /// Construct with defaults.
    pub fn new(chunks: &LoadedChunks) -> Self {
        ClientAddChunkManager {
            budget: DEFAULT_BUDGET,
            queue: chunks.new_per_chunk_mapped(|_, _| None),
            front_back: None,
        }
    }

    /// Call when a chunk is added to the server.
    pub fn on_add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.queue.add(cc, ci, None);
    }

    /// Call when a chunk is removed from the server.
    ///
    /// Chunk must not be in the queue.
    pub fn on_remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        let opt = self.queue.remove(cc, ci);
        debug_assert!(opt.is_none());
    }

    /// If the budget allows this chunk to be added to the client now, update
    /// budget info and return true. The user should then add it to the client.
    ///
    /// Elsewise, enqueue the chunk to be added when the budget allows it and
    /// return false.
    pub fn maybe_add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize) -> bool {
        debug_assert!(self.queue.get(cc, ci).is_none());
        if self.budget > 0 {
            self.budget -= 1;
            true
        } else {
            // add to back of queue
            let new_back = (cc, ci);

            if let Some((front, old_back)) = self.front_back {
                self.queue.get_mut(old_back.0, old_back.1)
                    .as_mut().unwrap()
                    .to_back = Some(new_back);

                *self.queue.get_mut(new_back.0, new_back.1) = Some(QueueNode {
                    to_front: Some(old_back),
                    to_back: None,
                });

                self.front_back = Some((front, new_back));
            } else {
                *self.queue.get_mut(new_back.0, new_back.1) = Some(QueueNode {
                    to_front: None,
                    to_back: None,
                });

                self.front_back = Some((new_back, new_back));
            }

            false
        }
    }

    /// Remove a chunk from the queue of chunks that would be added to the
    /// client upon resources allowing it.
    pub fn remove_from_queue(&mut self, cc: Vec3<i64>, ci: usize) {
        // remove queue node
        let QueueNode {
            to_front,
            to_back,
        } = self.queue.get_mut(cc, ci).take().unwrap();

        // re-link neighbors around it
        if let Some(to_front) = to_front {
            self.queue.get_mut(to_front.0, to_front.1).as_mut().unwrap().to_back = to_back;
        } else {
            let (_, back) = self.front_back.unwrap();
            self.front_back = to_back.map(|to_back| (to_back, back));
        }

        if let Some(to_back) = to_back {
            self.queue.get_mut(to_back.0, to_back.1).as_mut().unwrap().to_front = to_front;
        } else if let Some((front, _)) = self.front_back {
            self.front_back = to_front.map(|to_front| (front, to_front));
        }
    }

    /// Permit `amount` additional "add chunk to client" operations to occur to
    /// the client.
    pub fn increase_budget(&mut self, amount: u32) {
        self.budget += amount;
    }

    /// If the queue is non-empty and the budget allows a chunk to be added to
    /// the client now, update budget info and return the front of the queue.
    pub fn poll_queue(&mut self) -> Option<(Vec3<i64>, usize)> {
        if self.budget > 0 {
            if let Some((old_front, back)) = self.front_back {
                self.budget -= 1;

                // pop from front of queue
                let QueueNode {
                    to_front,
                    to_back,
                } = self.queue.get_mut(old_front.0, old_front.1).take().unwrap();
                debug_assert!(to_front.is_none());

                // re-link back
                if let Some(to_back) = to_back {
                    self.queue.get_mut(to_back.0, to_back.1).as_mut().unwrap().to_front = None;
                    self.front_back = Some((to_back, back));
                } else {
                    self.front_back = None;
                }

                Some(old_front)
            } else {
                None
            }
        } else {
            None
        }
    }
}
