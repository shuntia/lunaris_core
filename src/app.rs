use bevy_ecs::prelude::*;
use eframe::{
    App,
    egui::{CentralPanel, Ui},
};
use egui_tiles::{Behavior, Tree};
use lunaris_api::plugin::{Gui, Plugin, PluginContext};
use slab::Slab;

use crate::{orchestrator::Orchestrator, plugin::PluginEnum};

pub struct LunarisApp {
    world: World,
    plugins: Slab<PluginEnum>,
    open_tabs: Vec<usize>,
    orchestrator: Orchestrator,
}

impl Default for LunarisApp {
    fn default() -> Self {
        let enabled = vec!["testing::TestPlugin"];
        Self {
            world: World::new(),
            plugins: Slab::from_iter(enabled.iter().map(|el| PluginEnum::new(*el)).enumerate()),
            open_tabs: vec![0],
            orchestrator: Orchestrator::default(),
        }
    }
}

impl LunarisApp {
    fn test(&self) {
        for i in &self.plugins {
            println!("{}", i.1.name());
        }
    }
    fn ui(ui: &mut Ui) {}
    fn get_ctx(&mut self) -> PluginContext {
        PluginContext {
            world: &mut self.world,
        }
    }
}

impl App for LunarisApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, Self::ui);
    }
}

impl Behavior<PluginEnum> for LunarisApp {
    fn pane_ui(
        &mut self,
        ui: &mut eframe::egui::Ui,
        tile_id: egui_tiles::TileId,
        pane: &mut PluginEnum,
    ) -> egui_tiles::UiResponse {
        pane.ui(ui, self.get_ctx());
        egui_tiles::UiResponse::None
    }
    fn tab_title_for_pane(&mut self, pane: &PluginEnum) -> eframe::egui::WidgetText {
        pane.name().into()
    }
}
