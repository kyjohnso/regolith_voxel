//! Mineral types, cells, and map resource

use bevy::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};
use rand::{thread_rng, Rng};

use super::terrain::LunarDEM;
use super::physics::PhysicsType;
use super::{MAP_HEIGHT, MAP_WIDTH};

/// Types of minerals that can be found in the map
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MineralType {
    Empty,    // Black/dark gray - void/air
    Iron,     // Rusty orange
    Copper,   // Copper color
    Gold,     // Gold/yellow
    Silver,   // Light gray/silver
    Uranium,  // Green
    Diamond,  // Cyan/blue
    Coal,     // Dark gray
}

impl MineralType {
    /// Get the display color for this mineral type
    pub fn color(&self) -> Color {
        match self {
            MineralType::Empty => Color::srgb(0.1, 0.1, 0.15),
            MineralType::Iron => Color::srgb(0.8, 0.4, 0.2),
            MineralType::Copper => Color::srgb(0.72, 0.45, 0.2),
            MineralType::Gold => Color::srgb(1.0, 0.84, 0.0),
            MineralType::Silver => Color::srgb(0.75, 0.75, 0.75),
            MineralType::Uranium => Color::srgb(0.2, 0.8, 0.2),
            MineralType::Diamond => Color::srgb(0.4, 0.8, 1.0),
            MineralType::Coal => Color::srgb(0.2, 0.2, 0.2),
        }
    }

    /// Get the physics behavior type for this mineral
    pub fn physics_type(&self) -> PhysicsType {
        match self {
            MineralType::Empty => PhysicsType::Empty,
            MineralType::Diamond | MineralType::Uranium => PhysicsType::Solid,
            MineralType::Coal | MineralType::Iron | MineralType::Copper => PhysicsType::Granular,
            MineralType::Gold | MineralType::Silver => PhysicsType::Flowing,
        }
    }

    /// Determine mineral type from noise value and depth
    /// 
    /// Depth affects mineral distribution (deeper = rarer minerals)
    pub fn from_noise_value(value: f64, depth: f64) -> Self {
        let depth_factor = depth / MAP_HEIGHT as f64;

        match value {
            v if v < -0.4 => MineralType::Empty,
            v if v < -0.2 && depth_factor > 0.6 => MineralType::Uranium,
            v if v < 0.0 => MineralType::Coal,
            v if v < 0.2 => MineralType::Iron,
            v if v < 0.4 => MineralType::Copper,
            v if v < 0.6 && depth_factor > 0.5 => MineralType::Silver,
            v if v < 0.8 && depth_factor > 0.7 => MineralType::Gold,
            v if v < 1.0 && depth_factor > 0.8 => MineralType::Diamond,
            _ => MineralType::Empty,
        }
    }
}

/// Data for each cell/pixel in the map
#[derive(Debug, Clone)]
pub struct MineralCell {
    pub mineral_type: MineralType,
    pub density: f32,  // 0.0 to 1.0, how much mineral is present
    pub sampled: bool, // Has this cell been sampled?
    pub mined: bool,   // Has this cell been mined?
}

impl Default for MineralCell {
    fn default() -> Self {
        Self {
            mineral_type: MineralType::Empty,
            density: 0.0,
            sampled: false,
            mined: false,
        }
    }
}

/// The main mineral map resource containing terrain and mineral data
#[derive(Resource)]
pub struct MineralMap {
    pub width: usize,
    pub height: usize,
    pub data: Vec<MineralCell>,
    pub heightmap: Vec<f32>, // Invisible heightmap for flow simulation
}

impl Default for MineralMap {
    fn default() -> Self {
        Self::generate()
    }
}

