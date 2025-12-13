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
                            entity_commands.insert(TransportBody::truck());
                        }
                    }

                    // All equipment has mount points for attachments
                    entity_commands.insert((
                        super::HasCommunicationMount,
                        super::HasControlMount,
                        super::HasUtilityMount,
                    ));
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
    mut mount_occupancy: ResMut<super::MountPointOccupancy>,
    attachment_sprites: Res<super::AttachmentSprites>,
    mut spawn_events: EventReader<super::SpawnAttachmentEvent>,
    equipment_query: Query<(Entity, &Transform, &EquipmentSprite)>,
) {
    use super::{AttachmentInfo, AttachmentSprite, AttachmentType, Computer, Receiver, Transmitter};

    for event in spawn_events.read() {
        let attachment_id = attachment_manager.next_id;
        attachment_manager.next_id += 1;

        // Use standard offset based on attachment type
        let offset_from_parent = event.attachment_type.standard_offset();

        // Get the appropriate sprite
        if let Some(sprite_handle) = attachment_sprites.normal.get(&event.attachment_type) {
            // Find parent equipment entity if attaching
            if let Some(parent_id) = event.parent_equipment_id {
                // Check if the mount point is available
                let mount_type = event.attachment_type.mount_type();
                if !mount_occupancy.is_mount_available(parent_id, mount_type) {
                    println!("Cannot attach {}: {} mount point on equipment {} is occupied",
                        event.attachment_type.name(),
                        match mount_type {
                            super::MountType::Communication => "Communication",
                            super::MountType::Control => "Control",
                        },
                        parent_id
                    );
                    continue; // Skip this attachment
                }

                if let Some((parent_entity, _transform, _sprite)) = equipment_query
                    .iter()
                    .find(|(_, _, sprite)| sprite.equipment_id == parent_id)
                {
                    // Spawn as child of equipment entity
                    commands.entity(parent_entity).with_children(|parent| {
                        let mut child = parent.spawn((
                            Sprite::from_image(sprite_handle.clone()),
                            Transform::from_translation(offset_from_parent.extend(1.0)), // Offset relative to parent
                            AttachmentSprite {
                                attachment_id,
                                attachment_type: event.attachment_type,
                            },
                            super::AttachmentState::AttachedToEquipment { parent: parent_entity },
                        ));

                        // Add the specific attachment component
                        match event.attachment_type {
                            AttachmentType::Transmitter => {
                                child.insert(Transmitter::default());
                            }
                            AttachmentType::Receiver => {
                                child.insert(Receiver::default());
                            }
                            AttachmentType::Computer => {
                                child.insert(Computer::default());
                            }
                        }
                    });

                    // Occupy the mount point (should always succeed since we checked availability)
                    if !mount_occupancy.occupy_mount(parent_id, mount_type, attachment_id) {
                        println!("ERROR: Failed to occupy mount after availability check!");
                    }

                    // Track the attachment
                    attachment_manager.attachments.push(AttachmentInfo {
                        id: attachment_id,
                        attachment_type: event.attachment_type,
                        parent_equipment_id: Some(parent_id),
                        offset_from_parent,
                    });
                }
            } else {
                // Spawn as standalone detached entity
                let mut entity_commands = commands.spawn((
                    Sprite::from_image(sprite_handle.clone()),
                    Transform::from_translation(event.position.extend(2.0)),
                    AttachmentSprite {
                        attachment_id,
                        attachment_type: event.attachment_type,
                    },
                    super::AttachmentState::Detached { position: event.position },
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
                    parent_equipment_id: None,
                    offset_from_parent,
                });
            }
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
    attachment_query: Query<(&GlobalTransform, &super::AttachmentSprite)>,
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
            return;
        }

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

        for (global_transform, attachment_sprite) in &attachment_query {
            let sprite_pos = global_transform.translation().truncate();
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
        selected.selected_ids.clear();
        if let Some(id) = clicked_id {
            selected.selected_ids.insert(id);
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

/// System to move selected equipment or attachment with arrow keys (supports multi-selection)
pub fn move_selected_equipment(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    selected: Res<SelectedEquipment>,
    mut equipment_sprite_query: Query<(&mut Transform, &EquipmentSprite), Without<super::AttachmentSprite>>,
    mut attachment_sprite_query: Query<(&mut Transform, &super::AttachmentSprite), Without<EquipmentSprite>>,
    transport_query: Query<(&EquipmentSprite, &TransportBody)>,
) {
    if selected.selected_ids.is_empty() {
        return;
    }

    let base_move_speed = 200.0;

    // Calculate movement direction
    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction == Vec2::ZERO {
        return;
    }

    // Move all selected equipment
    for (mut transform, equipment_sprite) in &mut equipment_sprite_query {
        if selected.selected_ids.contains(&equipment_sprite.equipment_id) {
            // Check if this equipment has a TransportBody with speed multiplier
            let speed_multiplier = transport_query
                .iter()
                .find_map(|(sprite, transport)| {
                    if sprite.equipment_id == equipment_sprite.equipment_id {
                        Some(transport.speed)
                    } else {
                        None
                    }
                })
                .unwrap_or(1.0);

            let delta = direction * base_move_speed * speed_multiplier * time.delta_secs();
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
        }
    }

    // Move all selected attachments (use base speed without multiplier)
    let delta = direction * base_move_speed * time.delta_secs();
    for (mut transform, attachment_sprite) in &mut attachment_sprite_query {
        if selected.selected_ids.contains(&attachment_sprite.attachment_id) {
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
        }
    }
}

/// System to swap sprites between normal and selected versions based on selection (supports multi-selection)
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

    // Update all equipment sprites
    for (mut sprite, equipment_sprite) in equipment_sprite_query.iter_mut() {
        let is_selected = selected.selected_ids.contains(&equipment_sprite.equipment_id);

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
        let is_selected = selected.selected_ids.contains(&attachment_sprite.attachment_id);

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

/// System to delete selected attachments with Delete/Backspace key (supports multi-selection)
pub fn delete_selected_attachment(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut attachment_manager: ResMut<super::AttachmentManager>,
    mut mount_occupancy: ResMut<super::MountPointOccupancy>,
    attachment_query: Query<(Entity, &super::AttachmentSprite)>,
    mut commands: Commands,
) {
    // Only process on Delete or Backspace press
    if !keyboard.just_pressed(KeyCode::Delete) && !keyboard.just_pressed(KeyCode::Backspace) {
        return;
    }

    if selected.selected_ids.is_empty() {
        return;
    }

    // Find and delete all selected attachment entities
    for (entity, attachment_sprite) in &attachment_query {
        if selected.selected_ids.contains(&attachment_sprite.attachment_id) {
            // Free the mount point if attached
            if let Some(attachment_info) = attachment_manager
                .attachments
                .iter()
                .find(|a| a.id == attachment_sprite.attachment_id)
            {
                if let Some(parent_id) = attachment_info.parent_equipment_id {
                    mount_occupancy.free_attachment(parent_id, attachment_sprite.attachment_id);
                }
            }

            // Remove from attachment manager
            attachment_manager.attachments.retain(|a| a.id != attachment_sprite.attachment_id);

            // Despawn the entity
            commands.entity(entity).despawn();
        }
    }
}

/// System to attach selected attachments to nearby compatible equipment with 'T' key
pub fn attach_selected_attachment(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut commands: Commands,
    mut attachment_manager: ResMut<super::AttachmentManager>,
    mut mount_occupancy: ResMut<super::MountPointOccupancy>,
    attachment_query: Query<(Entity, &Transform, &super::AttachmentState, &super::AttachmentSprite)>,
    equipment_query: Query<(Entity, &Transform, &EquipmentSprite, Option<&super::HasCommunicationMount>, Option<&super::HasControlMount>, Option<&super::HasUtilityMount>)>,
) {
    use super::AttachmentType;

    // Only process on 'T' key press
    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }

    if selected.selected_ids.is_empty() {
        return;
    }

    const MAX_ATTACH_DISTANCE: f32 = 20.0;

    // Attach all selected detached attachments
    for (attachment_entity, attachment_transform, attachment_state, attachment_sprite) in &attachment_query {
        if selected.selected_ids.contains(&attachment_sprite.attachment_id) {
            // Check if it's currently detached
            if matches!(*attachment_state, super::AttachmentState::Detached { .. }) {
                // Get attachment info to find its type
                if let Some(attachment_info) = attachment_manager
                    .attachments
                    .iter()
                    .find(|a| a.id == attachment_sprite.attachment_id)
                {
                    let attachment_pos = attachment_transform.translation.truncate();
                    let attachment_type = attachment_info.attachment_type;

                    // Find nearest compatible equipment within range
                    let mut nearest_equipment: Option<(Entity, usize, f32, Vec2)> = None;

                    for (equipment_entity, equipment_transform, equipment_sprite, has_comm, has_control, _has_utility) in &equipment_query {
                        let equipment_pos = equipment_transform.translation.truncate();
                        let distance = attachment_pos.distance(equipment_pos);

                        if distance > MAX_ATTACH_DISTANCE {
                            continue;
                        }

                        // Check if equipment has compatible mount point
                        let is_compatible = match attachment_type {
                            AttachmentType::Transmitter | AttachmentType::Receiver => has_comm.is_some(),
                            AttachmentType::Computer => has_control.is_some(),
                        };

                        if !is_compatible {
                            continue;
                        }

                        // Update nearest if this is closer
                        if nearest_equipment.is_none() || distance < nearest_equipment.as_ref().unwrap().2 {
                            nearest_equipment = Some((equipment_entity, equipment_sprite.equipment_id, distance, equipment_pos));
                        }
                    }

                    // If we found compatible equipment, attach to it
                    if let Some((equipment_entity, equipment_id, _distance, equipment_pos)) = nearest_equipment {
                        // Check if the mount point is available
                        let mount_type = attachment_type.mount_type();
                        if !mount_occupancy.is_mount_available(equipment_id, mount_type) {
                            println!("Cannot attach {}: {} mount point on equipment {} is occupied",
                                attachment_type.name(),
                                match mount_type {
                                    super::MountType::Communication => "Communication",
                                    super::MountType::Control => "Control",
                                },
                                equipment_id
                            );
                            continue; // Skip this attachment
                        }

                        // Calculate offset from parent
                        let offset = attachment_pos - equipment_pos;

                        // Set parent relationship and update transform to be relative
                        commands.entity(attachment_entity)
                            .insert((
                                ChildOf(equipment_entity),
                                Transform::from_translation(offset.extend(1.0))
                            ));

                        // Change state to Attached
                        commands.entity(attachment_entity).insert(
                            super::AttachmentState::AttachedToEquipment {
                                parent: equipment_entity,
                            }
                        );

                        // Occupy the mount point (should always succeed since we checked availability)
                        if !mount_occupancy.occupy_mount(equipment_id, mount_type, attachment_sprite.attachment_id) {
                            println!("ERROR: Failed to occupy mount after availability check!");
                        }

                        // Update attachment manager
                        if let Some(attachment_info) = attachment_manager
                            .attachments
                            .iter_mut()
                            .find(|a| a.id == attachment_sprite.attachment_id)
                        {
                            attachment_info.parent_equipment_id = Some(equipment_id);
                            attachment_info.offset_from_parent = offset;
                        }
                    }
                }
            }
        }
    }
}

/// System to detach selected attachments with 'X' key (supports multi-selection)
pub fn detach_selected_attachment(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut commands: Commands,
    mut attachment_manager: ResMut<super::AttachmentManager>,
    mut mount_occupancy: ResMut<super::MountPointOccupancy>,
    attachment_query: Query<(Entity, &GlobalTransform, &super::AttachmentState, &super::AttachmentSprite)>,
) {
    // Only process on 'X' key press
    if !keyboard.just_pressed(KeyCode::KeyX) {
        return;
    }

    if selected.selected_ids.is_empty() {
        return;
    }

    // Detach all selected attachments
    for (entity, global_transform, attachment_state, attachment_sprite) in &attachment_query {
        if selected.selected_ids.contains(&attachment_sprite.attachment_id) {
            // Check if it's currently attached
            if matches!(*attachment_state, super::AttachmentState::AttachedToEquipment { .. }) {
                // Get current global position
                let current_pos = global_transform.translation().truncate();

                // Remove parent relationship and set new global transform
                commands.entity(entity)
                    .remove::<ChildOf>()
                    .insert(Transform::from_translation(current_pos.extend(2.0)));

                // Change state to Detached
                commands.entity(entity).insert(super::AttachmentState::Detached {
                    position: current_pos,
                });

                // Update attachment manager to remove parent and free mount
                if let Some(attachment_info) = attachment_manager
                    .attachments
                    .iter_mut()
                    .find(|a| a.id == attachment_sprite.attachment_id)
                {
                    // Free the mount point before clearing parent
                    if let Some(parent_id) = attachment_info.parent_equipment_id {
                        mount_occupancy.free_attachment(parent_id, attachment_sprite.attachment_id);
                    }

                    attachment_info.parent_equipment_id = None;
                }
            }
        }
    }
}

/// System to sync outliner selection to SelectedEquipment resource
/// When a container is selected, all its children are also selected
pub fn sync_outliner_selection(
    equipment_actions: Res<EquipmentTreeActions>,
    equipment_tree: Res<super::EquipmentTreeState>,
    mut selected: ResMut<SelectedEquipment>,
) {
    // If there's a change in outliner selection, sync it
    if equipment_actions.is_changed() {
        // Start with the selected IDs from the outliner
        let mut all_selected_ids = equipment_actions.selected.clone();

        // For each selected ID, if it's a container, add all its descendants
        for selected_id in equipment_actions.selected.iter() {
            // Find the node in the tree
            for root in &equipment_tree.nodes {
                if let Some(node) = root.find_node(*selected_id) {
                    // If this node is a container, collect all its descendants
                    if node.is_container() {
                        node.collect_all_descendant_ids(&mut all_selected_ids);
                    }
                    break;
                }
            }
        }

        selected.selected_ids = all_selected_ids;
    }
}

/// System to update attachment positions - no longer needed!
/// Attachments are now children of equipment entities, so they automatically follow
/// their parent via Bevy's transform propagation system. This eliminates the lag.
pub fn update_attachment_positions() {
    // This system is kept for backwards compatibility but does nothing.
    // Child entities automatically follow their parent transforms.
}

/// System for equipment to mine nearby cells
pub fn equipment_mining_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sprite_query: Query<(&Transform, &EquipmentSprite, &mut MinerBody)>,
    mut mineral_map: ResMut<MineralMap>,
) {
    // Press M to mine with active equipment
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }

    println!("M key pressed - checking for mining equipment...");

    // Find all active mining equipment with MinerBody component
    for (transform, _equipment_sprite, mut miner_body) in sprite_query.iter_mut() {
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

        // Determine mining behavior based on equipment type
        let (mining_radius, layers_to_dig) = match miner_body.equipment_type {
            EquipmentType::SurfaceMining => {
                // Surface miner: expands radius, digs 1 layer
                let radius = miner_body.current_radius as i32;
                println!("Surface miner - mining at radius {} (expanding)", radius);
                (radius, 1)
            },
            EquipmentType::DeepMining => {
                // Deep miner: fixed small radius, digs 3 layers
                let radius = miner_body.current_radius as i32;
                println!("Deep miner - mining at radius {} (digging {} layers deep)", radius, 3);
                (radius, 3)
            },
            _ => (miner_body.mining_radius as i32, 1),
        };

        let mut cells_mined = 0;
        let mut cells_out_of_range = 0;
        let mut minerals_extracted: std::collections::HashMap<MineralType, f32> = std::collections::HashMap::new();
        let mut capacity_reached = false;

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

                    // Get the mineral type before mining
                    let mineral_type = if let Some(cell) = mineral_map.get(x as usize, y as usize) {
                        if cell.mineral_type == MineralType::Empty {
                            continue; // Skip empty cells
                        }
                        cell.mineral_type
                    } else {
                        continue;
                    };

                    // Calculate extraction amount (mass in kg) based on layers
                    let extraction_amount = miner_body.extraction_rate * 5.0 * layers_to_dig as f32;

                    // Check if we have capacity to store it
                    if !miner_body.can_add(extraction_amount) {
                        capacity_reached = true;
                        break; // Stop mining when full
                    }

                    cells_mined += 1;

                    // Add to inventory
                    let amount_added = miner_body.add_to_inventory(mineral_type, extraction_amount);
                    *minerals_extracted.entry(mineral_type).or_insert(0.0) += amount_added;

                    // Reduce height by mining - more layers = deeper digging
                    mineral_map.heightmap[idx] =
                        (mineral_map.heightmap[idx] - miner_body.extraction_rate * 5.0 * layers_to_dig as f32).max(0.0);

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
            if capacity_reached {
                break;
            }
        }

        // Print mining results
        println!(
            "Mined {} cells, {} cells out of depth range ({:.0}-{:.0})",
            cells_mined, cells_out_of_range, miner_body.depth_range.0, miner_body.depth_range.1
        );

        // Print extracted minerals
        if !minerals_extracted.is_empty() {
            println!("Extracted minerals:");
            for (mineral_type, amount) in minerals_extracted.iter() {
                println!("  {:?}: {:.1} kg", mineral_type, amount);
            }
        }

        // Print inventory status
        println!(
            "Inventory: {:.1}/{:.1} kg ({:.1}% full)",
            miner_body.current_inventory_mass(),
            miner_body.capacity,
            (miner_body.current_inventory_mass() / miner_body.capacity) * 100.0
        );

        if capacity_reached {
            println!("WARNING: Inventory full! Cannot mine more.");
        }

        // Expand radius for surface miners
        if miner_body.equipment_type == EquipmentType::SurfaceMining {
            miner_body.current_radius += miner_body.mining_radius;
            println!("Surface miner radius expanded to {:.1}", miner_body.current_radius);
        }
    }
}

/// System to load nearby detached equipment and attachments into selected truck
pub fn load_into_truck(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut commands: Commands,
    mut truck_query: Query<(Entity, &Transform, &EquipmentSprite, &mut TransportBody)>,
    attachment_query: Query<(Entity, &GlobalTransform, &super::AttachmentSprite, &super::AttachmentState), Without<EquipmentSprite>>,
    equipment_query: Query<(Entity, &GlobalTransform, &EquipmentSprite, &super::AttachmentState), Without<super::AttachmentSprite>>,
) {
    // Press L to load
    if !keyboard.just_pressed(KeyCode::KeyL) {
        return;
    }

    println!("L key pressed - checking for loadable items...");

    // Iterate through trucks directly
    for (truck_entity, transform, truck_sprite, mut transport) in truck_query.iter_mut() {
        // Check if this truck is selected
        if !selected.selected_ids.contains(&truck_sprite.equipment_id) {
            continue;
        }

        let truck_pos = transform.translation.truncate();
        let load_radius = 50.0; // Load range
        let max_cargo_items = 10; // Maximum items that can be carried
        let mut loaded = false;

        // Check if truck is already at capacity
        if transport.cargo.len() >= max_cargo_items {
            println!("Truck {} is at full capacity ({}/{} items)", truck_sprite.equipment_id, transport.cargo.len(), max_cargo_items);
            continue;
        }

        // Find nearby detached attachments
        for (entity, global_transform, attachment_sprite, attachment_state) in attachment_query.iter() {
            // Only load detached items
            if !matches!(attachment_state, AttachmentState::Detached { .. }) {
                continue;
            }

            let item_pos = global_transform.translation().truncate();
            let distance = truck_pos.distance(item_pos);

            if distance <= load_radius {
                println!("Loading attachment {} into truck {}", attachment_sprite.attachment_id, truck_sprite.equipment_id);

                // Add to cargo
                transport.cargo.push(entity);

                // Update state to InCargo
                let cargo_index = transport.cargo.len() - 1;
                commands.entity(entity).insert(AttachmentState::InCargo {
                    truck: truck_entity,
                    cargo_index,
                });

                loaded = true;
                break; // Load one at a time per truck
            }
        }

        // If didn't load an attachment, try loading equipment
        if !loaded {
            for (entity, global_transform, equipment_sprite, equipment_state) in equipment_query.iter() {
                // Don't load the truck itself or other trucks
                if equipment_sprite.equipment_id == truck_sprite.equipment_id {
                    continue;
                }

                // Only load detached items
                if !matches!(equipment_state, AttachmentState::Detached { .. }) {
                    continue;
                }

                let item_pos = global_transform.translation().truncate();
                let distance = truck_pos.distance(item_pos);

                if distance <= load_radius {
                    println!("Loading equipment {} into truck {}", equipment_sprite.equipment_id, truck_sprite.equipment_id);

                    // Add to cargo
                    transport.cargo.push(entity);

                    // Update state to InCargo
                    let cargo_index = transport.cargo.len() - 1;
                    commands.entity(entity).insert(AttachmentState::InCargo {
                        truck: truck_entity,
                        cargo_index,
                    });

                    break; // Load one at a time per truck
                }
            }
        }
    }
}

/// System to unload cargo from selected truck
pub fn unload_from_truck(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedEquipment>,
    mut commands: Commands,
    mut truck_query: Query<(&Transform, &EquipmentSprite, &mut TransportBody)>,
) {
    // Press U to unload
    if !keyboard.just_pressed(KeyCode::KeyU) {
        return;
    }

    // Find selected trucks
    for id in &selected.selected_ids {
        if let Some((transform, _sprite, mut transport)) = truck_query.iter_mut().find_map(|(t, s, trans)| {
            if s.equipment_id == *id {
                Some((t, s, trans))
            } else {
                None
            }
        }) {
            if transport.cargo.is_empty() {
                println!("Truck {} has no cargo to unload", id);
                continue;
            }

            // Unload most recent cargo
            if let Some(cargo_entity) = transport.cargo.pop() {
                let truck_pos = transform.translation.truncate();
                println!("Unloading cargo from truck {}", id);

                // Update state to Detached at truck's position
                commands.entity(cargo_entity).insert(AttachmentState::Detached {
                    position: truck_pos,
                });

                // Update transform and restore full opacity
                commands.entity(cargo_entity).insert((
                    Transform::from_translation(truck_pos.extend(2.0)),
                    Visibility::Visible,
                ));
            }
        }
    }
}

/// System to update visual appearance of cargo items (transparency and positioning)
pub fn update_cargo_visuals(
    truck_query: Query<(&Transform, &TransportBody)>,
    mut cargo_query: Query<(&mut Transform, &mut Visibility, &AttachmentState), Without<TransportBody>>,
) {
    // Update all cargo items
    for (mut cargo_transform, mut visibility, attachment_state) in cargo_query.iter_mut() {
        if let AttachmentState::InCargo { truck, cargo_index } = attachment_state {
            // Find the truck
            if let Ok((truck_transform, _transport)) = truck_query.get(*truck) {
                // Position cargo under truck with slight offset based on cargo index
                let offset = Vec2::new(0.0, -10.0 * (*cargo_index as f32 + 1.0));
                let cargo_pos = truck_transform.translation.truncate() + offset;

                cargo_transform.translation = cargo_pos.extend(0.5); // Lower z-index
                *visibility = Visibility::Hidden; // Hide cargo when in truck
            }
        }
    }
}