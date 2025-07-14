#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]

use std::time::Duration;

use tokio::{
    join, spawn,
    sync::mpsc::{Receiver, channel},
    task::spawn_local,
    time::sleep,
};
use tracing::info;

use crate::{
    mailbox::{Endpoint, GLOBAL_MAILBOX, send_global},
    prelude::Envelope,
};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod consts;
mod mailbox;
mod plugin;
mod prelude;
mod protocol;
mod utils;

#[tokio::main]
async fn main() {
    utils::tracing::init_tracing();
    mailbox::init_mailbox().unwrap();
    let (tx, rx) = channel(128);
    GLOBAL_MAILBOX
        .get()
        .unwrap()
        .register(Endpoint::new(tx), "CORE".into());
    let _ = join!(spawn(send_loop()), spawn(watch_loop(rx)));
}

async fn watch_loop(mut rx: Receiver<Envelope>) {
    while let Some(s) = rx.recv().await {
        info!("Envelope received! {:#?}", s);
    }
}

async fn send_loop() {
    loop {
        info!("Sending message!");
        send_global(Envelope::new(
            0,
            0,
            false,
            prelude::Message {
                opcode: 0,
                data: prelude::DataEnum::None,
            },
        ))
        .await
        .unwrap();
        sleep(Duration::from_secs(1)).await;
    }
}
