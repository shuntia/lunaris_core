#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use colored::Colorize;
use lunaris_api::util::error::NResult;
use mimalloc::MiMalloc;
use tracing::*;

use crate::{
    app::LunarisApp,
    logging::{ansi_enabled, init_log_global},
    signals::register_hooks,
};

mod app;
mod consts;
mod dispatcher;
mod linked_plugins;
mod logging;
mod oops;
mod orchestrator;
mod plugin;
mod signals;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

pub fn main() -> NResult {
    init_log_global();
    info!("Initialized logger.");
    info!("Registering signal hooks...");
    register_hooks()?;
    info!("Done.");
    info!("Initializing app...");
    info!(
        "Finished intitialization! {}",
        "Welcome to Lunaris!".cyan().bold()
    );
    match eframe::run_native(
        "Lunaris",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(LunarisApp::default()))),
    ) {
        Ok(o) => info!("UI Exited normally: {o:?}"),
        Err(e) => error!("UI Failed with Error: {e}"),
    };
    info!("{}", "Goodbye!".cyan().bold());
    Ok(())
}
