use lunaris_api::plugin::{
    Gui as ApiGui, Plugin as ApiPlugin, PluginContext as ApiPluginContext, PluginReport,
};
use lunaris_api::util::error::Result;

pub trait PluginNode: Send + Sync {
    fn name(&self) -> &'static str;
    fn init(&self, ctx: ApiPluginContext<'_>) -> Result;
    fn update_world(&mut self, ctx: ApiPluginContext<'_>) -> Result;
    fn report(&self, ctx: ApiPluginContext<'_>) -> PluginReport;
    fn shutdown(&mut self, ctx: ApiPluginContext<'_>);
    fn reset(&mut self, ctx: ApiPluginContext<'_>);
    fn register_menu(&self, _menu_bar: &mut lunaris_api::egui::MenuBar) {}
    fn ui(&self, _ui: &mut lunaris_api::egui::Ui, _ctx: ApiPluginContext<'_>) {}
    fn is_gui(&self) -> bool {
        false
    }
}

pub struct CorePluginNode(pub Box<dyn ApiPlugin>);
impl CorePluginNode {
    pub fn new(inner: Box<dyn ApiPlugin>) -> Self {
        Self(inner)
    }
}
impl PluginNode for CorePluginNode {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn init(&self, ctx: ApiPluginContext<'_>) -> Result {
        self.0.init(ctx)
    }
    fn update_world(&mut self, ctx: ApiPluginContext<'_>) -> Result {
        self.0.update_world(ctx)
    }
    fn report(&self, ctx: ApiPluginContext<'_>) -> PluginReport {
        self.0.report(ctx)
    }
    fn shutdown(&mut self, ctx: ApiPluginContext<'_>) {
        self.0.shutdown(ctx)
    }
    fn reset(&mut self, ctx: ApiPluginContext<'_>) {
        self.0.reset(ctx)
    }
    fn register_menu(&self, menu_bar: &mut lunaris_api::egui::MenuBar) {
        self.0.register_menu(menu_bar)
    }
}

pub struct GuiPluginNode(pub Box<dyn ApiGui>);
impl GuiPluginNode {
    pub fn new(inner: Box<dyn ApiGui>) -> Self {
        Self(inner)
    }
}
impl PluginNode for GuiPluginNode {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn init(&self, ctx: ApiPluginContext<'_>) -> Result {
        self.0.init(ctx)
    }
    fn update_world(&mut self, ctx: ApiPluginContext<'_>) -> Result {
        self.0.update_world(ctx)
    }
    fn report(&self, ctx: ApiPluginContext<'_>) -> PluginReport {
        self.0.report(ctx)
    }
    fn shutdown(&mut self, ctx: ApiPluginContext<'_>) {
        self.0.shutdown(ctx)
    }
    fn reset(&mut self, ctx: ApiPluginContext<'_>) {
        self.0.reset(ctx)
    }
    fn register_menu(&self, menu_bar: &mut lunaris_api::egui::MenuBar) {
        self.0.register_menu(menu_bar)
    }
    fn ui(&self, ui: &mut lunaris_api::egui::Ui, ctx: ApiPluginContext<'_>) {
        ApiGui::ui(self.0.as_ref(), ui, ctx)
    }
    fn is_gui(&self) -> bool {
        true
    }
}
