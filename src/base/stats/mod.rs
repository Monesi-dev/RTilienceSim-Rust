//! Statistics utilities: random variables and distributions

mod trait_def;
mod deterministic;
mod uniform;
mod normal;
mod exponential;
mod poisson;
mod tstudent;
mod detvar;
mod filevar;

pub use trait_def::RandomVariable;
pub use deterministic::Deterministic;
pub use uniform::Uniform;
pub use normal::Normal;
pub use exponential::Exponential;
pub use poisson::Poisson;
pub use tstudent::TStudent;
pub use detvar::DetVar;
pub use filevar::FileVar;
