//! Equipment systems - handles sprites, selection, movement, and mining

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use rand::{thread_rng, Rng};
use std::collections::HashSet;

use crate::scene::{MineralMap, MineralType, MAP_HEIGHT, MAP_WIDTH};

use super::{
    AttachmentState, EquipmentPiece, EquipmentSprite, EquipmentSprites,
    EquipmentTreeActions, EquipmentTreeNode, EquipmentTreeState, EquipmentType,
    MinerBody, SamplerBody, SelectedEquipment, SmelterBody, TransportBody,
};

/// Load equipment sprites - generate them programmatically with distinct shapes
pub fn load_equipment_sprites(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut normal_sprites = std::collections::HashMap::new();
    let mut selected_sprites = std::collections::HashMap::new();
    let size = 48; // Larger sprites for more detail

    // Helper to create sprite with custom drawing function
    fn create_sprite<F>(images: &mut ResMut<Assets<Image>>, size: u32, draw_fn: F) -> Handle<Image>
    where
        F: Fn(u32, u32) -> [u8; 4],
    {
        let mut pixel_data = Vec::new();
        for y in 0..size {
            for x in 0..size {
                pixel_data.extend_from_slice(&draw_fn(x, y));
            }
        }

        let image = Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixel_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        images.add(image)
    }

    // Helper to create a selected version with green halo
    fn create_selected_sprite<F>(images: &mut ResMut<Assets<Image>>, size: u32, draw_fn: F) -> Handle<Image>
    where
        F: Fn(u32, u32) -> [u8; 4],
    {
        let halo_size = 60; // Larger to accommodate halo
        let offset = (halo_size - size) / 2; // Center the sprite
        let center = halo_size as f32 / 2.0;
        let equipment_radius = size as f32 / 2.0;
        let halo_radius = halo_size as f32 / 2.0;

        let mut pixel_data = Vec::new();
        for y in 0..halo_size {
            for x in 0..halo_size {
                // Calculate distance from center for halo effect
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let distance = (dx * dx + dy * dy).sqrt();

                // Check if we're in the sprite area
                if x >= offset && x < offset + size && y >= offset && y < offset + size {
                    // Draw the original sprite
                    let sprite_color = draw_fn(x - offset, y - offset);
                    pixel_data.extend_from_slice(&sprite_color);
                } else if distance > equipment_radius && distance <= halo_radius {
                    // Draw green halo
                    let halo_thickness = halo_radius - equipment_radius;
                    let dist_from_edge = distance - equipment_radius;
                    let normalized = 1.0 - (dist_from_edge / halo_thickness);
                    let alpha = (normalized * normalized * 200.0) as u8;
                    pixel_data.extend_from_slice(&[0, 255, 0, alpha]);
                } else {
                    // Transparent
                    pixel_data.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }

        let image = Image::new(
            Extent3d {
                width: halo_size,
                height: halo_size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixel_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        images.add(image)
    }

    // Sampler - Light blue with antenna/dish shape (like a radar)
    let sampler_draw = |x: u32, y: u32| -> [u8; 4] {
            let cx = size / 2;
            let cy = size / 2;
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            let dist = ((dx * dx + dy * dy) as f32).sqrt();

            // Tall antenna extending up
            if x >= cx - 1 && x <= cx + 1 && y < cy - 4 {
                [120, 200, 255, 255]
            }
            // Dish - multiple concentric circles
            else if dist < 16.0 && dist > 14.0 {
                [100, 200, 255, 255] // Outer ring
            } else if dist < 14.0 && dist > 11.0 {
                [70, 170, 235, 255] // Middle ring
            } else if dist < 11.0 && dist > 8.0 {
                [50, 150, 220, 255] // Inner ring
            } else if dist < 8.0 {
                [30, 120, 200, 255] // Center
            }
            // Tripod base
            else if (x > cx - 2 && x < cx + 2 && y > cy + 10)
                || ((x > cx - 10 && x < cx - 6 || x > cx + 6 && x < cx + 10) && y > cy + 14) {
                [80, 80, 80, 255]
            } else {
                [0, 0, 0, 0]
            }
    };
    normal_sprites.insert(EquipmentType::Sampler, create_sprite(&mut images, size, sampler_draw));
    selected_sprites.insert(EquipmentType::Sampler, create_selected_sprite(&mut images, size, sampler_draw));

    // Surface Miner - Orange excavator with bucket and tracks
    let surface_miner_draw = |x: u32, y: u32| -> [u8; 4] {
            let cx = size / 2;

            // Main body
            if x > 10 && x < 38 && y > 14 && y < 28 {
                [255, 180, 80, 255]
            }
            // Cab (driver area)
            else if x > 28 && x < 38 && y > 10 && y < 20 {
                [255, 200, 100, 255]
            }
            // Boom arm extending forward
            else if x > 4 && x < 18 && y > 16 && y < 20 {
                [220, 160, 60, 255]
            }
            // Bucket at the end
            else if x > 0 && x < 10 && y > 18 && y < 26 {
                [200, 140, 40, 255]
            }
            // Tracks (left and right)
            else if (x > 12 && x < 36) && (y > 26 && y < 32 || y > 34 && y < 38) {
                [60, 60, 60, 255]
            }
            // Track details
            else if (x > 12 && x < 36) && y > 32 && y < 34 {
                [40, 40, 40, 255]
            } else {
                [0, 0, 0, 0]
            }
    };
    normal_sprites.insert(EquipmentType::SurfaceMining, create_sprite(&mut images, size, surface_miner_draw));
    selected_sprites.insert(EquipmentType::SurfaceMining, create_selected_sprite(&mut images, size, surface_miner_draw));

    // Deep Miner - Purple drill rig with tall derrick
    let deep_miner_draw = |x: u32, y: u32| -> [u8; 4] {
            let cx = size / 2;

            // Derrick tower structure (tall)
            if (x >= cx - 2 && x <= cx + 2 && y > 2 && y < 20)
                || (x >= cx - 8 && x <= cx + 8 && y > 18 && y < 20) {
                [180, 80, 240, 255]
            }
            // Platform/base
            else if x > 6 && x < 42 && y > 20 && y < 28 {
                [200, 100, 255, 255]
            }
            // Heavy machinery on platform
            else if x > 14 && x < 34 && y > 24 && y < 32 {
                [160, 70, 200, 255]
            }
            // Thick drill pipe going down
            else if x >= cx - 4 && x <= cx + 4 && y >= 28 && y < 46 {
                [140, 50, 180, 255]
            }
            // Drill bit at bottom
            else if x >= cx - 6 && x <= cx + 6 && y >= 42 && y < 46 {
                [100, 30, 140, 255]
            }
            // Support legs
            else if ((x > 8 && x < 12) || (x > 36 && x < 40)) && y > 28 && y < 38 {
                [120, 60, 160, 255]
            } else {
                [0, 0, 0, 0]
            }
    };
    normal_sprites.insert(EquipmentType::DeepMining, create_sprite(&mut images, size, deep_miner_draw));
    selected_sprites.insert(EquipmentType::DeepMining, create_selected_sprite(&mut images, size, deep_miner_draw));

    // Smelter - Red furnace building with smokestacks
    let smelter_draw = |x: u32, y: u32| -> [u8; 4] {
            let cx = size / 2;

            // Main building structure
            if x > 8 && x < 40 && y > 18 && y < 42 {
                [255, 100, 100, 255]
            }
            // Roof
            else if x > 6 && x < 42 && y > 16 && y < 20 {
                [220, 80, 80, 255]
            }
            // Left smokestack
            else if x >= cx - 12 && x <= cx - 6 && y > 4 && y < 18 {
                [200, 70, 70, 255]
            }
            // Right smokestack
            else if x >= cx + 6 && x <= cx + 12 && y > 4 && y < 18 {
                [200, 70, 70, 255]
            }
            // Smoke (darker at top)
            else if ((x >= cx - 14 && x <= cx - 4) || (x >= cx + 4 && x <= cx + 14)) && y > 0 && y < 6 {
                [120, 120, 120, 200]
            }
            // Door
            else if x > 18 && x < 30 && y > 28 && y < 40 {
                [180, 60, 60, 255]
            }
            // Windows
            else if ((x > 12 && x < 16) || (x > 32 && x < 36)) && y > 24 && y < 30 {
                [255, 200, 100, 255]
            } else {
                [0, 0, 0, 0]
            }
    };
    normal_sprites.insert(EquipmentType::Refining, create_sprite(&mut images, size, smelter_draw));
    selected_sprites.insert(EquipmentType::Refining, create_selected_sprite(&mut images, size, smelter_draw));

    // Transport - Green flatbed truck with cargo area
    let transport_draw = |x: u32, y: u32| -> [u8; 4] {
            // Flatbed cargo area
            if x > 4 && x < 32 && y > 16 && y < 26 {
                [100, 255, 100, 255]
            }
            // Cargo sides/rails
            else if (x > 4 && x < 8 || x > 28 && x < 32) && y > 12 && y < 20 {
                [80, 220, 80, 255]
            }
            // Cab
            else if x > 28 && x < 44 && y > 10 && y < 26 {
                [80, 200, 80, 255]
            }
            // Cab windows
            else if x > 32 && x < 42 && y > 12 && y < 18 {
                [150, 220, 255, 255]
            }
            // Front bumper
            else if x > 42 && x < 46 && y > 18 && y < 26 {
                [60, 160, 60, 255]
            }
            // Wheels (large and visible)
            else if ((x > 8 && x < 16) || (x > 30 && x < 38)) && y > 24 && y < 34 {
                [40, 40, 40, 255]
            }
            // Wheel hubs
            else if ((x > 10 && x < 14) || (x > 32 && x < 36)) && y > 26 && y < 32 {
                [80, 80, 80, 255]
            } else {
                [0, 0, 0, 0]
            }
    };
    normal_sprites.insert(EquipmentType::Transport, create_sprite(&mut images, size, transport_draw));
    selected_sprites.insert(EquipmentType::Transport, create_selected_sprite(&mut images, size, transport_draw));

    commands.insert_resource(EquipmentSprites {
        normal: normal_sprites,
        selected: selected_sprites,
    });
}

/// Load attachment sprites - small colored squares for transmitters/receivers/computers
pub fn load_attachment_sprites(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use super::{AttachmentSprites, AttachmentType};

    let mut normal_sprites = std::collections::HashMap::new();
    let mut selected_sprites = std::collections::HashMap::new();
    let size = 20; // Small sprites for attachments

    // Helper to create sprite with custom drawing function
    fn create_sprite<F>(images: &mut ResMut<Assets<Image>>, size: u32, draw_fn: F) -> Handle<Image>
    where
        F: Fn(u32, u32) -> [u8; 4],
    {
        let mut pixel_data = Vec::new();
        for y in 0..size {
            for x in 0..size {
                pixel_data.extend_from_slice(&draw_fn(x, y));
            }
        }

        let image = Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixel_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        images.add(image)
    }

    // Helper to create a selected version with green halo
    fn create_selected_sprite<F>(images: &mut ResMut<Assets<Image>>, size: u32, draw_fn: F) -> Handle<Image>
    where
        F: Fn(u32, u32) -> [u8; 4],
    {
        let halo_size = 28; // Larger to accommodate halo
        let offset = (halo_size - size) / 2;
        let center = halo_size as f32 / 2.0;
        let equipment_radius = size as f32 / 2.0;
        let halo_radius = halo_size as f32 / 2.0;

        let mut pixel_data = Vec::new();
        for y in 0..halo_size {
            for x in 0..halo_size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let distance = (dx * dx + dy * dy).sqrt();

                if x >= offset && x < offset + size && y >= offset && y < offset + size {
                    let sprite_color = draw_fn(x - offset, y - offset);
                    pixel_data.extend_from_slice(&sprite_color);
                } else if distance > equipment_radius && distance <= halo_radius {
                    let halo_thickness = halo_radius - equipment_radius;
                    let dist_from_edge = distance - equipment_radius;
                    let normalized = 1.0 - (dist_from_edge / halo_thickness);
                    let alpha = (normalized * normalized * 200.0) as u8;
                    pixel_data.extend_from_slice(&[0, 255, 0, alpha]);
                } else {
                    pixel_data.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }

        let image = Image::new(
            Extent3d {
                width: halo_size,
                height: halo_size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixel_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        images.add(image)
    }

    // Transmitter - Cyan/blue with antenna pattern
    let transmitter_draw = |x: u32, y: u32| -> [u8; 4] {
        let cx = size / 2;
        let cy = size / 2;
        let dx = x as i32 - cx as i32;
        let dy = y as i32 - cy as i32;
        let dist = ((dx * dx + dy * dy) as f32).sqrt();

        // Antenna in center
        if x >= cx - 1 && x <= cx + 1 && y < cy {
            [100, 200, 255, 255]
        }
        // Circular body
        else if dist < 7.0 {
            [50, 150, 255, 255]
        }
        // Signal waves (concentric rings)
        else if dist > 7.0 && dist < 8.0 {
            [80, 180, 255, 200]
        } else if dist > 9.0 && dist < 9.5 {
            [80, 180, 255, 150]
        } else {
            [0, 0, 0, 0]
        }
    };
    normal_sprites.insert(AttachmentType::Transmitter, create_sprite(&mut images, size, transmitter_draw));
    selected_sprites.insert(AttachmentType::Transmitter, create_selected_sprite(&mut images, size, transmitter_draw));

    // Receiver - Orange/red with dish pattern
    let receiver_draw = |x: u32, y: u32| -> [u8; 4] {
        let cx = size / 2;
        let cy = size / 2;
        let dx = x as i32 - cx as i32;
        let dy = y as i32 - cy as i32;
        let dist = ((dx * dx + dy * dy) as f32).sqrt();

        // Dish (semi-circle)
        if dist < 7.0 && y >= cy {
            [255, 150, 50, 255]
        }
        // Center point
        else if dist < 3.0 {
            [255, 100, 0, 255]
        }
        // Support
        else if x >= cx - 1 && x <= cx + 1 && y > cy - 3 && y < cy {
            [200, 120, 40, 255]
        } else {
            [0, 0, 0, 0]
        }
    };
    normal_sprites.insert(AttachmentType::Receiver, create_sprite(&mut images, size, receiver_draw));
    selected_sprites.insert(AttachmentType::Receiver, create_selected_sprite(&mut images, size, receiver_draw));

    // Computer - Purple with circuit board pattern
    let computer_draw = |x: u32, y: u32| -> [u8; 4] {
        // Chip body (square)
        if x > 3 && x < 17 && y > 3 && y < 17 {
            // Circuit traces
            if (x == 6 || x == 10 || x == 14) && y > 5 && y < 15 {
                [200, 150, 255, 255]
            } else if (y == 6 || y == 10 || y == 14) && x > 5 && x < 15 {
                [200, 150, 255, 255]
            } else {
                [150, 80, 200, 255]
            }
        }
        // Connection pins
        else if (x < 3 || x > 17) && (y == 6 || y == 10 || y == 14) {
            [180, 100, 220, 255]
        } else {
            [0, 0, 0, 0]
        }
    };
    normal_sprites.insert(AttachmentType::Computer, create_sprite(&mut images, size, computer_draw));
    selected_sprites.insert(AttachmentType::Computer, create_selected_sprite(&mut images, size, computer_draw));

    commands.insert_resource(AttachmentSprites {
        normal: normal_sprites,
        selected: selected_sprites,
    });
}

/// System to spawn sprite entities for equipment that doesn't have one yet
pub fn spawn_equipment_sprites(
    mut commands: Commands,
    equipment_state: Res<EquipmentTreeState>,
    equipment_sprites: Res<EquipmentSprites>,
    existing_sprites: Query<&EquipmentSprite>,
) {
    // Get all existing equipment IDs that already have sprites
    let existing_ids: HashSet<usize> = existing_sprites
        .iter()
        .map(|sprite| sprite.equipment_id)
        .collect();

    // Helper function to recursively spawn sprites
    fn spawn_for_node(
        node: &EquipmentTreeNode,
        existing_ids: &HashSet<usize>,
        equipment_sprites: &EquipmentSprites,
        commands: &mut Commands,
    ) {
        // If this is an equipment node (not a container)
        if let Some(equipment_type) = node.equipment_type() {
            if !existing_ids.contains(&node.id) {
                // Equipment needs a sprite
                if let Some(sprite_handle) = equipment_sprites.normal.get(&equipment_type) {
                    let position = node.position.unwrap_or_else(|| {
                        // Random position on map if not set
                        let mut rng = thread_rng();
                        Vec2::new(
                            rng.gen_range(-400.0..400.0),
                            rng.gen_range(-300.0..300.0),
                        )
                    });

                    // Create base EquipmentPiece component
                    let equipment_piece = EquipmentPiece {
                        id: node.id,
                        name: node.name.clone(),
                        mass: 100.0, // Default mass
                        energy_consumption: 5.0, // Default energy consumption
                    };

                    // Create AttachmentState component
                    let attachment_state = AttachmentState::Detached { position };

                    // Spawn entity with appropriate body component based on type
                    let mut entity_commands = commands.spawn((
                        Sprite::from_image(sprite_handle.clone()),
                        Transform::from_translation(position.extend(1.0)),
                        EquipmentSprite {
                            equipment_id: node.id,
                            equipment_type,
                        },
                        equipment_piece,
                        attachment_state,
                    ));

                    // Add the specific body component based on equipment type
                    match equipment_type {
                        EquipmentType::Sampler => {
                            entity_commands.insert(SamplerBody::default());
                        }
                        EquipmentType::SurfaceMining => {
                            entity_commands.insert(MinerBody::surface_miner());
                        }
                        EquipmentType::DeepMining => {
                            entity_commands.insert(MinerBody::deep_miner());
                        }
                        EquipmentType::Refining => {
                            entity_commands.insert(SmelterBody::default());
                        }
                        EquipmentType::Transport => {
                            entity_commands.insert(TransportBody::default());
                        }
                    }
                }
            }
        }

        // Recursively spawn for children
        for child in &node.children {
            spawn_for_node(child, existing_ids, equipment_sprites, commands);
        }
    }

    // Spawn sprites for all equipment nodes in the tree
    for node in &equipment_state.nodes {
        spawn_for_node(node, &existing_ids, &equipment_sprites, &mut commands);
    }
}

/// System to spawn attachment entities (transmitters, receivers, computers)
pub fn spawn_attachments(
    mut commands: Commands,
    mut attachment_manager: ResMut<super::AttachmentManager>,
    attachment_sprites: Res<super::AttachmentSprites>,
    mut spawn_events: EventReader<super::SpawnAttachmentEvent>,
    equipment_query: Query<(&Transform, &EquipmentSprite)>,
) {
    use super::{AttachmentInfo, AttachmentSprite, AttachmentType, Computer, Receiver, Transmitter};

    for event in spawn_events.read() {
        let attachment_id = attachment_manager.next_id;
        attachment_manager.next_id += 1;

        // Calculate offset from parent equipment
        let offset_from_parent = if let Some(parent_id) = event.parent_equipment_id {
            // Find parent equipment position
            let parent_pos = equipment_query
                .iter()
                .find_map(|(transform, sprite)| {
                    if sprite.equipment_id == parent_id {
                        Some(transform.translation.truncate())
                    } else {
                        None
                    }
                })
                .unwrap_or(Vec2::ZERO);

            // Calculate offset: attachment position - parent position
            event.position - parent_pos
        } else {
            Vec2::ZERO
        };

        // Get the appropriate sprite
        if let Some(sprite_handle) = attachment_sprites.normal.get(&event.attachment_type) {
            // Create the attachment entity
            let mut entity_commands = commands.spawn((
                Sprite::from_image(sprite_handle.clone()),
                Transform::from_translation(event.position.extend(2.0)), // Higher z to appear above equipment
                AttachmentSprite {
                    attachment_id,
                    attachment_type: event.attachment_type,
                },
                // Start detached
                super::AttachmentState::Detached {
                    position: event.position,
                },
            ));

            // Add the specific attachment component
            match event.attachment_type {
                AttachmentType::Transmitter => {
                    entity_commands.insert(Transmitter::default());
                }
                AttachmentType::Receiver => {
                    entity_commands.insert(Receiver::default());
                }
                AttachmentType::Computer => {
                    entity_commands.insert(Computer::default());
                }
            }

            // Track the attachment
            attachment_manager.attachments.push(AttachmentInfo {
                id: attachment_id,
                attachment_type: event.attachment_type,
                parent_equipment_id: event.parent_equipment_id,
                offset_from_parent,
            });

            println!(
                "Spawned {} at position ({}, {}) with offset ({}, {})",
                event.attachment_type.name(),
                event.position.x,
                event.position.y,
                offset_from_parent.x,
                offset_from_parent.y
            );
        }
    }
}

/// System to update equipment positions in the state when sprites move
pub fn update_equipment_positions(
    mut equipment_state: ResMut<EquipmentTreeState>,
    mut sprite_query: Query<
        (&Transform, &EquipmentSprite, &mut AttachmentState),
        Changed<Transform>,
    >,
) {
    for (transform, equipment_sprite, mut attachment_state) in &mut sprite_query {
        let position = transform.translation.truncate();

        // Update the tree node position (for backwards compatibility)
        if let Some(node) = equipment_state.find_node_mut(equipment_sprite.equipment_id) {
            node.position = Some(position);
        }

        // Update the AttachmentState position if detached
        if let AttachmentState::Detached { position: ref mut state_pos } = attachment_state.as_mut()
        {
            *state_pos = position;
        }
    }
}

/// System to select equipment or attachments by clicking on them
pub fn click_select_equipment(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    equipment_query: Query<(&Transform, &EquipmentSprite)>,
    attachment_query: Query<(&Transform, &super::AttachmentSprite)>,
    mut selected: ResMut<SelectedEquipment>,
    mut equipment_state: ResMut<EquipmentTreeState>,
    mut equipment_actions: ResMut<EquipmentTreeActions>,
    mut contexts: bevy_egui::EguiContexts,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        // Don't process clicks if egui wants the pointer input
        let ctx = contexts.ctx_mut();
        let wants_pointer = ctx.wants_pointer_input();
        let is_over_area = ctx.is_pointer_over_area();

        if wants_pointer || is_over_area {
            println!("Click ignored - egui wants pointer: {}, is over area: {}", wants_pointer, is_over_area);
            return;
        }
        println!("Processing click - egui doesn't want pointer");

        let Ok(window) = windows.single() else {
            return;
        };
        let Some(cursor_position) = window.cursor_position() else {
            return;
        };

        // Get camera
        let Ok((camera, camera_transform)) = camera_query.single() else {
            return;
        };

        // Convert screen position to world position
        let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
        else {
            return;
        };

        // Check if we clicked on any attachment or equipment
        // Check attachments first (they're smaller and should have priority)
        let mut clicked_id: Option<usize> = None;
        let attachment_size = 24.0; // Attachment sprite click radius

        for (transform, attachment_sprite) in &attachment_query {
            let sprite_pos = transform.translation.truncate();
            let distance = world_position.distance(sprite_pos);

            if distance < attachment_size {
                clicked_id = Some(attachment_sprite.attachment_id);
                break;
            }
        }

        // If no attachment clicked, check equipment
        if clicked_id.is_none() {
            let sprite_size = 64.0; // Equipment sprite click radius (increased for easier clicking)

            for (transform, equipment_sprite) in &equipment_query {
                let sprite_pos = transform.translation.truncate();
                let distance = world_position.distance(sprite_pos);

                if distance < sprite_size {
                    clicked_id = Some(equipment_sprite.equipment_id);
                    break;
                }
            }
        }

        // Update selection in both resources
        selected.selected_id = clicked_id;

        if let Some(id) = clicked_id {
            println!("Selected equipment/attachment with ID: {}", id);
        } else {
            println!("Deselected (clicked empty space)");
        }

        // Clear previous selection and set new one in equipment_actions
        equipment_actions.selected.clear();
        if let Some(id) = clicked_id {
            equipment_actions.selected.insert(id);
        }

        // Activate/deactivate equipment - helper function to recursively update
        fn update_active_state(node: &mut EquipmentTreeNode, active_id: usize) {
            node.active = node.id == active_id;
            for child in &mut node.children {
                update_active_state(child, active_id);
            }
        }

        if let Some(id) = clicked_id {
            for node in &mut equipment_state.nodes {
                update_active_state(node, id);
            }
        }
    }
}

/// System to move selected equipment or attachment with arrow keys
pub fn move_selected_equipment(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    selected: Res<SelectedEquipment>,
    mut equipment_sprite_query: Query<(&mut Transform, &EquipmentSprite), Without<super::AttachmentSprite>>,
    mut attachment_sprite_query: Query<(&mut Transform, &super::AttachmentSprite), Without<EquipmentSprite>>,
) {
    let Some(selected_id) = selected.selected_id else {
        return;
    };

    let move_speed = 200.0 * time.delta_secs();

    // Try to move equipment first
    let mut found = false;
    for (mut transform, equipment_sprite) in &mut equipment_sprite_query {
        if equipment_sprite.equipment_id == selected_id {
            // Move with arrow keys
            if keyboard.pressed(KeyCode::ArrowUp) {
                transform.translation.y += move_speed;
            }
            if keyboard.pressed(KeyCode::ArrowDown) {
                transform.translation.y -= move_speed;
            }
            if keyboard.pressed(KeyCode::ArrowLeft) {
                transform.translation.x -= move_speed;
            }
            if keyboard.pressed(KeyCode::ArrowRight) {
                transform.translation.x += move_speed;
            }
            found = true;
            break;
        }
    }

    // If not equipment, try attachments
    if !found {
        for (mut transform, attachment_sprite) in &mut attachment_sprite_query {
            if attachment_sprite.attachment_id == selected_id {
                // Move with arrow keys
                if keyboard.pressed(KeyCode::ArrowUp) {
                    transform.translation.y += move_speed;
                }
                if keyboard.pressed(KeyCode::ArrowDown) {
                    transform.translation.y -= move_speed;
                }
                if keyboard.pressed(KeyCode::ArrowLeft) {
                    transform.translation.x -= move_speed;
                }
                if keyboard.pressed(KeyCode::ArrowRight) {
                    transform.translation.x += move_speed;
                }
                break;
            }
        }
    }
}

/// System to swap sprites between normal and selected versions based on selection
pub fn update_selection_sprites(
    selected: Res<SelectedEquipment>,
    equipment_sprites: Res<EquipmentSprites>,
    attachment_sprites: Res<super::AttachmentSprites>,
    mut equipment_sprite_query: Query<(&mut Sprite, &EquipmentSprite), Without<super::AttachmentSprite>>,
    mut attachment_sprite_query: Query<(&mut Sprite, &super::AttachmentSprite), Without<EquipmentSprite>>,
) {
    // Only update when selection changes
    if !selected.is_changed() {
        return;
    }

    let selected_id = selected.selected_id;

    // Update all equipment sprites
    for (mut sprite, equipment_sprite) in equipment_sprite_query.iter_mut() {
        let is_selected = Some(equipment_sprite.equipment_id) == selected_id;

        // Choose the appropriate sprite (normal or selected)
        let sprite_handle = if is_selected {
            equipment_sprites.selected.get(&equipment_sprite.equipment_type)
        } else {
            equipment_sprites.normal.get(&equipment_sprite.equipment_type)
        };

        // Swap the sprite image
        if let Some(handle) = sprite_handle {
            sprite.image = handle.clone();
        }
    }

    // Update all attachment sprites
    for (mut sprite, attachment_sprite) in attachment_sprite_query.iter_mut() {
        let is_selected = Some(attachment_sprite.attachment_id) == selected_id;

        // Choose the appropriate sprite (normal or selected)
        let sprite_handle = if is_selected {
            attachment_sprites.selected.get(&attachment_sprite.attachment_type)
        } else {
            attachment_sprites.normal.get(&attachment_sprite.attachment_type)
        };

        // Swap the sprite image
        if let Some(handle) = sprite_handle {
            sprite.image = handle.clone();
        }
    }
}

/// System to delete selected attachment with Delete/Backspace key
pub fn delete_selected_attachment(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut attachment_manager: ResMut<super::AttachmentManager>,
    attachment_query: Query<(Entity, &super::AttachmentSprite)>,
    mut commands: Commands,
) {
    // Only process on Delete or Backspace press
    if !keyboard.just_pressed(KeyCode::Delete) && !keyboard.just_pressed(KeyCode::Backspace) {
        return;
    }

    let Some(selected_id) = selected.selected_id else {
        return;
    };

    // Find the attachment entity with this ID
    for (entity, attachment_sprite) in &attachment_query {
        if attachment_sprite.attachment_id == selected_id {
            // Remove from attachment manager
            attachment_manager.attachments.retain(|a| a.id != selected_id);

            // Despawn the entity
            commands.entity(entity).despawn();

            println!("Deleted attachment #{}", selected_id);
            break;
        }
    }
}

/// System to update attachment positions to follow their parent equipment
pub fn update_attachment_positions(
    attachment_manager: Res<super::AttachmentManager>,
    equipment_query: Query<(&Transform, &EquipmentSprite)>,
    mut attachment_query: Query<(&mut Transform, &super::AttachmentSprite), Without<EquipmentSprite>>,
) {
    // For each attachment
    for (mut attachment_transform, attachment_sprite) in &mut attachment_query {
        // Find the attachment info to get the parent equipment ID and offset
        if let Some(attachment_info) = attachment_manager
            .attachments
            .iter()
            .find(|a| a.id == attachment_sprite.attachment_id)
        {
            // If this attachment has a parent
            if let Some(parent_id) = attachment_info.parent_equipment_id {
                // Find the parent equipment entity
                for (equipment_transform, equipment_sprite) in &equipment_query {
                    if equipment_sprite.equipment_id == parent_id {
                        // Update attachment position: parent position + stored offset
                        let new_position = equipment_transform.translation.truncate() + attachment_info.offset_from_parent;
                        attachment_transform.translation = new_position.extend(2.0);
                        break;
                    }
                }
            }
        }
    }
}

/// System for equipment to mine nearby cells
pub fn equipment_mining_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    sprite_query: Query<(&Transform, &EquipmentSprite, &MinerBody)>,
    mut mineral_map: ResMut<MineralMap>,
) {
    // Press M to mine with active equipment
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }

    println!("M key pressed - checking for mining equipment...");

    // Find all active mining equipment with MinerBody component
    for (transform, _equipment_sprite, miner_body) in sprite_query.iter() {
        // Get equipment position in world space
        let world_pos = transform.translation.truncate();

        println!(
            "Mining with {} (depth: {:.0}-{:.0}, radius: {:.1}) at world pos: {:?}",
            match miner_body.equipment_type {
                EquipmentType::SurfaceMining => "Surface Miner",
                EquipmentType::DeepMining => "Deep Miner",
                _ => "Unknown Miner",
            },
            miner_body.depth_range.0,
            miner_body.depth_range.1,
            miner_body.mining_radius,
            world_pos
        );

        // Convert to map coordinates (accounting for 2x scale of map sprite)
        // Map is centered at (0, 0) in world space
        // Flip Y because image coordinates go down but world coordinates go up
        let map_x = ((world_pos.x / 2.0) + (MAP_WIDTH as f32 / 2.0)) as i32;
        let map_y = ((MAP_HEIGHT as f32 / 2.0) - (world_pos.y / 2.0)) as i32;

        println!("Map coordinates: x={}, y={}", map_x, map_y);

        // Use mining radius from the miner body
        let mining_radius = miner_body.mining_radius as i32;

        let mut cells_mined = 0;
        let mut cells_out_of_range = 0;

        for dy in -mining_radius..=mining_radius {
            for dx in -mining_radius..=mining_radius {
                let x = map_x + dx;
                let y = map_y + dy;

                if x >= 0 && x < MAP_WIDTH as i32 && y >= 0 && y < MAP_HEIGHT as i32 {
                    let idx = y as usize * MAP_WIDTH + x as usize;
                    let current_height = mineral_map.heightmap[idx];

                    // Check if this depth is within the miner's working range
                    if current_height < miner_body.depth_range.0
                        || current_height > miner_body.depth_range.1
                    {
                        cells_out_of_range += 1;
                        continue; // Skip cells outside depth range
                    }

                    cells_mined += 1;

                    // Gradually reduce height by mining using extraction rate
                    mineral_map.heightmap[idx] =
                        (mineral_map.heightmap[idx] - miner_body.extraction_rate * 5.0).max(0.0);

                    // If height is depleted, remove the material
                    if mineral_map.heightmap[idx] <= 0.5 {
                        if let Some(cell) = mineral_map.get_mut(x as usize, y as usize) {
                            cell.mineral_type = MineralType::Empty;
                            cell.mined = true;
                            cell.density = 0.0;
                            mineral_map.heightmap[idx] = 0.0;
                        }
                    }
                }
            }
        }

        println!(
            "Mined {} units, {} units out of depth range ({:.0}-{:.0})",
            cells_mined, cells_out_of_range, miner_body.depth_range.0, miner_body.depth_range.1
        );
    }
}