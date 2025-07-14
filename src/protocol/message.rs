use std::{any::Any, sync::Arc};

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Standard struct for envelopes.
///
/// This struct is used for all communication that goes through the kernel mailbox.
///
///
/// The uuid is for deduplication and logging purposes.
///
///
/// The source and destination is the bus address for the sender and receiver accordingly.
///
/// require_ack is if the sender requires an ACK. This may be used to check if a task has started.
///
/// message is the actual content that this envelope carries.
#[derive(Debug, Clone)]
pub struct Envelope {
    uuid: u64,
    pub source: u32,
    pub destination: u32,
    pub require_ack: bool,
    pub message: Message,
}

impl Envelope {
    pub fn new(source: u32, destination: u32, require_ack: bool, message: Message) -> Self {
        Self {
            uuid: crate::utils::uuid::get_next(),
            source,
            destination,
            require_ack,
            message,
        }
    }
}

unsafe impl Send for Envelope {}
unsafe impl Sync for Envelope {}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct CEnvelope {
    uuid: u64,
    pub source: u32,
    pub destination: u32,
    pub require_ack: bool,
    pub message: Message,
}

impl Into<Envelope> for CEnvelope {
    fn into(self) -> Envelope {
        Envelope {
            uuid: self.uuid,
            source: self.source,
            destination: self.destination,
            require_ack: self.require_ack,
            message: self.message.into(),
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
                    DataType::Bytes => Data {
                        bytes: self.data.bytes.clone(),
                    },
                    DataType::FfiPeek => Data {
                        ffi_peek: self.data.ffi_peek.clone(),
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
                DataType::Code | DataType::FfiPeek | DataType::None => {}
                // Required drop
                DataType::Bytes => {
                    std::mem::ManuallyDrop::drop(&mut self.data.bytes);
                }
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
    Bytes = 2,
    FfiPeek = 3,
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
    bytes: std::mem::ManuallyDrop<Vec<u8>>,
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
                DataType::Bytes => DataEnum::Bytes(std::mem::ManuallyDrop::into_inner(self.bytes)),
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
            .map(|b| format!("{:02X}", b))
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
///
/// It requires a free() equivalent callback.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FFIPeek {
    ptr: *const u8,
    len: usize,
    free: extern "C" fn(*const u8, usize),
}

unsafe impl Send for FFIPeek {}
unsafe impl Sync for FFIPeek {}

impl FFIPeek {
    /// Obtain a slice of the target bytes.
    pub unsafe fn extract_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for FFIPeek {
    fn drop(&mut self) {
        (self.free)(self.ptr, self.len);
    }
}
