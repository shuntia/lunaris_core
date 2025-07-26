use std::{fmt::Debug, mem::MaybeUninit, path::PathBuf};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use tokio::sync::mpsc::Sender;

use crate::{mailbox::Endpoint, plugin::host_api::HostApiV1, prelude::*};
pub mod host_api;

/// Uniform Plugin struct that gets registered to every bus.
pub struct Plugin {
    name: String,
    manifest_loc: PathBuf,
    state: PluginState,
    /// Basically a tag to determine the type of plugin.
    plugin_type: PluginType,
    /// Raw pointer. Very dangerous. don't touch unless you know what you're doing.
    raw_ptr: *mut u8,
}

unsafe fn to_raw_plugin<T>(value: T) -> *mut u8 {
    Box::into_raw(Box::new(value)) as *mut u8
}

unsafe fn free_raw<T>(ptr: *mut u8) {
    unsafe { drop(Box::from_raw(ptr as *mut T)) }
}

impl Endpoint for Plugin {
    fn receive(&self, envelope: Envelope) -> NResult {
        match self.plugin_type {
            None => Ok(()),
            PluginType::Sender => *(self.raw_ptr as *mut Sender<Envelope>),
        }
    }
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum PluginState {
    Ready = 0,
    Uninit = 1,
    Busy = 2,
    Unresponsive = 3,
    Killed = 4,
    Dead = 5,
    Error = 6,
}

#[derive(Clone, Copy, IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u32)]
/// Universal enum for Plugin.
pub enum PluginType {
    /// Dummy because why not?
    None = 0,
    /// Plugin where the wrapper takes care of which function to use.
    Callbacks = 1,
    /// Plugin where it just has an endpoint. That's it.
    Sender = 2,
}

pub struct CallbackPlugin {
    /// Manifest Location
    manifest: PathBuf,
    /// Optional init. Introduces the Host API.
    init: InitApiVersion,
    /// Callbacks sorted by op code.
    callbacks: Vec<Option<fn() -> u32>>,
    /// Save call
    save: Option<fn() -> serde_json::Value>,
    /// Load save file
    load: Option<fn(serde_json::Value) -> LunaticError>,
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
