pub mod dispatcher;
pub mod responder;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_timer_rs::{hashed::Timeout, Timer};
    use futures::channel::mpsc::SendError;
    use thiserror::Error;

    use crate::dispatcher::Dispatcher;

    #[derive(Debug, Error)]
    enum TestError {
        #[error(transparent)]
        SendError(#[from] SendError),

        #[error(transparent)]
        IO(#[from] std::io::Error),
    }

    #[futures_test::test]
    async fn test_timeout() {
        let (mut dispatcher, _receiver) = Dispatcher::<String, String, TestError>::new(100);

        dispatcher
            .call(
                0,
                "hello".to_owned(),
                Some(Timeout::new(Duration::from_secs(2))),
            )
            .await
            .unwrap()
            .await
            .expect_err("Timeout expect");
    }
}
