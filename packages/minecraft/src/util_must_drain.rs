//! Return type for manager methods which necessitate draining an effect queue.


/// Denotes that calling a manager method must be followed by draining the manager's effect queue.
///
/// It is a common pattern we employ that a manager maintains an internal queue of "effects",
/// denoting instructions flowing to the caller to make other things happen externally, and that
/// the caller should drain and process this effect queue after calling certain methods on the
/// manager. To help prevent bugs wherein the caller forgets to do this, these methods can be made
/// to return `MustDrain`, which is annotated with `must_use.
#[must_use]
pub struct MustDrain;
