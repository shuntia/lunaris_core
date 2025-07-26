use arc_swap::ArcSwap;
use futures::executor::block_on;
use parking_lot::Mutex;
use slab::Slab;
use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    sync::{Arc, OnceLock},
};
use tracing::warn;

use crate::{
    plugin::Plugin,
    prelude::{CEnvelope, Envelope, LunaticError, NResult, Result},
};

pub trait Endpoint {
    fn receive(&self, envelope: Envelope) -> NResult;
}

pub struct MailBox {
    /// Lock for write operations
    swap_lock: Mutex<()>,
    /// registry of endpoints
    registry: ArcSwap<Slab<Arc<Plugin>>>,
    /// Strings for resolution
    id: ArcSwap<HashMap<String, u32>>,
}

impl MailBox {
    pub fn register(&self, plugin: Plugin, name: String) -> u32 {
        let _ = self.swap_lock.lock();
        let mut regswap = arc_swap::access::Access::<Slab<_>>::load(&self.registry).clone();
        let id = regswap.insert(Arc::new(plugin)) as u32;
        self.registry.swap(Arc::new(regswap));
        let mut swapid: HashMap<String, u32> =
            arc_swap::access::Access::<HashMap<_, _>>::load(&self.id).clone();
        swapid.insert(name, id);
        self.id.swap(Arc::new(swapid));
        id
    }
    pub fn unregister(&self, id: u32) -> Result<Arc<Plugin>> {
        if !self.registry.load().contains(id as usize) {
            Err(LunaticError::PluginUnloadFailed { id })
        } else {
            let _ = self.swap_lock.lock();
            let mut swapreg = arc_swap::access::Access::<Slab<_>>::load(&self.registry).clone();
            let ret = Ok(swapreg.remove(id as usize));
            self.registry.swap(Arc::new(swapreg));
            ret
        }
    }
    pub async fn send(&self, envelope: Envelope) -> NResult {
        /*let arc_envelope = Arc::new(envelope.clone());
        self.resolve(envelope.destination)?
        .listener
        .iter()
        .for_each(|el| {
            let arc_envelope = arc_envelope.clone();
            tokio::task::spawn_blocking(|| {
                let envelope = arc_envelope;
                el(envelope)
            });
        });*/
        self.registry
            .load()
            .get(envelope.destination as usize)
            .ok_or(LunaticError::PluginNotFound {
                id: envelope.destination,
            })?
            .receive(envelope)
            .await
            .map_err(|send_error| LunaticError::PluginFailedMessage {
                envelope: send_error.0,
            })
    }
    pub fn resolve(&self, id: &str) -> Result<u32> {
        match self.id.load().get(id) {
            Some(s) => Ok(*s),
            None => Err(LunaticError::PluginNameNotFound {
                name: id.to_string(),
            }),
        }
    }
    pub fn new() -> Self {
        Self {
            swap_lock: Mutex::new(()),
            registry: ArcSwap::new(Arc::new(Slab::new())),
            id: ArcSwap::new(Arc::new(HashMap::new())),
        }
    }
    pub fn re_init(&self) {
        let _ = self.swap_lock.lock();
        self.registry.swap(Arc::new(Slab::new()));
        self.id.swap(Arc::new(HashMap::new()));
    }
}

pub static GLOBAL_MAILBOX: OnceLock<MailBox> = OnceLock::new();

pub async fn send_global(msg: Envelope) -> NResult {
    match GLOBAL_MAILBOX.get() {
        Some(s) => s.send(msg).await,
        None => Err(LunaticError::Uninit {
            resource: "lunatic::mailbox::GLOBAL_MAILBOX".to_string(),
        }),
    }
}

pub async fn send_global_async(msg: Envelope) -> NResult {
    send_global(msg).await
}

pub extern "C" fn send_global_c(msg: CEnvelope) -> u32 {
    match block_on(send_global(msg.into())) {
        Ok(_) => 0,
        Err(e) => {
            warn!("Failed to send envelope: {}", e);
            1
        }
    }
}

pub extern "C" fn resolve_global_c(query: *const c_char) -> u32 {
    unsafe {
        if query.is_null() {
            return u32::MAX;
        }
        match CStr::from_ptr(query).to_str() {
            Ok(s) => match resolve_global(s) {
                Ok(o) => o,
                Err(e) => {
                    warn!("Error resolving plugin: {}", e);
                    u32::MAX
                }
            },
            Err(e) => {
                warn!("Invalid string: {}", e);
                u32::MAX
            }
        }
    }
}

pub fn resolve_global(query: &str) -> Result<u32> {
    match GLOBAL_MAILBOX.get() {
        Some(s) => s.resolve(query),
        None => Err(LunaticError::Uninit {
            resource: "GLOBAL_MAILBOX".into(),
        }),
    }
}

pub fn init_mailbox() -> NResult {
    GLOBAL_MAILBOX
        .set(MailBox::new())
        .map_err(|_| LunaticError::KernelInitFailed {
            reason: "You tried to re-init the mailbox! use clear_all()!".into(),
        })
}
