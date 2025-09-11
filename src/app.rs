use bevy_ecs::prelude::*;
use eframe::{
    App,
    egui::{CentralPanel, MenuBar, TopBottomPanel},
};
#[cfg(not(feature = "headless"))]
use egui_tiles::Tree;
use egui_tiles::{Behavior, Tiles};
use lunaris_api::plugin::PluginContext;
use slab::Slab;

use crate::{orchestrator::Orchestrator, plugin::PluginEnum};

pub struct LunarisApp {
    world: World,
    plugins: Slab<PluginEnum>,
    #[cfg(not(feature = "headless"))]
    tree: Tree<PluginEnum>,
    #[cfg(not(feature = "headless"))]
    open_tabs: Vec<usize>,
    orchestrator: Orchestrator,
}

impl Default for LunarisApp {
    fn default() -> Self {
        let mut tiles = Tiles::default();
        let orchestrator = Orchestrator::default();
        let mut world = World::new();
        let mut plugins_vec = PluginEnum::new_all(&mut world, &orchestrator);
        if plugins_vec.is_empty() {
            // no plugins found; keep tree minimal
            plugins_vec = Vec::new();
        }
        let mut tileids = plugins_vec
            .drain(..)
            .map(|el| tiles.insert_pane(el))
            .collect();
        let mut root = tiles.insert_tab_tile(tileids);
        Self {
            world,
            plugins: Slab::from_iter(Vec::new().into_iter()),
            #[cfg(not(feature = "headless"))]
            tree: Tree::new("main_tree", root, tiles),
            #[cfg(not(feature = "headless"))]
            open_tabs: vec![0],
            orchestrator,
        }
    }
}

impl LunarisApp {
    fn test(&self) {
        for i in &self.plugins {
            println!("{}", i.1.name());
        }
    }
    fn get_ctx(&mut self) -> PluginContext<'_> {
        PluginContext {
            world: &mut self.world,
            orch: &self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
        }
    }
}

impl App for LunarisApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        // If the OS/window requested close, do minimal work and exit frame.
        if !ctx.input(|i| i.viewport().close_requested()) {
            // Avoid borrowing `self` mutably twice by creating a lightweight behavior
            // that only borrows the fields needed by the Behavior impl.
            #[cfg(not(feature = "headless"))]
            {
                struct AppBehavior<'a> {
                    world: &'a mut World,
                    orchestrator: &'a Orchestrator,
                    // Defer tab insertion until after tree.ui to avoid borrow issues
                    pending_add: Option<(egui_tiles::TileId, &'static str)>,
                }

                impl<'a> Behavior<PluginEnum> for AppBehavior<'a> {
                    fn pane_ui(
                        &mut self,
                        ui: &mut eframe::egui::Ui,
                        _tile_id: egui_tiles::TileId,
                        pane: &mut PluginEnum,
                    ) -> egui_tiles::UiResponse {
                        let ctx = PluginContext {
                            world: self.world,
                            orch: self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
                        };
                        pane.ui(ui, ctx);
                        egui_tiles::UiResponse::None
                    }
                    fn tab_title_for_pane(
                        &mut self,
                        pane: &PluginEnum,
                    ) -> eframe::egui::WidgetText {
                        pane.name().into()
                    }
                    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
                        egui_tiles::SimplificationOptions {
                            prune_empty_tabs: true,
                            prune_empty_containers: true,
                            prune_single_child_tabs: true,
                            prune_single_child_containers: false,
                            all_panes_must_have_tabs: true,
                            join_nested_linear_containers: false,
                        }
                    }
                    fn is_tab_closable(
                        &self,
                        _tiles: &egui_tiles::Tiles<PluginEnum>,
                        _tile_id: egui_tiles::TileId,
                    ) -> bool {
                        true
                    }

                    fn top_bar_right_ui(
                        &mut self,
                        _tiles: &egui_tiles::Tiles<PluginEnum>,
                        ui: &mut eframe::egui::Ui,
                        tab_container_id: egui_tiles::TileId,
                        _tabs: &egui_tiles::Tabs,
                        _scroll_offset: &mut f32,
                    ) {
                        use lunaris_api::plugin::GuiRegistration;
                        ui.menu_button("＋", |ui| {
                            ui.set_min_width(220.0);
                            ui.label("Add plugin tab");
                            ui.separator();
                            for reg in inventory::iter::<GuiRegistration> {
                                if ui.button(reg.name).clicked() {
                                    self.pending_add = Some((tab_container_id, reg.name));
                                    ui.close_menu();
                                }
                            }
                            if ui.button("Close").clicked() {
                                ui.close_menu();
                            }
                        });
                    }
                }

                let mut behavior = AppBehavior {
                    world: &mut self.world,
                    orchestrator: &self.orchestrator,
                    pending_add: None,
                };

                let mut skip_body = false;
                TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                    MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Quit").clicked() {
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                                skip_body = true;
                            }
                        });
                    });
                });
                if !skip_body {
                    CentralPanel::default().show(ctx, |ui| self.tree.ui(&mut behavior, ui));

                    // Apply any pending tab insertions after the UI pass
                    if let Some((tabs_id, name)) = behavior.pending_add.take() {
                        // Find matching GUI registration by name and build it
                        if let Some(reg) = inventory::iter::<lunaris_api::plugin::GuiRegistration>
                            .into_iter()
                            .find(|r| r.name == name)
                        {
                            let mut pane = PluginEnum::PluginGui((reg.build)());
                            // Initialize the plugin
                            let ctx = PluginContext {
                                world: &mut self.world,
                                orch: &self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
                            };
                            let _ = pane.init(ctx);

                            // Insert pane and add to the tabs container
                            let new_id = self.tree.tiles.insert_pane(pane);
                            if let Some(egui_tiles::Tile::Container(container)) =
                                self.tree.tiles.get_mut(tabs_id)
                            {
                                if let egui_tiles::Container::Tabs(tabs) = container {
                                    tabs.add_child(new_id);
                                    tabs.set_active(new_id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Drop for LunarisApp {
    fn drop(&mut self) {
        // Best-effort: allow any foreground jobs to finish before tearing down workers.
        let _ = self.orchestrator.join_foreground();
    }
}
