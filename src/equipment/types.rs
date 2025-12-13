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
    pub inventory: std::collections::HashMap<crate::scene::MineralType, f32>, // Current inventory
    pub capacity: f32,                  // Maximum inventory capacity in kg
    pub current_radius: f32,            // Current radius for surface mining expansion
}

impl MinerBody {
    /// Create a surface miner with appropriate parameters
    pub fn surface_miner() -> Self {
        Self {
            equipment_type: EquipmentType::SurfaceMining,
            depth_range: (0.0, 1000.0), // Works on all heights
            mining_radius: 3.0, // Starting radius (expands with each press)
            extraction_rate: 0.1, // Balanced for ~25-40 M presses to fill
            inventory: std::collections::HashMap::new(),
            capacity: 500.0, // 500 kg capacity for surface miner
            current_radius: 3.0, // Start at initial radius
        }
    }

    /// Create a deep miner with appropriate parameters
    pub fn deep_miner() -> Self {
        Self {
            equipment_type: EquipmentType::DeepMining,
            depth_range: (0.0, 1000.0), // Works on all heights
            mining_radius: 2.0, // Small focused area
            extraction_rate: 0.05, // Balanced for ~20-30 M presses to fill
            inventory: std::collections::HashMap::new(),
            capacity: 300.0, // 300 kg capacity for deep miner (smaller but more valuable)
            current_radius: 2.0, // Fixed radius for deep mining
        }
    }

    /// Get the current total mass of inventory
    pub fn current_inventory_mass(&self) -> f32 {
        self.inventory.values().sum()
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> f32 {
        self.capacity - self.current_inventory_mass()
    }

    /// Check if can add amount to inventory
    pub fn can_add(&self, amount: f32) -> bool {
        self.remaining_capacity() >= amount
    }

    /// Add mineral to inventory, returns amount actually added
    pub fn add_to_inventory(&mut self, mineral_type: crate::scene::MineralType, amount: f32) -> f32 {
        let available_capacity = self.remaining_capacity();
        let amount_to_add = amount.min(available_capacity);

        if amount_to_add > 0.0 {
            *self.inventory.entry(mineral_type).or_insert(0.0) += amount_to_add;
        }

        amount_to_add
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

// ============================================================================
// Attachment Components - Transmitters, Receivers, Computers
// ============================================================================

/// Component for transmitter attachments - broadcasts messages
#[derive(Component, Debug, Clone)]
pub struct Transmitter {
    pub range: f32,           // Broadcast range in world units
    pub power: f32,           // Transmission power (affects range)
}

impl Default for Transmitter {
    fn default() -> Self {
        Self {
            range: 100.0,
            power: 1.0,
        }
    }
}

/// Component for receiver attachments - listens for messages
#[derive(Component, Debug, Clone)]
pub struct Receiver {
    pub sensitivity: f32,     // Reception sensitivity (affects range)
}

impl Default for Receiver {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
        }
    }
}

/// Component for computer attachments - runs programs
#[derive(Component, Debug, Clone)]
pub struct Computer {
    pub memory_size: usize,   // Available memory for programs
    pub clock_speed: f32,     // Instructions per second
}

impl Default for Computer {
    fn default() -> Self {
        Self {
            memory_size: 256,
            clock_speed: 10.0,
        }
    }
}

/// Enum to identify attachment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentType {
    Transmitter,
    Receiver,
    Computer,
}

impl AttachmentType {
    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            AttachmentType::Transmitter => "Transmitter",
            AttachmentType::Receiver => "Receiver",
            AttachmentType::Computer => "Computer",
        }
    }

    /// Get the standard offset for this attachment type relative to parent
    pub fn standard_offset(&self) -> bevy::prelude::Vec2 {
        match self {
            AttachmentType::Transmitter => bevy::prelude::Vec2::new(20.0, 20.0),  // Top-right
            AttachmentType::Receiver => bevy::prelude::Vec2::new(-20.0, 20.0),    // Top-left
            AttachmentType::Computer => bevy::prelude::Vec2::new(0.0, 25.0),      // Center-top
        }
    }

    /// Get the mount type required for this attachment
    pub fn mount_type(&self) -> crate::equipment::MountType {
        match self {
            AttachmentType::Transmitter | AttachmentType::Receiver => crate::equipment::MountType::Communication,
            AttachmentType::Computer => crate::equipment::MountType::Control,
        }
    }
}

// Mount point marker components - indicate what types of attachments can be mounted

/// Marker component for equipment that has a communication mount (transmitter/receiver)
#[derive(Component, Debug, Clone, Copy)]
pub struct HasCommunicationMount;

/// Marker component for equipment that has a control mount (computer)
#[derive(Component, Debug, Clone, Copy)]
pub struct HasControlMount;

/// Marker component for equipment that has a utility mount (general purpose)
#[derive(Component, Debug, Clone, Copy)]
pub struct HasUtilityMount;