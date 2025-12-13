//! Equipment type definitions

use bevy::prelude::*;

/// Mining equipment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentType {
    Sampler,
    SurfaceMining,
    DeepMining,
    Refining,
    Transport,
}

impl EquipmentType {
    /// Get the display name for this equipment type
    pub fn name(&self) -> &str {
        match self {
            EquipmentType::Sampler => "Sampler",
            EquipmentType::SurfaceMining => "Surface Mining",
            EquipmentType::DeepMining => "Deep Mining",
            EquipmentType::Refining => "Refining",
            EquipmentType::Transport => "Transport",
        }
    }

    /// Get the description for this equipment type
    pub fn description(&self) -> &str {
        match self {
            EquipmentType::Sampler => "Analyzes mineral composition without extraction",
            EquipmentType::SurfaceMining => "Extracts minerals from the upper layers",
            EquipmentType::DeepMining => "Extracts minerals from deep deposits",
            EquipmentType::Refining => "Processes raw minerals into refined materials",
            EquipmentType::Transport => "Moves resources between locations",
        }
    }

    /// Get the sprite path for this equipment type
    pub fn sprite_path(&self) -> &str {
        match self {
            EquipmentType::Sampler => "sprites/sampler.png",
            EquipmentType::SurfaceMining => "sprites/surface_mining.png",
            EquipmentType::DeepMining => "sprites/deep_mining.png",
            EquipmentType::Refining => "sprites/refining.png",
            EquipmentType::Transport => "sprites/transport.png",
        }
    }
}

// ============================================================================
// ECS Component System
// ============================================================================

/// Base component for all equipment pieces - every piece of equipment has this
#[derive(Component, Debug, Clone)]
pub struct EquipmentPiece {
    pub id: usize,
    pub name: String,
    pub mass: f32,
    pub energy_consumption: f32,
}

/// Describes how an equipment entity relates to other entities
#[derive(Component, Debug, Clone)]
pub enum AttachmentState {
    /// Equipment is free-standing on the map at the given position
    Detached { position: Vec2 },
    /// Equipment is attached to a parent equipment (future use for transmitters/receivers/computers)
    AttachedToEquipment { parent: Entity },
    /// Equipment is being carried in a truck's cargo (future use)
    InCargo { truck: Entity, cargo_index: usize },
}

/// Component for miner equipment bodies
#[derive(Component, Debug, Clone)]
pub struct MinerBody {
    pub equipment_type: EquipmentType, // SurfaceMining or DeepMining
    pub depth_range: (f32, f32),        // Min and max depth this miner can work at
    pub mining_radius: f32,             // Radius of mining operation
    pub extraction_rate: f32,           // How much material extracted per operation
}

impl MinerBody {
    /// Create a surface miner with appropriate parameters
    pub fn surface_miner() -> Self {
        Self {
            equipment_type: EquipmentType::SurfaceMining,
            depth_range: (0.0, 20.0),
            mining_radius: 8.0,
            extraction_rate: 1.0,
        }
    }

    /// Create a deep miner with appropriate parameters
    pub fn deep_miner() -> Self {
        Self {
            equipment_type: EquipmentType::DeepMining,
            depth_range: (50.0, 200.0),
            mining_radius: 5.0,
            extraction_rate: 2.0,
        }
    }
}

/// Component for sampler equipment bodies
#[derive(Component, Debug, Clone)]
pub struct SamplerBody {
    pub scan_radius: f32,    // How far the sampler can scan
    pub scan_interval: f32,  // Time between scans in seconds
}

impl Default for SamplerBody {
    fn default() -> Self {
        Self {
            scan_radius: 30.0,
            scan_interval: 2.0,
        }
    }
}

/// Component for smelter/refining equipment bodies
#[derive(Component, Debug, Clone)]
pub struct SmelterBody {
    pub processing_rate: f32,                           // Units per second
    pub input_buffer: std::collections::HashMap<crate::scene::MineralType, f32>,
    pub output_buffer: std::collections::HashMap<crate::scene::MineralType, f32>,
}

impl Default for SmelterBody {
    fn default() -> Self {
        Self {
            processing_rate: 1.0,
            input_buffer: std::collections::HashMap::new(),
            output_buffer: std::collections::HashMap::new(),
        }
    }
}

/// Component for transport equipment bodies
#[derive(Component, Debug, Clone)]
pub struct TransportBody {
    pub speed: f32,            // Movement speed multiplier
    pub cargo_capacity: f32,   // Maximum cargo weight
}

impl Default for TransportBody {
    fn default() -> Self {
        Self {
            speed: 1.5,
            cargo_capacity: 1000.0,
        }
    }
}