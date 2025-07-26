#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![feature(layout_for_ptr)]

use crate::{
    mailbox::{Endpoint, GLOBAL_MAILBOX, send_global},
    prelude::Envelope,
};

mod consts;
mod mailbox;
mod plugin;
mod prelude;
mod protocol;
mod utils;
use futures::future::{join_all, select};
use std::sync::LazyLock;
use tokio::{
    io::{AsyncWriteExt, stdout},
    join, select,
    sync::mpsc::{Receiver, channel},
};
use tokio::{
    spawn,
    task::spawn_blocking,
    time::{Duration, Instant, sleep},
};
use tracing::{error, info};

static START: LazyLock<Instant> = LazyLock::new(Instant::now);

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    info!("Welcome to Lunatic Studio");
    utils::tracing::init_tracing();
    mailbox::init_mailbox().expect("Failed to initialize mailbox");
    let (tx, rx) = channel(128);
    GLOBAL_MAILBOX
        .get()
        .expect("GLOBAL_MAILBOX not initialized")
        .register(Endpoint::new(tx), "CORE".into());
    let fut1 = spawn(watch_loop(rx));
    let fut2 = spawn(send_loop());
    let fut3 = spawn(send_loop());
    join_all(vec![fut1, fut2, fut3]).await;
    info!("Exiting.");
}

async fn watch_loop(mut rx: Receiver<Envelope>) {
    while let Some(s) = rx.recv().await {
        let id = s.id as f64;
        let dur = Instant::now().duration_since(*START).as_secs_f64();
        /*
        info!(
            "Envelope received!\nEnv id:{:?}, time elapsed: {:?}, Average Env/s: {:?}",
            id,
            dur,
            id as f64 / dur
        );
        */
        stdout()
            .write(format!("Env/s: {:?}\n", id as f64 / dur).as_bytes())
            .await;
    }
    info!("Channel closed, watch_loop exiting");
}

async fn send_loop() {
    let mut message_id = 0;
    loop {
        //info!("Sending message!");
        let envelope = Envelope::new(
            message_id,
            0,
            false,
            prelude::Message {
                opcode: 0,
                data: prelude::DataEnum::None,
            },
        );
        match send_global(envelope).await {
            Ok(()) => {
                message_id += 1;
                //info!("Message sent successfully, id: {}", message_id);
            }
            Err(e) => {
                error!("Failed to send message: {:?}", e);
            }
        }
    }
}
