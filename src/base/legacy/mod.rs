//! Legacy implementations kept for reference
//! These were used in earlier versions of the simulator but are no longer part of the active codebase
//!
//! Contents:
//! - event.rs, eventlist.rs, periodic_event.rs, sporadic_event.rs: Old event-based architecture
//! - priority_queue_dll.rs, arena_priority_queue_dll.rs: Old DLL-based queues (before EventQueue trait)
//! - priority_queue_binary_heap.rs: Binary heap implementation (before EventQueue trait)

pub mod event;
pub mod eventlist;
pub mod periodic_event;
pub mod sporadic_event;
pub mod priority_queue_dll;
pub mod arena_priority_queue_dll;
pub mod priority_queue_binary_heap;
