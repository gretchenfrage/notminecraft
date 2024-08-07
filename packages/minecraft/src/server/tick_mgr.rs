//! See `TickMgr`.

use std::time::{
    Instant,
    Duration,
};


/// Desired duration of a tick.
pub const TICK: Duration = Duration::from_millis(50);


/// Manages ticks and the passage of time.
pub struct TickMgr {
    tick: u64,
    next_tick: Instant,
}

impl TickMgr {
    /// Construct with defaults: tick number 0, scheduled to happen now.
    pub fn new() -> Self {
        TickMgr {
            tick: 0,
            next_tick: Instant::now(),
        }
    }

    /// Get the number of the currently scheduled tick.
    ///
    /// If currently doing a tick, this refers to the current tick.
    pub fn tick_num(&self) -> u64 {
        self.tick
    }

    /// Get the time that the currently scheduled tick is scheduled to occur.
    ///
    /// If currently doing a tick, this refers to the current tick.
    ///
    /// The tick "occurring" refers to the tick computations beginning. Ideally, inputs available
    /// to the game logic thread temporally before this instant should be avilable to the game
    /// logic when the next tick occurs.
    pub fn tick_instant(&self) -> Instant {
        self.next_tick
    }

    /// Call this after doing a tick, so as to update timing information and schedule the next
    /// tick.
    ///
    /// Returns the number of tick instants that will be skipped.
    pub fn on_tick_done(&mut self) -> u32 {
        self.tick += 1;

        self.next_tick += TICK;
        let now = Instant::now();
        if self.next_tick < now {
            let behind_nanos = (now - self.next_tick).as_nanos();
            // poor man's div_ceil
            let behind_ticks = match behind_nanos % TICK.as_nanos() {
                0 => behind_nanos / TICK.as_nanos(),
                _ => behind_nanos / TICK.as_nanos() + 1,
            };
            let behind_ticks = u32::try_from(behind_ticks).expect("time broke");
            warn!("running too slow, skipping {} ticks", behind_ticks);
            self.next_tick += TICK * behind_ticks;
            behind_ticks
        } else {
            0
        }
    }
}

impl Default for TickMgr {
    fn default() -> Self {
        Self::new()
    }
}
