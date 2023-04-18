use std::thread::spawn;

use async_timer_rs::hashed::Timeout;
use criterion::{async_executor::FuturesExecutor, *};
use futures::{channel::mpsc::Receiver, executor::block_on, StreamExt};
use librpc::{dispatcher::Dispatcher, responder::Responder};

async fn echo(mut receiver: Receiver<(u64, String)>, responder: Responder<String>) {
    let mut i = 0;

    while let Some((id, msg)) = receiver.next().await {
        i += 1;

        responder.complete(id, Ok(msg));
    }

    log::debug!("echo server exit with counter: {}", i)
}

async fn client(mut dispatcher: Dispatcher<String, String>) {
    let echo = dispatcher
        .call::<Timeout>(0, "hello".to_owned(), None)
        .await
        .unwrap()
        .await
        .unwrap();

    assert_eq!(echo, "hello");
}

fn bench_rpc(c: &mut Criterion) {
    _ = pretty_env_logger::try_init();

    let (dispatcher, receiver) = Dispatcher::new(100);

    let responder = dispatcher.responder.clone();

    spawn(move || block_on(echo(receiver, responder)));

    c.bench_function("echo rpc", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| client(dispatcher.clone()))
    });

    log::debug!("exit bench_rpc");
}

criterion_group!(benches, bench_rpc);
criterion_main!(benches);
