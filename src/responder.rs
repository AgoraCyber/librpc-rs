//! RPC response associate types.

use std::{
    collections::HashMap,
    future::Future,
    io::Result,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use async_timer_rs::Timer;
use futures::FutureExt;

#[derive(Debug)]
struct DispatcherImpl<Output> {
    wakers: HashMap<u64, Waker>,
    completed: HashMap<u64, Result<Output>>,
}

impl<Output> Default for DispatcherImpl<Output> {
    fn default() -> Self {
        Self {
            wakers: HashMap::new(),
            completed: HashMap::new(),
        }
    }
}

/// Rpc message dispatcher.
#[derive(Debug)]
pub struct Responder<Output> {
    inner: Arc<Mutex<DispatcherImpl<Output>>>,
}

impl<Output> Clone for Responder<Output> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<Output> Responder<Output> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DispatcherImpl::default())),
        }
    }
    /// Emit complete event with [`output`](Result<Output>)
    pub fn complete(&self, id: u64, output: Result<Output>) {
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
    fn poll_once(&self, id: u64, waker: Waker) -> Poll<Result<Output>> {
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
pub struct ResponsePoller<T, Output> {
    id: u64,
    responder: Responder<Output>,
    timeout: Option<T>,
}

impl<T, Output> ResponsePoller<T, Output> {
    /// Create new response object
    pub fn new(id: u64, responder: Responder<Output>, timeout: Option<T>) -> Self {
        ResponsePoller {
            id,
            responder,
            timeout,
        }
    }
}

impl<T, Output> Future for ResponsePoller<T, Output>
where
    T: Timer + Unpin,
{
    type Output = Result<Output>;

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
                    )));
                }
            }
        }

        self.responder.poll_once(self.id, cx.waker().clone())
    }
}

/// Response error for one call.
pub struct ResponseError<Output> {
    err: Option<std::io::Error>,
    _marked: PhantomData<Output>,
}

impl<Output> ResponseError<Output> {
    /// Create new response object
    pub fn new(err: std::io::Error) -> Self {
        ResponseError {
            err: Some(err),
            _marked: Default::default(),
        }
    }
}

impl<Output> Future for ResponseError<Output>
where
    Output: Unpin,
{
    type Output = Result<Output>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        Poll::Ready(Err(self.get_mut().err.take().unwrap()))
    }
}

/// Future for response
pub enum Response<T, Output> {
    Poller(ResponsePoller<T, Output>),
    Err(ResponseError<Output>),
}

impl<T, Output> Response<T, Output> {
    /// Create new poller response object
    pub fn poller(id: u64, responder: Responder<Output>, timeout: Option<T>) -> Self {
        Self::Poller(ResponsePoller {
            id,
            responder,
            timeout,
        })
    }

    /// Create new error response object
    pub fn error(err: std::io::Error) -> Self {
        Self::Err(ResponseError {
            err: Some(err),
            _marked: Default::default(),
        })
    }
}

impl<T, Output> Future for Response<T, Output>
where
    Output: Unpin,
    T: Timer + Unpin,
{
    type Output = Result<Output>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            Self::Err(err) => Pin::new(err).poll(cx),
            Self::Poller(poller) => Pin::new(poller).poll(cx),
        }
    }
}
