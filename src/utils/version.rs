use std::fmt::Display;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

pub struct Version {
    pub full: &'static str,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<&'static str>,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full)
    }
}

pub const VERSION: Version = Version {
    full: VERSION_FULL,
    major: VERSION_MAJOR,
    minor: VERSION_MINOR,
    patch: VERSION_PATCH,
    pre: VERSION_PRE,
};
