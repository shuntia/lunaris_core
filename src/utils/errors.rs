use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::prelude::Envelope;

pub type NResult = core::result::Result<(), LunaticError>;
pub type Result<T> = core::result::Result<T, LunaticError>;
#[derive(Debug, Error)]
pub enum LunaticError {
    /// Generic error.
    /// Refrain from using as much as possible.
    #[error("Unknown error occurred: {context:?}")]
    Unknown { context: Option<String> },

    /// Tried to use feature that was not implemented.
    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: &'static str },

    /// Tried  to invoke a command with wrong arguments.
    #[error("Invalid argument: {name} - {reason:?}")]
    InvalidArgument {
        name: String,
        reason: Option<String>,
    },

    /// Resource not initialized yet.
    #[error("Tried to access resource which was not initialized: {resource}")]
    Uninit { resource: String },

    /// Found a null pointer.
    /// This is pretty bad.
    #[error("Null pointer at {location}")]
    NullPointer { location: &'static str },

    /// Out of memory. Self-explanatory.
    #[error("Out of memory")]
    OutOfMemory,

    /// Command timed out.
    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// some sort of interrupt
    #[error("Interrupted during {during}")]
    Interrupted { during: &'static str },

    /// Tried to access a resource but was denied.
    #[error("Resource busy: {resource}")]
    Busy { resource: String },

    /// Tried to do an operation but failed.
    #[error("Permission denied for operation: {operation}")]
    PermissionDenied { operation: String },

    /// Tried to do an operation that lacked support.
    #[error("Operation not supported: {operation}")]
    NotSupported { operation: &'static str },

    /// Tried to create a duplicate item
    #[error("Item already exists: {item}")]
    AlreadyExists { item: String },

    /// Tried to find resource from somewhere but failed.
    #[error("Item not found: {item}")]
    NotFound { item: String },

    // Mailbox / IPC errors
    /// Mailbox failed to connnect.
    #[error("Mailbox disconnected")]
    MailboxDisconnected,

    /// the mailbox mpsc is full.
    #[error("Mailbox is full")]
    MailboxFull,

    /// Tried to fetch contents from an empty mailbox.
    #[error("Mailbox is empty")]
    MailboxEmpty,

    /// kernel message was rejected.
    #[error("Invalid message format: {reason}")]
    InvalidMessageFormat { reason: String },

    /// Envelope failed to send.
    #[error("Invalid envelope, expected: {expected}")]
    InvalidEnvelope { expected: String },

    /// Message size was too large to hold. Maybe try to split it up.
    #[error("Message too large: {size} bytes")]
    MessageTooLarge { size: usize },

    /// Failed to find destination for envelope.
    #[error("Invalid destination ID: {id}")]
    InvalidDestination { id: u32 },

    /// Tried to get acknowledgement from a plugin, but failed.
    #[error("Acknowledgment timeout for opcode {opcode} from {src}")]
    AckTimeout { opcode: u32, src: u32 },

    // Kernel/System-level errors
    /// Failed to initialize kernel. Very bad news.
    #[error("Kernel initialization failed: {reason}")]
    KernelInitFailed { reason: String },

    /// Kernel internally paniced. Not a plugin's fault(probably)
    #[error("Kernel panic: {reason}")]
    KernelPanic { reason: String },

    /// Kernel contents are off. Maybe something is poking at its memory in a bad way.
    #[error("Invalid state: expected {expected}, found {found}")]
    InvalidState { expected: String, found: String },

    /// Found a deadlock. Will try to kill that command.
    #[error("Deadlock detected in {component}")]
    DeadlockDetected { component: String },

    /// Tried to shut down while shutting down.
    #[error("Shutdown already in progress")]
    ShutdownInProgress,

    // Renderer-related errors
    /// renderer failed to initialize
    #[error("Renderer initialization failed: {reason}")]
    RenderInitFailed { reason: String },

    /// GPU device or CPU device failed.
    #[error("Render device lost")]
    RenderDeviceLost,

    /// Ran out of VRAM.
    #[error("Render ran out of memory")]
    RenderOutOfMemory,

    /// Too much rendering queue contents.
    #[error("Render queue is full")]
    RenderQueueFull,

    /// Took too much time to render.
    #[error("Render timeout during: {stage}")]
    RenderTimeout { stage: &'static str },

    // Plugin loader/runtime issues
    #[error("Could not send message to plugin.")]
    PluginFailedMessage { envelope: Envelope },

    /// Plugin was not loaded?
    #[error("Could not find plugin with id: {id}")]
    PluginNotFound { id: u32 },

    #[error("Could not find plugin with name: {name}")]
    PluginNameNotFound { name: String },

    #[error("Failed to load plugin: {path:?}, reason: {reason}")]
    PluginLoadFailed { path: PathBuf, reason: String },

    #[error("Failed to unload plugin: {id}")]
    PluginUnloadFailed { id: u32 },

    #[error("Invalid plugin file: {path:?}")]
    PluginInvalid { path: PathBuf },

    #[error("Plugin incompatible: required version {required_version}, found {found_version}")]
    PluginIncompatible {
        required_version: String,
        found_version: String,
    },

    #[error("Plugin already loaded: {id}")]
    PluginAlreadyLoaded { id: String },

    #[error("Plugin missing required symbol: {symbol}")]
    PluginMissingSymbols { symbol: String },

    #[error("Plugin {id} crashed. {backtrace:?}")]
    PluginCrashed {
        id: String,
        backtrace: Option<String>,
    },

    #[error("Plugin {id} failed to acknowledge opcode {opcode}")]
    PluginAckTimeout { id: String, opcode: u32 },

    // File IO / Resource loading
    #[error("File not found: {path:?}")]
    FileNotFound { path: PathBuf },

    #[error("Failed to read file: {path:?}, reason: {reason}")]
    FileReadError { path: PathBuf, reason: String },

    #[error("Failed to write file: {path:?}, reason: {reason}")]
    FileWriteError { path: PathBuf, reason: String },

    #[error("File corrupted: {path:?}")]
    FileCorrupted { path: PathBuf },

    #[error("Invalid path: {reason}")]
    InvalidPath { reason: String },

    // Config / runtime environment
    #[error("Invalid config key: {key}, reason: {reason:?}")]
    ConfigInvalid { key: String, reason: Option<String> },

    #[error("Missing config key: {key}")]
    ConfigMissing { key: String },

    #[error("Config mismatch: expected {expected}, found {found}")]
    ConfigMismatch { expected: String, found: String },

    #[error("Missing environment variable: {name}")]
    EnvVariableMissing { name: String },

    #[error("Resource unavailable: {name}")]
    ResourceUnavailable { name: String },

    // Audio / MIDI backend
    #[error("Audio initialization failed: {reason}")]
    AudioInitFailed { reason: String },

    #[error("Audio device unavailable: {name:?}")]
    AudioDeviceUnavailable { name: Option<String> },

    #[error("Audio stream error: {reason}")]
    AudioStreamError { reason: String },

    // Dynamic plugin error wrapping
    #[error("Plugin {id} returned an error: {source}")]
    PluginError {
        id: String,
        #[source]
        source: Arc<dyn Error + Send + Sync>,
    },
}
