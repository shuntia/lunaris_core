use lunaris_api::plugin::{GuiRegistration, PluginRegistration};
use lunaris_api::plugin::{Plugin as ApiPlugin, Gui as ApiGui, PluginGui as ApiPluginGui, PluginContext as ApiPluginContext, PluginReport};
use bevy_ecs::prelude::World;
use crate::orchestrator::Orchestrator;

pub enum PluginEnum {
    Plugin(Box<dyn ApiPlugin>),
    PluginGui(Box<dyn ApiPluginGui>),
}

impl PluginEnum {
    pub fn new_all(world: &mut World, orch: &Orchestrator) -> Vec<PluginEnum> {
        // Prefer GUI constructors when available (they also implement Plugin)
        let mut out: Vec<PluginEnum> = Vec::new();
        let mut gui_names = std::collections::HashSet::new();
        for reg in inventory::iter::<GuiRegistration> {
            gui_names.insert(reg.name);
            out.push(PluginEnum::PluginGui((reg.build)()));
        }
        for reg in inventory::iter::<PluginRegistration> {
            if !gui_names.contains(reg.name) {
                out.push(PluginEnum::Plugin((reg.build)()));
            }
        }
        // Optionally initialize immediately
        for p in out.iter_mut() {
            let ctx = ApiPluginContext { world, orch: orch as &dyn lunaris_api::request::DynOrchestrator };
            match p {
                PluginEnum::Plugin(p) => { let _ = p.init(ctx); },
                PluginEnum::PluginGui(p) => { let _ = p.init(ctx); },
            }
        }
        out
    }

    pub fn name(&self) -> &'static str {
        match self {
            PluginEnum::Plugin(p) => p.name(),
            PluginEnum::PluginGui(p) => p.name(),
        }
    }

    pub fn init(&self, ctx: ApiPluginContext<'_>) -> lunaris_api::util::error::NResult {
        match self {
            PluginEnum::Plugin(p) => p.init(ctx),
            PluginEnum::PluginGui(p) => p.init(ctx),
        }
    }
    pub fn update_world(&mut self, ctx: ApiPluginContext<'_>) -> lunaris_api::util::error::NResult {
        match self {
            PluginEnum::Plugin(p) => p.update_world(ctx),
            PluginEnum::PluginGui(p) => p.update_world(ctx),
        }
    }
    pub fn report(&self, ctx: ApiPluginContext<'_>) -> PluginReport {
        match self {
            PluginEnum::Plugin(p) => p.report(ctx),
            PluginEnum::PluginGui(p) => p.report(ctx),
        }
    }
    pub fn shutdown(&mut self, ctx: ApiPluginContext<'_>) {
        match self {
            PluginEnum::Plugin(p) => p.shutdown(ctx),
            PluginEnum::PluginGui(p) => p.shutdown(ctx),
        }
    }
    pub fn reset(&mut self, ctx: ApiPluginContext<'_>) {
        match self {
            PluginEnum::Plugin(p) => p.reset(ctx),
            PluginEnum::PluginGui(p) => p.reset(ctx),
        }
    }
    pub fn register_menu(&self, menu_bar: &mut lunaris_api::egui::MenuBar) {
        match self {
            PluginEnum::Plugin(p) => p.register_menu(menu_bar),
            PluginEnum::PluginGui(p) => p.register_menu(menu_bar),
        }
    }
    pub fn ui(&self, ui: &mut lunaris_api::egui::Ui, ctx: ApiPluginContext<'_>) {
        match self {
            PluginEnum::Plugin(_) => {}
            PluginEnum::PluginGui(p) => ApiGui::ui(p.as_ref(), ui, ctx),
        }
    }
}
