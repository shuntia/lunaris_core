use core::alloc;
use std::alloc::Layout;
use std::mem::ManuallyDrop;
use std::{alloc::alloc, ptr::null_mut};
use std::{
    any::Any,
    mem::align_of_val_raw,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use tracing::*;

/// Standard struct for envelopes.
///
/// This struct is used for all communication that goes through the kernel mailbox.
///
///
/// The id is for deduplication and logging purposes.
///
///
/// The source and destination is the bus address for the sender and receiver accordingly.
///
/// require_ack is if the sender requires an ACK. This may be used to check if a task has started.
///
/// message is the actual content that this envelope carries.
#[derive(Debug, Clone)]
pub struct Envelope {
    pub id: u64,
    pub source: u32,
    pub destination: u32,
    pub require_ack: bool,
    pub message: Message,
}

impl Envelope {
    pub fn new(source: u32, destination: u32, require_ack: bool, message: Message) -> Self {
        Self {
            id: crate::utils::id::get_next(),
            source,
            destination,
            require_ack,
            message,
        }
    }
    pub fn clone_empty(&self) -> Self {
        Self {
            id: self.id,
            source: self.source,
            destination: self.destination,
            require_ack: self.require_ack,
            message: Message {
                opcode: self.message.opcode,
                data: DataEnum::None,
            },
        }
    }
}

unsafe impl Send for Envelope {}
unsafe impl Sync for Envelope {}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct CEnvelope {
    id: u64,
    pub source: u32,
    pub destination: u32,
    pub require_ack: bool,
    pub message: CMessage,
}

impl From<CEnvelope> for Envelope {
    fn from(value: CEnvelope) -> Self {
        Envelope {
            id: value.id,
            source: value.source,
            destination: value.destination,
            require_ack: value.require_ack,
            message: value.message.into(),
        }
    }
}

/// This is the uniform type of data that gets passed around in Lunatic Studio.
///
/// This is the weak point in this architecture, as it is prone to memory leaks, segmentation
/// faults, and other memory-related issues.
///
/// Especially the union struct Data, which is very dangerous, as it contains FFI elements as well.
#[derive(Debug, Clone)]
pub struct Message {
    pub opcode: u32,
    pub data: DataEnum,
}

#[repr(C)]
pub struct CMessage {
    pub opcode: u32,
    data_type: DataType,
    data: Data,
}

impl Clone for CMessage {
    fn clone(&self) -> Self {
        CMessage {
            opcode: self.opcode,
            data_type: self.data_type,
            data: unsafe {
                match self.data_type {
                    DataType::None => Data { none: () },
                    DataType::Code => Data {
                        code: self.data.code,
                    },
                    DataType::FfiPeek => Data {
                        ffi_peek: self.data.ffi_peek,
                    },
                    DataType::FfiData => Data {
                        ffi_data: self.data.ffi_data.clone(),
                    },
                }
            },
        }
    }
}

/// I couldn't think of a better way to represent a union without touching it badly :(
///
/// If you want to actually debug a message, it'd be better to use try_into.
impl std::fmt::Debug for CMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message{{opcode: {:?}, data_type: {:?}, data: {}}}",
            self.opcode,
            self.data_type,
            unsafe { self.data.to_hex_string() }
        )
    }
}

/// Custom drop handler for Message.
///
/// This should not fail unless the plugin or usage is wrong, and unsafe operations have been
/// applied to it.
impl Drop for CMessage {
    fn drop(&mut self) {
        unsafe {
            match self.data_type {
                DataType::Code | DataType::FfiPeek | DataType::None => {} // Required drop
                DataType::FfiData => (self.data.ffi_data.free)(self.data.ffi_data.ptr),
            }
        }
    }
}

impl From<CMessage> for Message {
    fn from(value: CMessage) -> Self {
        unsafe {
            Message {
                opcode: value.opcode,
                data: match value.data_type {
                    DataType::None => DataEnum::None,
                    DataType::Code => DataEnum::Code(value.data.code),
                    DataType::FfiPeek => DataEnum::FfiPeek(*value.data.ffi_peek),
                    DataType::FfiData => {
                        DataEnum::FfiData(ManuallyDrop::into_inner(value.data.ffi_data.clone()))
                    }
                },
            }
        }
    }
}

/// Internal DataType to track how to unwrap the union. Required for C interop.
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum DataType {
    None = 0,
    Code = 1,
    FfiPeek = 2,
    FfiData = 3,
}

