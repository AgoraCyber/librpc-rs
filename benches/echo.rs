use std::{sync::mpsc::Receiver, thread::spawn};

use criterion::{async_executor::FuturesExecutor, *};
use librpc::{dispatcher::Dispatcher, responder::Responder};

fn echo(receiver: Receiver<(u64, String)>, responder: Responder<String>) {
    let mut i = 0;

    loop {
        i += 1;

        match receiver.recv() {
            Err(err) => {
                log::debug!("received: {}, {}", i, err);
                break;
            }
            Ok((id, msg)) => {
                responder.complete(id, Ok(msg));
            }
        }
    }
}

async fn client(dispatcher: Dispatcher<String, String>) {
    let echo = dispatcher.call(0, "hello".to_owned()).await.unwrap();

    assert_eq!(echo, "hello");
}

fn bench_rpc(c: &mut Criterion) {
    _ = pretty_env_logger::try_init();

    let (dispatcher, receiver) = Dispatcher::new(100);

    let responder = dispatcher.responder.clone();

    spawn(move || echo(receiver, responder));

    c.bench_function("echo rpc", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| client(dispatcher.clone()))
    });

    log::debug!("exit bench_rpc");
}

criterion_group!(benches, bench_rpc);
criterion_main!(benches);
