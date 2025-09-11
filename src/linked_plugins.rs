// Ensure plugin crates are linked so `inventory` can discover their registrations.
include!(concat!(env!("OUT_DIR"), "/linking.rs"));
