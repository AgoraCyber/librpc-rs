//! RPC dispatcher types

use std::io::Result;

use async_timer_rs::Timer;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    SinkExt,
};

use crate::responder::{Responder, Response};

/// RPC dispatcher
#[derive(Debug, Clone)]
pub struct Dispatcher<Input, Output> {
    sender: Sender<(u64, Input)>,
    pub responder: Responder<Output>,
}

impl<Input, Output> Dispatcher<Input, Output>
where
    Input: Send + Sync + 'static,
{
    /// Create new dispatcher and
    pub fn new(cache_size: usize) -> (Self, Receiver<(u64, Input)>) {
        let (sender, receiver) = channel(cache_size);

        (
            Self {
                sender,
                responder: Responder::new(),
            },
            receiver,
        )
    }

    /// Start a new rpc call with sequence id.
    pub async fn call<T: Timer>(
        &mut self,
        id: u64,
        input: Input,
        timeout: Option<T>,
    ) -> Result<Response<T, Output>> {
        match self.sender.send((id, input)).await {
            Ok(_) => Ok(Response::new(id, self.responder.clone(), timeout)),
            Err(err) => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, err)),
        }
    }
}
