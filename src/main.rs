#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::App;

use crate::app::LunarisApp;

mod app;
mod consts;
mod orchestrator;
mod plugin;

pub fn main() {
    let mut app = LunarisApp::default();
    eframe::run_simple_native(
        "Lunaris",
        eframe::NativeOptions::default(),
        move |ctx, frame| app.update(ctx, frame),
    )
    .unwrap();
}
