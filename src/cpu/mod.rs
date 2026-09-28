//! # CPU Scheduler Simulators
//!
//! This module provides various CPU scheduling simulators for discrete event simulation (DES).

pub mod fixed_priority_SP;

// Re-export the single-processor simulator
pub use fixed_priority_SP::CPUSimulatorSP;
