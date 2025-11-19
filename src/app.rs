use eframe::{
    App,
    egui::{CentralPanel, MenuBar, TopBottomPanel},
};
use egui_tiles::{Behavior, Tiles, Tree};
use futures::channel::mpsc;
use lunaris_api::plugin::{GuiRegistration, PluginContext};
use lunaris_ecs::prelude::*;
use slab::Slab;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use crate::{
    bridge::SharedState,
    orchestrator::Orchestrator,
    plugin::{GuiPluginNode, PluginNode},
};

type PluginId = usize;

// --- Data structures for cross-thread communication ---

/// Commands sent from the UI thread to the World thread.
enum WorldCommand {
    Quit,
    // Add other commands here, e.g., for user interactions
}

pub struct LunarisApp {
    /// Handle to the dedicated world thread.
    world_thread: Option<JoinHandle<()>>,
    /// Sender to send commands to the world thread.
    command_sender: mpsc::Sender<WorldCommand>,

    // The following fields are purely for the UI and are managed only on the UI thread.
    plugins: Slab<Box<dyn PluginNode>>,
    tree: Tree<PluginId>,
    gui_index_by_name: HashMap<&'static str, PluginId>,
    last_tab_container_id: Option<egui_tiles::TileId>,
}

impl Default for LunarisApp {
    fn default() -> Self {
        let (command_sender, mut command_receiver) = mpsc::channel(8);
        let ui_state = Arc::new(RwLock::new(SharedState::default()));
        let ui_state_clone = ui_state.clone();

        // --- Spawn the dedicated World thread ---
        let world_thread = thread::spawn(move || {
            let mut world = World::new();
            let mut schedule = Schedule::default();

            // --- Initialize World Resources ---
            world.insert_resource(Orchestrator::default());

            // --- Main World Loop ---
            loop {
                // Check for commands from the UI thread
                match command_receiver.try_next() {
                    Ok(Some(WorldCommand::Quit)) => {
                        println!("World thread received quit command.");
                        break;
                    }
                    Ok(None) => {
                        // Channel closed, should also quit
                        break;
                    }
                    _ => {}
                }

                // Run all systems in the schedule!
                schedule.run(&mut world);

                // Update the shared UI state for the next frame
                if let Ok(state) = ui_state_clone.write() {
                    // e.g., state.some_value = world.get_resource::<MyResource>().unwrap().some_value;
                }

                // Sleep to prevent busy-looping and yield CPU time
                thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
            }
        });

        // --- Initialize UI-specific state ---
        let mut tiles: Tiles<PluginId> = Tiles::default();
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

        let tileids: Vec<_> = gui_ids
            .iter()
            .copied()
            .map(|id| tiles.insert_pane(id))
            .collect();
        let root = tiles.insert_tab_tile(tileids);

        Self {
            world_thread: Some(world_thread),
            command_sender,
            plugins,
            tree: Tree::new("main_tree", root, tiles),
            gui_index_by_name,
            last_tab_container_id: None,
        }
    }
}

// The AppBehavior now needs to be adapted to the new architecture.
// For now, we'll pass dummy data to the plugins' UI methods.
struct AppBehavior<'a> {
    plugins: &'a mut Slab<Box<dyn PluginNode>>,
    // We no longer have direct access to the World or Orchestrator here.
}

impl<'a> Behavior<PluginId> for AppBehavior<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut eframe::egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut PluginId,
    ) -> egui_tiles::UiResponse {
        // This is a temporary solution. A proper implementation would require
        // the UI plugins to get their state from the `SharedUiState`.
        let dummy_world = &mut World::new();
        let dummy_orch = &Orchestrator::default() as &dyn lunaris_api::request::DynOrchestrator;
        let ctx = PluginContext {
            world: dummy_world,
            orch: dummy_orch,
        };

        if let Some(p) = self.plugins.get(*pane) {
            p.ui(ui, ctx);
        }
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &PluginId) -> eframe::egui::WidgetText {
        self.plugins
            .get(*pane)
            .map_or("<missing>".into(), |p| p.name().into())
    }

    // ... other Behavior methods can be simplified as they don't have world access ...
    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<PluginId>,
        ui: &mut eframe::egui::Ui,
        _tab_container_id: egui_tiles::TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        // This functionality would need to be re-thought. Adding a tab would
        // now involve sending a command to the world thread.
    }
}

impl App for LunarisApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // The UI thread is now much simpler. It just draws the UI.
        // The complex logic and system updates are all happening in the background.

        if ctx.input(|i| i.viewport().close_requested()) {
            // When the user tries to close the window, send the Quit command.
            if let Some(thread) = self.world_thread.take() {
                self.command_sender.try_send(WorldCommand::Quit).ok();
                thread.join().expect("World thread panicked!");
            }
            // Actually close the window now that the thread is joined.
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        }

        let mut behavior = AppBehavior {
            plugins: &mut self.plugins,
        };

        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        // This will trigger the close sequence on the next frame.
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                    }
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| self.tree.ui(&mut behavior, ui));
    }
}

impl Drop for LunarisApp {
    fn drop(&mut self) {
        // Ensure the world thread is shut down cleanly when the app is dropped.
        if let Some(thread) = self.world_thread.take() {
            self.command_sender.try_send(WorldCommand::Quit).ok();
            thread.join().expect("World thread panicked!");
        }
    }
}
