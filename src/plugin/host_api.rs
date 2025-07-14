use std::{ffi::c_char, pin::Pin};

use crate::{
    consts::{self, VERSION},
    mailbox::{resolve_global, resolve_global_c, send_global, send_global_c},
    prelude::*,
};

#[repr(C)]
pub struct HostCApiV1 {
    /// API Version.
    pub version: u32,
    /// Log contents.
    pub log: extern "C" fn(*const c_char, *const c_char, u8) -> u32,
    /// Sender to mailbox.
    pub sender: extern "C" fn(CEnvelope) -> u32,
    /// Resolve the name of the plugin into a u32 bus address.
    pub resolve: extern "C" fn(*const c_char) -> u32,
}

impl HostCApiV1 {
    pub fn new() -> Self {
        Self {
            version: 1,
            log: crate::utils::tracing::log_c,
            sender: send_global_c,
            resolve: resolve_global_c,
        }
    }
}

pub struct HostApiV1 {
    /// API Version.
    pub version: u32,
}

impl HostApiV1 {
    fn resolve(query: &str) -> Result<u32> {
        resolve_global(query)
    }
    async fn send(envelope: Envelope) -> NResult {
        send_global(envelope).await
    }
}

impl HostApiV1 {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}
