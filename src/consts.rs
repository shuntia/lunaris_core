/// Compiled in headless mode.
/// This does not mean that
#[cfg(not(feature = "headless"))]
pub const HEADLESS: bool = false;
#[cfg(feature = "headless")]
pub const HEADLESS: bool = true;

pub const TPS: u64 = 42336000000;

// The rest will be created by the build script.
include!(concat!(env!("OUT_DIR"), "/version.rs"));
