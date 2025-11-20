use lunaris_api::plugin::DynPlugin;

pub struct PluginRegistry {
    inner: Vec<PluginEntry>,
}

pub struct PluginEntry {
    inner: Box<dyn DynPlugin>,
}
