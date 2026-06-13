pub mod api;
pub mod damage_bridge;
pub mod data;
pub mod optimize;
pub mod showdown;
pub mod spreads;
pub mod stats;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use stats::{champions_final_stats, BaseStats, FinalStats, StatPoints};