impl MineralMap {
    /// Generate a new mineral map with procedural terrain
    pub fn generate() -> Self {
        let mut rng = thread_rng();
        let seed: u32 = rng.gen();

        // Create noise generators
        let perlin = Perlin::new(seed);
        let fbm = Fbm::<Perlin>::new(seed);
        let height_noise = Perlin::new(seed.wrapping_add(1000));

        let mut data = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);
        let mut heightmap = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);

        // Try to load lunar DEM for heightmap
        let lunar_heights = LunarDEM::load("data/ldem_16.tif").and_then(|dem| {
            // Pick random point on lunar surface (avoid extreme poles)
            let center_lon = rng.gen_range(-180.0..180.0); // Longitude: -180 to 180
            let center_lat = rng.gen_range(-60.0..60.0); // Latitude: -60 to 60 (avoid poles)
            let angular_extent = 40.0; // Angular extent in degrees (~700 km patch at equator)

            println!(
                "Sampling lunar DEM at lon={:.1}°, lat={:.1}° with extent={:.1}°",
                center_lon, center_lat, angular_extent
            );

            // Sample with latitude-corrected rectangular projection
            let samples =
                dem.sample_lat_corrected(center_lon, center_lat, angular_extent, MAP_WIDTH, MAP_HEIGHT);

            // Normalize and scale
            Some(LunarDEM::normalize_and_scale(
                &samples, MAP_WIDTH, MAP_HEIGHT, 200.0,
            ))
        });

        if lunar_heights.is_some() {
            println!("Successfully loaded lunar DEM heightmap!");
        } else {
            println!("Falling back to procedural heightmap generation");
        }

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                // Use multiple octaves of noise for varied terrain
                let scale = 0.02;
                let noise_value = fbm.get([x as f64 * scale, y as f64 * scale]);

                // Add some fine detail
                let detail = perlin.get([x as f64 * 0.1, y as f64 * 0.1]) * 0.2;
                let combined = noise_value + detail;

                let mineral_type = MineralType::from_noise_value(combined, y as f64);
                let density = ((combined + 1.0) / 2.0) as f32; // Normalize to 0-1

                data.push(MineralCell {
                    mineral_type,
                    density,
                    sampled: false,
                    mined: false,
                });

                // Generate heightmap - represents material depth/height at this location
                let height = if let Some(ref lunar_heights) = lunar_heights {
                    // Use real lunar elevation data directly!
                    let idx = y * MAP_WIDTH + x;
                    let lunar_height = lunar_heights[idx];
                    // Apply to all cells including empty ones to show craters
                    if mineral_type == MineralType::Empty {
                        0.0
                    } else {
                        // Add small density variation but preserve lunar features
                        lunar_height + (density * 10.0)
                    }
                } else if mineral_type == MineralType::Empty {
                    0.0
                } else {
                    // Fallback to procedural generation
                    let height_scale = 0.02; // Larger features
                    let detail_scale = 0.08; // Fine detail

                    let large_variation =
                        height_noise.get([x as f64 * height_scale, y as f64 * height_scale]);
                    let detail =
                        height_noise.get([x as f64 * detail_scale, y as f64 * detail_scale]);

                    // Combine for dramatic height differences (0-200 range)
                    let base_height = density * 80.0;
                    let terrain = (large_variation as f32 * 60.0) + (detail as f32 * 40.0);
                    (base_height + terrain).max(5.0) // Minimum height 5
                };
                heightmap.push(height);
            }
        }

        Self {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            data,
            heightmap,
        }
    }

    /// Get a cell at the given coordinates
    pub fn get(&self, x: usize, y: usize) -> Option<&MineralCell> {
        if x < self.width && y < self.height {
            Some(&self.data[y * self.width + x])
        } else {
            None
        }
    }

    /// Get a mutable reference to a cell at the given coordinates
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut MineralCell> {
        if x < self.width && y < self.height {
            Some(&mut self.data[y * self.width + x])
        } else {
            None
        }
    }
}

/// Component to mark the mineral map sprite entity
#[derive(Component)]
pub struct MineralMapRenderer;

/// View mode for the map display
#[derive(Resource, Default, PartialEq, Debug)]
pub enum ViewMode {
    Minerals,
    #[default]
    Heightmap,
}