//! # Multi-Processor Multi-Queue CPU Scheduler with Push/Pull Load Balancing
//!
//! Unlike `mq` (static affinity), jobs here can migrate across CPUs via push/pull
//! load balancing while maintaining M-CPU global scheduling invariant without a
//! shared queue.

pub mod state;
pub mod simulator;
pub mod stats;

pub use simulator::CPUSimulatorMQLB;
