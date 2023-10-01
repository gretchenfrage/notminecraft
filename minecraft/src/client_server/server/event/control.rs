
// ==== control events ====

/// Trusted command to reconfigure the server somehow.
#[derive(Debug)]
pub enum ControlEvent {
    /// Shut down the server cleanly.
    Stop,
}
