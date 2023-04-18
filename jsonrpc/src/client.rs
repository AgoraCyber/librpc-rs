use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use async_timer_rs::Timer;
use futures::channel::mpsc::Receiver;
use librpc::dispatcher::Dispatcher;
use serde::{Deserialize, Serialize};

use crate::{
    object::{Request, Version},
    result::{RPCError, RPCResult},
};

/// JSONRPC V2.0 client
#[derive(Debug, Clone)]
pub struct Client {
    id_gen: Arc<AtomicU64>,
    dispatcher: Dispatcher<Vec<u8>, Vec<u8>, RPCError>,
}

pub type Responder = librpc::responder::Responder<Vec<u8>, RPCError>;
pub type Output = Receiver<(Option<u64>, Vec<u8>)>;

impl Client {
    /// Create new JSONRPC client instance with sending cache quene length.
    pub fn new(cache_size: usize) -> (Self, Output, Responder) {
        let (dispatcher, receiver) = Dispatcher::new(cache_size);

        let responder = dispatcher.responder.clone();

        (
            Client {
                id_gen: Default::default(),
                dispatcher,
            },
            receiver,
            responder,
        )
    }

    /// Asynchronous send a JSONRPC v2.0 request and wait response
    pub async fn call<P, R, T>(
        &mut self,
        method: &str,
        params: P,
        timeout: Option<T>,
    ) -> RPCResult<R>
    where
        P: Serialize,
        for<'b> R: Deserialize<'b> + Send + 'static,
        T: Timer + Unpin,
    {
        let id = self.id_gen.fetch_add(1, Ordering::SeqCst);

        let request = Request {
            id: Some(id),
            method,
            params,
            jsonrpc: Version::default(),
        };

        let data = serde_json::to_vec(&request).expect("Inner error, assembly json request");

        let result = self.dispatcher.call(id, data, timeout).await?.await?;

        Ok(serde_json::from_slice(&result)?)
    }

    /// Asynchronous send a JSONRPC v2.0 notification
    pub async fn notification<P>(&mut self, method: &str, params: P) -> RPCResult<()>
    where
        P: Serialize,
    {
        let request = Request {
            method,
            params,
            id: None,
            jsonrpc: Version::default(),
        };

        let data = serde_json::to_vec(&request)?;

        self.dispatcher.notification(data).await?;

        Ok(())
    }
}
