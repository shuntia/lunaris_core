use std::sync::atomic::AtomicU64;

static NEXT_UUID: AtomicU64 = AtomicU64::new(0);

pub fn get_next() -> u64 {
    NEXT_UUID.fetch_add(1, std::sync::atomic::Ordering::Release)
}