/// Data content of Messages.
///
/// This uniformly carries data of the messages passed around in Lunatic Studio.
///
/// code is used for simple messages like error codes.
///
/// boxed is the generic type that Lunatic Studio utilizes to pass around messages.
///
/// bytes is a special struct just for sequences of bytes. Often used for data retrieved through
/// sockets.
///
/// ffi is the most dangerous type, as it could leak very easily due to the insufficiency of
/// guarantees.
///
/// ffi objects(ESPECIALLY C/C++) MUST use ffi_data or ffi_peek(as long as you deallocate properly)
#[repr(C)]
pub union Data {
    none: (),
    code: u32,
    ffi_data: std::mem::ManuallyDrop<FFIData>,
    ffi_peek: std::mem::ManuallyDrop<FFIPeek>,
}

impl Data {
    /// Convert to safe enum, according to the data type.
    unsafe fn into_enum_as(self, d: DataType) -> DataEnum {
        unsafe {
            match &d {
                DataType::None => DataEnum::None,
                DataType::FfiPeek => {
                    DataEnum::FfiPeek(std::mem::ManuallyDrop::<FFIPeek>::into_inner(self.ffi_peek))
                }
                DataType::Code => DataEnum::Code(self.code),
                DataType::FfiData => DataEnum::FfiData(ManuallyDrop::into_inner(self.ffi_data)),
            }
        }
    }
    unsafe fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Data as *const u8;
        let size = std::mem::size_of::<Data>();
        unsafe { std::slice::from_raw_parts(ptr, size) }
    }
    pub unsafe fn to_hex_string(&self) -> String {
        let bytes = unsafe { self.as_bytes() };
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone)]
pub enum DataEnum {
    None,
    Code(u32),
    Arc(Arc<dyn Any + Send + 'static>),
    Bytes(Vec<u8>),
    FfiData(FFIData),
    FfiPeek(FFIPeek),
}

impl From<&FFIPeek> for Vec<u8> {
    fn from(value: &FFIPeek) -> Self {
        unsafe { value.extract_slice().to_vec() }
    }
}

/// This struct is explicitly for data that is not owned by Rust and is only for C compatibility.
/// You cannot clone this, but you can
///
/// When using this, the sender must not free the memory while FFIPeek is alive.
///
/// This provides very quick memory reads with almost zero cost.
///
/// This is arguably the most unsafe struct in this whole project.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FFIPeek {
    ptr: *const u8,
    len: usize,
}

unsafe impl Send for FFIPeek {}
unsafe impl Sync for FFIPeek {}

impl FFIPeek {
    /// Obtain a slice of the target bytes.
    pub unsafe fn extract_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

/// This is an "owned" equivalent for FfiPeek.
/// It must contain a callback for free(), as it is heap-alloc.
///
/// If you forget to add free(), congrats. you've leaked memory.
#[derive(Debug)]
#[repr(C)]
pub struct FFIData {
    ptr: *mut u8,
    len: usize,
    free: unsafe extern "C" fn(*mut u8),
}

unsafe extern "C" fn dummy(_: *mut u8) {}

impl Clone for FFIData {
    fn clone(&self) -> Self {
        let loc = unsafe {
            alloc(
                alloc::Layout::from_size_align(self.len, align_of_val_raw(self.ptr))
                    .unwrap_or_else(|_| {
                        error!("Failed to create layout from pointer!");
                        Layout::new::<()>()
                    }),
            )
        };
        Self {
            ptr: loc,
            len: self.len,
            free: dummy,
        }
    }
}

impl Drop for FFIData {
    fn drop(&mut self) {
        unsafe { (self.free)(self.ptr) }
    }
}

impl FFIData {
    pub unsafe fn force_cast<T>(self) -> Option<*mut T> {
        if self.ptr.is_null() {
            return None;
        }
        if self.len != size_of::<T>() {
            return None;
        }
        if self.ptr as usize % align_of::<T>() != 0 {
            return None;
        }
        Some(self.ptr as *mut T)
    }
    pub unsafe fn force_box<T>(self) -> Option<BoxedFFI<T>> {
        if self.ptr.is_null() {
            return None;
        }
        if self.len != size_of::<T>() {
            return None;
        }
        unsafe {
            if self.ptr as usize % align_of::<T>() != 0 {
                return None;
            }
        }
        unsafe {
            Some(BoxedFFI {
                content: ManuallyDrop::new(Box::from_raw(self.ptr as *mut T)),
                free: self.free,
            })
        }
    }
}

pub struct BoxedFFI<T> {
    content: ManuallyDrop<Box<T>>,
    free: unsafe extern "C" fn(*mut u8),
}

impl<T> Drop for BoxedFFI<T> {
    fn drop(&mut self) {
        unsafe {
            (self.free)(
                Box::into_raw(ManuallyDrop::into_inner(std::ptr::read(&self.content))) as *mut u8,
            )
        }
    }
}

impl<T> DerefMut for BoxedFFI<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.content
    }
}

impl<T> Deref for BoxedFFI<T> {
    type Target = Box<T>;
    fn deref(&self) -> &Self::Target {
        &*self.content
    }
}
