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
//! - `mq_cbs`: Multi-Processor Constant Bandwidth Server scheduler, per-CPU queues (static affinity)
//! - `mp_hcbs`: Multi-Processor **hard** CBS scheduler, global queue, servers go dormant when throttled
//! - `mq_hcbs`: Multi-Processor **hard** CBS scheduler, per-CPU queues (static affinity), servers go dormant when throttled

pub mod sp;
pub mod mp;
pub mod mq;
pub mod mq_lb;
pub mod mp_cbs;
pub mod mq_cbs;
pub mod mp_hcbs;
pub mod mq_hcbs;
pub mod scheduler;

// Re-export simulators for convenience
pub use sp::CPUSimulatorSP;
pub use mp::CPUSimulatorMP;
pub use mq::CPUSimulatorMQ;
pub use mq_lb::CPUSimulatorMQLB;
pub use mp_cbs::CPUSimulatorMPCBS;
pub use mq_cbs::CPUSimulatorMQCBS;
pub use mp_hcbs::CPUSimulatorMPHCBS;
pub use mq_hcbs::CPUSimulatorMQHCBS;
pub use scheduler::{Scheduler, StaticPriority, EDF};
