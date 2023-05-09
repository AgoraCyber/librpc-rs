//! RPC client type

use crossbeam_channel::{bounded, Receiver};

/// Channel client side
pub struct Client<Payload> {
    receiver: Receiver<Payload>,
}

impl<Payload> Client<Payload> {
    /// Create new client instance with channel `cap`
    pub fn new(cap: usize) -> Self {
        let (sender, receiver) = bounded(cap);
        Client { receiver }
    }
}
