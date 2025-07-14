use std::ffi::{CStr, c_char};
use tracing::{debug, error, info, trace, warn};

pub fn init_tracing() {
    tracing_subscriber::fmt().pretty().with_level(true).init()
}

#[unsafe(no_mangle)]
pub extern "C" fn log_c(msg: *const c_char, source: *const c_char, level: u8) -> u32 {
    unsafe {
        let msg_str = if msg.is_null() {
            "<<null message>>"
        } else {
            CStr::from_ptr(msg)
                .to_str()
                .unwrap_or("<<non-UTF8 message>>")
        };

        let src_str = if source.is_null() {
            "UNKNOWN"
        } else {
            CStr::from_ptr(source).to_str().unwrap_or("UNKNOWN")
        };

        match level {
            1 => error!(target: "[FFI][C][{}] {}",src_str, msg_str),
            2 => warn!(target: "[FFI][C][{}] {}",src_str, msg_str),
            3 => info!(target: "[FFI][C][{}] {}",src_str, msg_str),
            4 => debug!(target: "[FFI][C][{}] {}",src_str, msg_str),
            5 => trace!(target: "[FFI][C][{}] {}",src_str, msg_str),
            _ => {
                debug!(
                    target = "[CORE][FFI] Received log with illegal log level: {}",
                    level
                );
                debug!(target = "[CORE][FFI] Defaulting message to log level: 3");
                info!(target: "[FFI][C][{}] {}",src_str, msg_str);
            }
        }
    }

    0
}
