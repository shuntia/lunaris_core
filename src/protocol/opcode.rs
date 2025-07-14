use num_enum::{IntoPrimitive, TryFromPrimitive};

// This module contains the system op codes ranging from 0-1023.
// Op codes from 0-1023 are reserved and therefore no plugins are allowed to register these op
// codes.
//
// The main use of this file is to provide constants for unified op code standards accross LS.
//

/// Basic set of op codes that any plugin must utilize
#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum Basic {
    /// No operation
    NOOP = 0,
    /// Call to initialize plugin
    INIT = 1,
    /// Reset plugin
    RESET = 3,
    /// Tick frame or event
    TICK = 2,
}

/// System call to kernel.
/// These will not be handled by any plugin at all.
/// Required that the message destination is 0
#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum Sys {
    LOAD_PLUGIN = 8,
    PROBE = 9,
}
