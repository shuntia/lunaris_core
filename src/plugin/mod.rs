use std::path::PathBuf;

use tokio::sync::mpsc::Sender;

use crate::{plugin::host_api::HostApiV1, prelude::*};
mod host_api;

/// Uniform Plugin struct that gets registered to every bus.
pub struct Plugin {
    name: String,
    manifest_version: u32,
    plugin_type: PluginType,
}

pub enum PluginType {
    Callbacks(),
    Mpsc(Sender<Envelope>),
}

pub struct CallbackPlugin {
    /// Manifest Location
    manifest: PathBuf,
    /// Optional init. Introduces the Host API.
    init: InitApiVersion,
    /// Callbacks sorted by op code.
    callbacks: Vec<Option<fn() -> u32>>,
}

pub enum InitApiVersion {
    V1(fn(HostApiV1) -> u32),
}

impl InitApiVersion {
    pub fn call_init(&self) -> u32 {
        match &self {
            Self::V1(f) => f(HostApiV1::new()),
        }
    }
}
