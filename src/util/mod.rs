use std::sync::atomic::{AtomicU64, Ordering};

#[macro_use]
pub mod macros;

/// Generate globally unique event IDs
pub fn get_next_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
