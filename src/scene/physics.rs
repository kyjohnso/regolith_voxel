//! Physics simulation using cellular automata

use bevy::prelude::*;
use rand::{thread_rng, Rng};

use super::minerals::{MineralCell, MineralMap, MineralMapRenderer, MineralType, ViewMode};
use super::{CA_TICK_RATE, MAP_HEIGHT, MAP_WIDTH};

/// Physics behavior types for cellular automata simulation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsType {
    Empty,    // Void/air - materials can move into this
    Solid,    // Structural - doesn't move
    Granular, // Falls like sand when unsupported
    Flowing,  // Flows like liquid
}

/// Timer resource for cellular automata updates
#[derive(Resource)]
pub struct CellularAutomataTimer {
    pub timer: Timer,
}

impl Default for CellularAutomataTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(CA_TICK_RATE, TimerMode::Repeating),
        }
    }
}

/// Cellular automata system - updates mineral cells based on physics rules
pub fn cellular_automata_system(
    time: Res<Time>,
    mut timer: ResMut<CellularAutomataTimer>,
    mut mineral_map: ResMut<MineralMap>,
) {
    // Only update at the configured tick rate
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    let width = mineral_map.width;
    let height = mineral_map.height;

    // Create a copy of the data to read from (avoid borrowing issues)
    let mut next_data = mineral_map.data.clone();
    let mut next_heightmap = mineral_map.heightmap.clone();

    let mut rng = thread_rng();

    // Process all cells - materials flow toward lower heights in ANY direction
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let cell = &mineral_map.data[idx];
            let physics = cell.mineral_type.physics_type();

            if physics == PhysicsType::Empty || physics == PhysicsType::Solid {
                continue; // Nothing to do for empty or solid cells
            }

            let current_height = mineral_map.heightmap[idx];

            // Check all 4 cardinal neighbors (simpler, more stable)
            let mut candidates: Vec<(usize, usize, f32)> = Vec::new();

            // Define 4 directions (N, E, S, W)
            let directions = [
                (0, -1),  // N
                (1, 0),   // E
                (0, 1),   // S
                (-1, 0),  // W
            ];

            for (dx, dy) in directions.iter() {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                // Check bounds
                if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                    continue;
                }

                let nx = nx as usize;
                let ny = ny as usize;
                let neighbor_idx = ny * width + nx;
                let neighbor_height = mineral_map.heightmap[neighbor_idx];

                // Calculate height difference
                let height_diff = current_height - neighbor_height;

                // Materials flow toward lower heights based on physics type
                // GRANULAR PHYSICS - move to areas with significantly lower height
                if physics == PhysicsType::Granular {
                    // Need bigger height difference to move
                    if height_diff > 8.0 && rng.gen_bool(0.5) {
                        candidates.push((nx, ny, neighbor_height));
                    }
                }
                // FLOWING PHYSICS - move to areas with moderately lower height
                else if physics == PhysicsType::Flowing {
                    // More sensitive to height differences
                    if height_diff > 4.0 && rng.gen_bool(0.7) {
                        candidates.push((nx, ny, neighbor_height));
                    }
                }
            }

            // Pick a random candidate (don't always pick lowest for variety)
            if !candidates.is_empty() && rng.gen_bool(0.4) {
                let chosen = candidates[rng.gen_range(0..candidates.len())];
                let (nx, ny, target_height) = chosen;
                let target_idx = ny * width + nx;

                // Transfer height between cells (like grains sliding down)
                let height_transfer = 1.0; // Transfer 1 unit of height

                // If target is empty or has very low height, place our material there
                if next_data[target_idx].mineral_type == MineralType::Empty
                    || next_heightmap[target_idx] < 1.0
                {
                    next_data[target_idx] = cell.clone();
                    next_heightmap[target_idx] = target_height + height_transfer;
                } else {
                    // Target has material - just add height (visual only, material stays)
                    next_heightmap[target_idx] = target_height + height_transfer;
                }

                // Reduce source height
                next_heightmap[idx] = current_height - height_transfer;

                // If source height is depleted, remove the material
                if next_heightmap[idx] < 0.5 {
                    next_data[idx] = MineralCell {
                        mineral_type: MineralType::Empty,
                        density: 0.0,
                        sampled: cell.sampled,
                        mined: true,
                    };
                    next_heightmap[idx] = 0.0;
                }
            }
        }
    }

    // Update the mineral map with the new state
    mineral_map.data = next_data;
    mineral_map.heightmap = next_heightmap;
}

/// System to update the mineral map texture after CA updates
pub fn update_mineral_map_texture(
    mineral_map: Res<MineralMap>,
    view_mode: Res<ViewMode>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&Sprite, With<MineralMapRenderer>>,
) {
    // Always update when mineral map changes, or when view mode changes
    if !mineral_map.is_changed() && !view_mode.is_changed() {
        return;
    }

    // Find the mineral map sprite
    for sprite in query.iter() {
        if let Some(image) = images.get_mut(&sprite.image) {
            // Update the texture data based on view mode
            match *view_mode {
                ViewMode::Minerals => {
                    // Render minerals with colors
                    if let Some(data) = &mut image.data {
                        let mut idx = 0;
                        for cell in &mineral_map.data {
                            let color = cell.mineral_type.color();
                            let brightness = 0.5 + cell.density * 0.5;
                            data[idx] = (color.to_srgba().red * brightness * 255.0) as u8;
                            data[idx + 1] = (color.to_srgba().green * brightness * 255.0) as u8;
                            data[idx + 2] = (color.to_srgba().blue * brightness * 255.0) as u8;
                            data[idx + 3] = 255;
                            idx += 4;
                        }
                    }
                }
                ViewMode::Heightmap => {
                    // Render heightmap as grayscale with enhanced contrast
                    if let Some(data) = &mut image.data {
                        for (cell_idx, cell) in mineral_map.data.iter().enumerate() {
                            let height = mineral_map.heightmap[cell_idx];

                            // Map height to grayscale (0-200 range with dramatic variation)
                            let normalized = (height / 200.0).clamp(0.0, 1.0);
                            let gray_value = (normalized * 255.0) as u8;

                            let pixel_idx = cell_idx * 4;

                            // Color empty cells slightly blue to distinguish them
                            if cell.mineral_type == MineralType::Empty {
                                data[pixel_idx] = gray_value / 3;
                                data[pixel_idx + 1] = gray_value / 3;
                                data[pixel_idx + 2] = gray_value.saturating_add(40);
                            } else {
                                data[pixel_idx] = gray_value;
                                data[pixel_idx + 1] = gray_value;
                                data[pixel_idx + 2] = gray_value;
                            }
                            data[pixel_idx + 3] = 255;
                        }
                    }
                }
            }
        }
    }
}