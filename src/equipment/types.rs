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
}

/// Component for sampler equipment bodies
#[derive(Component, Debug, Clone)]
pub struct SamplerBody;

/// Component for smelter/refining equipment bodies
#[derive(Component, Debug, Clone)]
pub struct SmelterBody;

/// Component for transport equipment bodies
#[derive(Component, Debug, Clone)]
pub struct TransportBody;