mod sim_time;
pub mod priority_queue;
pub mod stats;
// pub mod legacy;  // Kept for reference but not compiled (uses old Event trait)

pub mod prelude {
    pub use crate::base::sim_time::*;
    pub use crate::base::priority_queue::{BTreeQueue, Node, EventQueue, ArenaQueue};
    pub use crate::base::stats::RandomVariable;
}
