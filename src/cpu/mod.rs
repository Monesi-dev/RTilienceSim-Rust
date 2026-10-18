//! # CPU Scheduler Simulators
//!
//! This module provides various CPU scheduling simulators for discrete event simulation (DES).
//!
//! Organized by scheduler variant:
//! - `sp`: Single-Processor fixed-priority scheduler
//! - `mp`: Multi-Processor fixed-priority scheduler, single global ready queue
//! - `mq`: Multi-Processor fixed-priority scheduler, per-CPU local ready queues (static affinity)
//! - `mq_lb`: Multi-Processor fixed-priority scheduler, per-CPU local ready queues with push/pull migration
//! - `mp_cbs`: Multi-Processor Constant Bandwidth Server scheduler, single global queue of servers

pub mod sp;
pub mod mp;
pub mod mq;
pub mod mq_lb;
pub mod mp_cbs;
pub mod scheduler;

// Re-export simulators for convenience
pub use sp::CPUSimulatorSP;
pub use mp::CPUSimulatorMP;
pub use mq::CPUSimulatorMQ;
pub use mq_lb::CPUSimulatorMQLB;
pub use mp_cbs::CPUSimulatorMPCBS;
pub use scheduler::{Scheduler, StaticPriority, EDF};
