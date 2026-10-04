//! # Single-Processor CPU Scheduler

pub mod state;
pub mod simulator;
pub mod simulator_minimal;
pub mod stats;
pub mod stats_minimal;

pub use simulator::CPUSimulatorSP;
pub use simulator_minimal::CPUSimulatorSPMinimal;
