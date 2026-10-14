//! Trait for priority queue implementations to allow swapping different strategies

use crate::base::sim_time::SimTime;
use super::priority_queue::Node;
use std::fmt::Debug;

/// Maps an event variant to a numeric priority used to tie-break events that share the
/// same simulation time (lower value is popped first). Implementations are expected to
/// be zero-sized marker types: the mapping is a fixed, compile-time policy rather than
/// runtime state, so `priority` is an associated function instead of a `&self` method.
pub trait EventPriorities<T> {
    fn priority(event_type: &T) -> u32;
}

/// Trait that all priority queue implementations must satisfy
pub trait EventQueue<T: Debug + Copy, P: EventPriorities<T>> {
    /// Create a new queue with default capacity
    fn new() -> Self;

    /// Insert an event, returning a handle for later removal if needed
    fn insert(&mut self, event_id: u64, time: SimTime, event_type: T) -> Result<(), String>;

    /// Extract and return the first (earliest) event
    fn pop_first(&mut self) -> Option<Node<T>>;

    /// Peek at the time of the next event without removing it
    fn peek_first(&self) -> Option<SimTime>;

    /// Check if the queue is empty
    fn is_empty(&self) -> bool;

    /// Get the number of events in the queue
    fn len(&self) -> usize;

    /// Clear all events from the queue
    fn clear(&mut self);

    /// Remove a specific event (if implementation supports it)
    fn remove(&mut self, time: SimTime, event_id: u64) -> bool;
}
