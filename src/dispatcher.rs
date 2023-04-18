//! RPC dispatcher types

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use async_timer_rs::Timer;

use crate::responder::{Responder, Response};

/// RPC dispatcher
#[derive(Debug, Clone)]
pub struct Dispatcher<Input, Output> {
    sender: SyncSender<(u64, Input)>,
    pub responder: Responder<Output>,
}

impl<Input, Output> Dispatcher<Input, Output>
where
    Input: Send + Sync + 'static,
{
    /// Create new dispatcher and
    pub fn new(cache_size: usize) -> (Self, Receiver<(u64, Input)>) {
        let (sender, receiver) = sync_channel(cache_size);

        (
            Self {
                sender,
                responder: Responder::new(),
            },
            receiver,
        )
    }

    /// Start a new rpc call with sequence id.
    pub fn call<T: Timer>(&self, id: u64, input: Input, timeout: Option<T>) -> Response<T, Output> {
        match self.sender.send((id, input)) {
            Ok(_) => Response::poller(id, self.responder.clone(), timeout),
            Err(err) => Response::error(std::io::Error::new(std::io::ErrorKind::BrokenPipe, err)),
        }
    }
}
