use bevy_ecs::prelude::*;
use eframe::{
    App,
    egui::{CentralPanel, MenuBar, TopBottomPanel},
};
use egui_tiles::{Behavior, Tiles, Tree};
use lunaris_api::plugin::{GuiRegistration, PluginContext};
use slab::Slab;
use std::collections::{HashMap, HashSet};

use crate::{
    orchestrator::Orchestrator,
    plugin::{CorePluginNode, GuiPluginNode, PluginNode},
};

type PluginId = usize;

#[derive(Clone, Copy, Debug)]
enum PaneRef {
    Core(PluginId),
    Gui(PluginId),
}

pub struct LunarisApp {
    world: World,
    plugins: Slab<Box<dyn PluginNode>>,
    #[cfg(not(feature = "headless"))]
    tree: Tree<PaneRef>,
    #[cfg(not(feature = "headless"))]
    open_tabs: Vec<usize>,
    orchestrator: Orchestrator,
    #[cfg(not(feature = "headless"))]
    gui_index_by_name: HashMap<&'static str, PluginId>,
    #[cfg(not(feature = "headless"))]
    last_tab_container_id: Option<egui_tiles::TileId>,
}

impl Default for LunarisApp {
    fn default() -> Self {
        let mut tiles: Tiles<PaneRef> = Tiles::default();
        let orchestrator = Orchestrator::default();
        let mut world = World::new();

        let mut plugins: Slab<Box<dyn PluginNode>> = Slab::new();
        let mut gui_index_by_name: HashMap<&'static str, PluginId> = HashMap::new();
        let mut gui_names: HashSet<&'static str> = HashSet::new();
        let mut gui_ids: Vec<PluginId> = Vec::new();

        for reg in inventory::iter::<GuiRegistration> {
            let id = plugins.insert(Box::new(GuiPluginNode::new((reg.build)())));
            gui_index_by_name.insert(reg.name, id);
            gui_names.insert(reg.name);
            gui_ids.push(id);
        }
        for reg in inventory::iter::<lunaris_api::plugin::PluginRegistration> {
            if !gui_names.contains(reg.name) {
                let _ = plugins.insert(Box::new(CorePluginNode::new((reg.build)())));
            }
        }

        // Initialize all plugins
        for (_, p) in plugins.iter() {
            let ctx = PluginContext {
                world: &mut world,
                orch: &orchestrator as &dyn lunaris_api::request::DynOrchestrator,
            };
            let _ = p.init(ctx);
        }

        // Initial panes from GUI plugins
        let tileids: Vec<_> = gui_ids
            .iter()
            .copied()
            .map(|id| tiles.insert_pane(PaneRef::Gui(id)))
            .collect();
        let root = tiles.insert_tab_tile(tileids);

        Self {
            world,
            plugins,
            #[cfg(not(feature = "headless"))]
            tree: Tree::new("main_tree", root, tiles),
            #[cfg(not(feature = "headless"))]
            open_tabs: vec![0],
            orchestrator,
            #[cfg(not(feature = "headless"))]
            gui_index_by_name,
            #[cfg(not(feature = "headless"))]
            last_tab_container_id: None,
        }
    }
}

impl LunarisApp {
    fn get_ctx(&mut self) -> PluginContext<'_> {
        PluginContext {
            world: &mut self.world,
            orch: &self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
        }
    }
}

