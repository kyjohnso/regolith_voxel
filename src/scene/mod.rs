//! Scene module - handles terrain, minerals, and physics simulation

mod terrain;
mod minerals;
mod physics;

pub use minerals::{MineralType, MineralMap, MineralMapRenderer, ViewMode};
pub use physics::{CellularAutomataTimer, cellular_automata_system, update_mineral_map_texture};

/// Map dimensions
pub const MAP_WIDTH: usize = 512;
pub const MAP_HEIGHT: usize = 512;

/// Cellular automata tick rate (30 updates per second)
pub const CA_TICK_RATE: f32 = 1.0 / 30.0;