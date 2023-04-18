use std::thread::spawn;

use async_timer_rs::hashed::Timeout;
use criterion::{async_executor::FuturesExecutor, *};
use futures::{channel::mpsc::Receiver, executor::block_on, StreamExt};

use librpc_json::{
    client::{Client, Responder},
    object::Request,
};

async fn echo(mut receiver: Receiver<(Option<u64>, Vec<u8>)>, responder: Responder) {
    let mut i = 0;

    while let Some((id, msg)) = receiver.next().await {
        i += 1;

        let request: Request<String, String> = serde_json::from_slice(&msg).unwrap();

        let data = serde_json::to_vec(&request.params).unwrap();

        responder.complete(id.unwrap(), Ok(data));
    }

    log::debug!("echo server exit with counter: {}", i)
}

async fn client(mut c: Client) {
    let echo = c
        .call::<String, String, Timeout>("hello", "world".to_string(), None)
        .await
        .unwrap();

    assert_eq!(echo, "world");
}

fn bench_jsonrpc(c: &mut Criterion) {
    _ = pretty_env_logger::try_init();

    let (dispatcher, receiver, responder) = Client::new(100);

    spawn(move || block_on(echo(receiver, responder)));

    c.bench_function("echo jsonrpc", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| client(dispatcher.clone()))
    });

    log::debug!("exit bench_rpc");
}

criterion_group!(benches, bench_jsonrpc);
criterion_main!(benches);
