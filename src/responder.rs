//! RPC response associate types.

use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use async_timer_rs::Timer;
use futures::FutureExt;

#[derive(Debug)]
struct DispatcherImpl<Output, Error> {
    wakers: HashMap<u64, Waker>,
    completed: HashMap<u64, Result<Output, Error>>,
}

impl<Output, Error> Default for DispatcherImpl<Output, Error> {
    fn default() -> Self {
        Self {
            wakers: HashMap::new(),
            completed: HashMap::new(),
        }
    }
}

/// Rpc message dispatcher.
#[derive(Debug)]
pub struct Responder<Output, Error> {
    inner: Arc<Mutex<DispatcherImpl<Output, Error>>>,
}

impl<Output, Error> Clone for Responder<Output, Error> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<Output, Error> Responder<Output, Error> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DispatcherImpl::default())),
        }
    }
    /// Emit complete event with [`output`](Result<Output>)
    pub fn complete(&self, id: u64, output: Result<Output, Error>) {
        let waker = {
            let mut inner = self.inner.lock().unwrap();

            inner.completed.insert(id, output);

            inner.wakers.remove(&id)
        };

        if let Some(waker) = waker {
            waker.wake()
        }
    }

    /// Poll response data once.
    ///
    /// # Parameters
    /// - `id` RPC id for [`responder`](Responder<Output>)
    /// - `waker` [`Waker`] of [`responder`](Responder<Output>) [`future`](Future)
    fn poll_once(&self, id: u64, waker: Waker) -> Poll<Result<Output, Error>> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(r) = inner.completed.remove(&id) {
            return Poll::Ready(r);
        }

        inner.wakers.insert(id, waker);

        Poll::Pending
    }

    fn remove_pending_poll(&self, id: u64) {
        self.inner.lock().unwrap().wakers.remove(&id);
    }
}

/// Response poller of one call.
pub struct Response<T, Output, Error> {
    id: u64,
    responder: Responder<Output, Error>,
    timeout: Option<T>,
}

impl<T, Output, Error> Response<T, Output, Error> {
    /// Create new response object
    pub fn new(id: u64, responder: Responder<Output, Error>, timeout: Option<T>) -> Self {
        Response {
            id,
            responder,
            timeout,
        }
    }
}

impl<T, Output, Error> Future for Response<T, Output, Error>
where
    T: Timer + Unpin,
    Error: From<std::io::Error>,
{
    type Output = Result<Output, Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let timer = self.timeout.take();

        if let Some(mut timer) = timer {
            match timer.poll_unpin(cx) {
                Poll::Pending => {
                    self.timeout = Some(timer);
                }
                Poll::Ready(_) => {
                    // Remove pending poll operation .
                    self.responder.remove_pending_poll(self.id);

                    // Return timeout error
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Response timeout",
                    )
                    .into()));
                }
            }
        }

        self.responder.poll_once(self.id, cx.waker().clone())
    }
}
