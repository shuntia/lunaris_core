use dashmap::DashMap;
use lunaris_api::bridge::ShareableState;

// --- Type alias for Plugin IDs ---
pub type PluginId = usize;

#[derive(Default)]
pub struct SharedState {
    state: DashMap<PluginId, Box<dyn ShareableState>>,
}

impl SharedState {
    pub fn read<'a>(
        &'a self,
        id: PluginId,
    ) -> Option<dashmap::mapref::one::Ref<'a, PluginId, Box<dyn ShareableState>>> {
        self.state.get(&id)
    }
    pub fn write<'a>(
        &'a self,
        id: PluginId,
    ) -> Option<dashmap::mapref::one::RefMut<'a, PluginId, Box<dyn ShareableState>>> {
        self.state.get_mut(&id)
    }
}