impl App for LunarisApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Update world via plugins (foreground)
        {
            let indices: Vec<usize> = self.plugins.iter().map(|(i, _)| i).collect();
            for id in indices {
                if let Some(p) = self.plugins.get_mut(id) {
                    let _ = p.update_world(PluginContext {
                        world: &mut self.world,
                        orch: &self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
                    });
                }
            }
        }

        if !ctx.input(|i| i.viewport().close_requested()) {
            #[cfg(not(feature = "headless"))]
            {
                struct AppBehavior<'a> {
                    world: &'a mut World,
                    orchestrator: &'a Orchestrator,
                    plugins: &'a mut Slab<Box<dyn PluginNode>>,
                    pending_add: Option<(egui_tiles::TileId, &'static str)>,
                    last_tabs_local: Option<egui_tiles::TileId>,
                }

                impl<'a> Behavior<PaneRef> for AppBehavior<'a> {
                    fn pane_ui(
                        &mut self,
                        ui: &mut eframe::egui::Ui,
                        _tile_id: egui_tiles::TileId,
                        pane: &mut PaneRef,
                    ) -> egui_tiles::UiResponse {
                        let ctx = PluginContext {
                            world: self.world,
                            orch: self.orchestrator as &dyn lunaris_api::request::DynOrchestrator,
                        };
                        if let PaneRef::Core(id) | PaneRef::Gui(id) = *pane
                            && let Some(p) = self.plugins.get(id)
                        {
                            p.ui(ui, ctx)
                        }
                        egui_tiles::UiResponse::None
                    }
                    fn tab_title_for_pane(&mut self, pane: &PaneRef) -> eframe::egui::WidgetText {
                        if let PaneRef::Core(id) | PaneRef::Gui(id) = *pane
                            && let Some(p) = self.plugins.get(id)
                        {
                            return p.name().into();
                        }
                        "<missing>".into()
                    }
                    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
                        egui_tiles::SimplificationOptions {
                            prune_empty_tabs: false,
                            prune_empty_containers: true,
                            prune_single_child_tabs: true,
                            prune_single_child_containers: false,
                            all_panes_must_have_tabs: true,
                            join_nested_linear_containers: false,
                        }
                    }
                    fn is_tab_closable(
                        &self,
                        _tiles: &egui_tiles::Tiles<PaneRef>,
                        _tile_id: egui_tiles::TileId,
                    ) -> bool {
                        true
                    }
                    fn top_bar_right_ui(
                        &mut self,
                        _tiles: &egui_tiles::Tiles<PaneRef>,
                        ui: &mut eframe::egui::Ui,
                        tab_container_id: egui_tiles::TileId,
                        _tabs: &egui_tiles::Tabs,
                        _scroll_offset: &mut f32,
                    ) {
                        // Remember the last seen tab container so global menu can target it (next frame)
                        self.last_tabs_local = Some(tab_container_id);
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
                        });
                    }
                }

                let mut behavior = AppBehavior {
                    world: &mut self.world,
                    orchestrator: &self.orchestrator,
                    plugins: &mut self.plugins,
                    pending_add: None,
                    last_tabs_local: None,
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
                        ui.menu_button("Tabs", |ui| {
                            ui.set_min_width(220.0);
                            ui.label("Add plugin tab");
                            ui.separator();
                            for (name, id) in self.gui_index_by_name.iter() {
                                if ui.button(*name).clicked() {
                                    if let Some(tabs_id) = self.last_tab_container_id {
                                        let new_id = self.tree.tiles.insert_pane(PaneRef::Gui(*id));
                                        if let Some(egui_tiles::Tile::Container(container)) =
                                            self.tree.tiles.get_mut(tabs_id)
                                            && let egui_tiles::Container::Tabs(tabs) = container
                                        {
                                            tabs.add_child(new_id);
                                            tabs.set_active(new_id);
                                        }
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                    });
                });
                if !skip_body {
                    CentralPanel::default().show(ctx, |ui| self.tree.ui(&mut behavior, ui));
                    // Persist last seen container id for Tabs menu to use on next frame
                    if let Some(id) = behavior.last_tabs_local.take() {
                        self.last_tab_container_id = Some(id);
                    }

                    if let Some((tabs_id, name)) = behavior.pending_add.take()
                        && let Some(&id) = self.gui_index_by_name.get(name)
                    {
                        let new_id = self.tree.tiles.insert_pane(PaneRef::Gui(id));
                        if let Some(egui_tiles::Tile::Container(container)) =
                            self.tree.tiles.get_mut(tabs_id)
                            && let egui_tiles::Container::Tabs(tabs) = container
                        {
                            tabs.add_child(new_id);
                            tabs.set_active(new_id);
                        }
                    }
                }
            }
        }
    }
}

impl Drop for LunarisApp {
    fn drop(&mut self) {
        let _ = self.orchestrator.join_foreground();
    }
}
