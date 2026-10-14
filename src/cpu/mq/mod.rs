//! # Multi-Processor Multi-Queue CPU Scheduler
//!
//! Like `mp`, but each CPU has its own local ready queue and tasks are statically
//! pinned (affinity) to the CPU with the lowest estimated utilization at registration
//! time, instead of competing for a single global ready queue.

pub mod state;
pub mod simulator;
pub mod stats;

pub use simulator::CPUSimulatorMQ;
