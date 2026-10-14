//! Priority queue implementations with pluggable strategies via EventQueue trait

mod priority_queue;
mod priority_queue_trait;
mod arena_queue;

pub use priority_queue::{BTreeQueue, Node, Event};
pub use priority_queue_trait::{EventQueue, EventPriorities};
pub use arena_queue::ArenaQueue;
