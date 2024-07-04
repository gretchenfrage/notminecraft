//! Time handling utilities.

use crate::game_binschema::GameBinschema;
use std::time::{Instant, Duration};


/// Microsecond-precision timestamp relative to a known or estimated `server_t0` instant of a
/// particular server/client connection.
///
/// Suitable for transmission across that connection to convey real time instants without relying
/// on either side having accurately synchronized Unix clocks.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, GameBinschema)]
pub struct ServerRelTime(pub i64);

impl ServerRelTime {
    /// Represent an instant relative to `server_t0`.
    ///
    /// Warns and saturates if called with instants ridiculously far before or after `server_t0`.
    pub fn new(instant: Instant, server_t0: Instant) -> Self {
        ServerRelTime(if instant >= server_t0 {
            i64::try_from(instant.duration_since(server_t0).as_micros()).ok()
                .unwrap_or_else(|| {
                    warn!("time_relativize called with timestamp ridiculously far into future");
                    i64::MAX
                })
        } else {
            i64::try_from(server_t0.duration_since(instant).as_micros()).ok()
                .unwrap_or_else(|| {
                    warn!("time_relativize called with timestamp ridiculously far into past");
                    i64::MIN
                })
                .saturating_neg()
        })
    }

    /// Convert to an instant relative to `server_t0`.
    pub fn to_instant(self, server_t0: Instant) -> Instant {
        if self.0 >= 0 {
            server_t0 + Duration::from_micros(self.0 as u64)
        } else {
            server_t0 - Duration::from_micros(-self.0 as u64)
        }
    }
}
