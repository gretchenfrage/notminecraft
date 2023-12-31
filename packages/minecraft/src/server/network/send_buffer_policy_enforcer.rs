//! Enforcing "send buffer policies" in connection to clients.
//!
//! This means policies to set a limit for how much of certain things can be buffered in server
//! memory, waiting to be transmitted to the client, before the client is allowed to request more.
//! This protects the server against slow-read attacks wherein the client could request that many
//! large resources be sent to it and not actually read them, causing the server's send buffer to
//! get really large and make the server run out of memory. This is separated out here because it
//! is both neutral to connection transport and particularly game logic-laden.
//!
//! This can be seen as kind of like an alternative to client-to-server message backpressure, which
//! is more coupled to game logic in exchange for simpler interactions with the game logic's more
//! stringent synchronization requirements.

use std::sync::atomic::{
    AtomicU64,
    Ordering::SeqCst,
};
use anyhow::*;


/// Enforces send buffer policies for a single connection. Should be shared between the sending and
/// receiving half.
pub(super) struct SendBufferPolicyEnforcer {
    accept_more_chunks_budget: AtomicU64,
}

impl SendBufferPolicyEnforcer {
    /// Construct new.
    pub(super) fn new() -> Self {
        SendBufferPolicyEnforcer {
            accept_more_chunks_budget: AtomicU64::new(0),
        }
    }

    /// Called right before the message is transmitted.
    pub(super) fn pre_transmit(&self, msg: &DownMsg) {
        if let &DownMessage::AddChunk(_) = msg {
            self.accept_more_chunks_budget.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Called right after the message is received. If errors, send buffer policies were violated
    /// and the connection should be killed.
    pub(super) fn post_receive(&self, msg: &UpMsg) -> Result<()> {
        if let &UpMessage::AcceptMoreChunks(n) = msg {
            let pre_sub = self.accept_more_chunks_budget.fetch_sub(n, Ordering::SeqCst);
            if n as u64 > pre_sub {
                bail!("client violated accept more chunks buffer policy");
            }
        }
        Ok(())
    }
}
