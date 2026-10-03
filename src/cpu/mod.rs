//! # CPU Scheduler Simulators
//!
//! This module provides various CPU scheduling simulators for discrete event simulation (DES).
//!
//! Organized by scheduler variant:
//! - `sp`: Single-Processor fixed-priority scheduler
//! - `mp`: Multi-Processor fixed-priority scheduler

pub mod sp;
pub mod mp;

// Re-export simulators for convenience
pub use sp::CPUSimulatorSP;
pub use mp::CPUSimulatorMP;
