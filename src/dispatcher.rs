//! RPC dispatcher types

use async_timer_rs::Timer;
use futures::{
    channel::mpsc::{channel, Receiver, SendError, Sender},
    SinkExt,
};

use crate::responder::{Responder, Response};

/// RPC dispatcher
#[derive(Debug)]
pub struct Dispatcher<Input, Output, Error> {
    sender: Sender<(Option<u64>, Input)>,
    pub responder: Responder<Output, Error>,
}

impl<Input, Output, Error> Clone for Dispatcher<Input, Output, Error> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            responder: self.responder.clone(),
        }
    }
}

impl<Input, Output, Error> Dispatcher<Input, Output, Error>
where
    Input: Send + Sync + 'static,
    Error: From<SendError>,
{
    /// Create new dispatcher and
    pub fn new(cache_size: usize) -> (Self, Receiver<(Option<u64>, Input)>) {
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
    ) -> Result<Response<T, Output, Error>, Error> {
        match self.sender.send((Some(id), input)).await {
            Ok(_) => Ok(Response::new(id, self.responder.clone(), timeout)),
            Err(err) => Err(err.into()),
        }
    }

    /// Start send one notification to remote.
    pub async fn notification(&mut self, input: Input) -> Result<(), Error> {
        match self.sender.send((None, input)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}
