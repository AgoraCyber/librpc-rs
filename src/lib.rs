pub mod dispatcher;
pub mod responder;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_timer_rs::{hashed::Timeout, Timer};

    use crate::dispatcher::Dispatcher;

    #[futures_test::test]
    async fn test_timeout() {
        let (dispatcher, _receiver) = Dispatcher::<String, String>::new(100);

        let result = dispatcher
            .call(
                0,
                "hello".to_owned(),
                Some(Timeout::new(Duration::from_secs(2))),
            )
            .await;

        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    }
}
