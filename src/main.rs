#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_ecs::{schedule::Schedule, world::World};
use colored::Colorize;
use futures::executor::block_on;
use lunaris_api::{render, util::error::Result};
use mimalloc::MiMalloc;
use tracing::*;

// Force linking of statically-registered plugins via the linker crate.
// This ensures inventory submissions (e.g., GUI plugins like Profiler) are discovered.
#[allow(unused_imports)]
use linker as _;
use wgpu::{DeviceDescriptor, Instance, RequestAdapterOptions};

use crate::{
    app::LunarisApp, logging::init_log_global, orchestrator::Orchestrator, signals::register_hooks,
};

/// Things related to the main Lunaris UI and app.
/// Everything `egui` is mainly contained in this module.
mod app;
mod bridge;
mod consts;
mod dispatcher;
mod logging;
mod oops;
mod orchestrator;
mod plugin;
mod signals;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

pub fn main() -> Result {
    init_log_global();
    info!("Initialized logger.");
    info!("Registering signal hooks...");
    register_hooks()?;
    info!("Done.");
    info!("Initializing app...");
    debug!("Preparing GPU resources...");
    let (device, queue) = block_on(async {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to initialize the GPU instance.");

        adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("Failed to fetch GPU Device.")
    });
    render::init_gpu(device, queue)?;
    debug!("GPU resources successfully initialized!");
    debug!("Preparing ECS and runtime state...");
    let mut world = World::new();
    let sched = Schedule::default();
    debug!("ECS state ready to launch!");
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
