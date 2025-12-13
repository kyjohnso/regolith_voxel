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
    MinerBody, SamplerBody, SelectedEquipment, SelectionOutline, SmelterBody, TransportBody,
};

/// Load equipment sprites - generate them programmatically
pub fn load_equipment_sprites(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut sprites = std::collections::HashMap::new();

    // Helper to create a colored square sprite
    fn create_colored_sprite(images: &mut ResMut<Assets<Image>>, color: [u8; 4]) -> Handle<Image> {
        let size = 32;
        let mut pixel_data = Vec::new();
        for y in 0..size {
            for x in 0..size {
                // Create a border effect
                if x < 2 || x >= size - 2 || y < 2 || y >= size - 2 {
                    // Border - slightly darker
                    pixel_data.extend_from_slice(&[
                        (color[0] as f32 * 0.7) as u8,
                        (color[1] as f32 * 0.7) as u8,
                        (color[2] as f32 * 0.7) as u8,
                        color[3],
                    ]);
                } else {
                    // Inner color
                    pixel_data.extend_from_slice(&color);
                }
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

    // Create colored sprites for each equipment type
    sprites.insert(
        EquipmentType::Sampler,
        create_colored_sprite(&mut images, [100, 200, 255, 255]), // Light blue
    );
    sprites.insert(
        EquipmentType::SurfaceMining,
        create_colored_sprite(&mut images, [255, 200, 100, 255]), // Orange
    );
    sprites.insert(
        EquipmentType::DeepMining,
        create_colored_sprite(&mut images, [200, 100, 255, 255]), // Purple
    );
    sprites.insert(
        EquipmentType::Refining,
        create_colored_sprite(&mut images, [255, 100, 100, 255]), // Red
    );
    sprites.insert(
        EquipmentType::Transport,
        create_colored_sprite(&mut images, [100, 255, 100, 255]), // Green
    );

    commands.insert_resource(EquipmentSprites { sprites });
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
                if let Some(sprite_handle) = equipment_sprites.sprites.get(&equipment_type) {
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
                        },
                        equipment_piece,
                        attachment_state,
                    ));

                    // Add the specific body component based on equipment type
                    match equipment_type {
                        EquipmentType::Sampler => {
                            entity_commands.insert(SamplerBody);
                        }
                        EquipmentType::SurfaceMining | EquipmentType::DeepMining => {
                            entity_commands.insert(MinerBody { equipment_type });
                        }
                        EquipmentType::Refining => {
                            entity_commands.insert(SmelterBody);
                        }
                        EquipmentType::Transport => {
                            entity_commands.insert(TransportBody);
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

/// System to select equipment by clicking on them
pub fn click_select_equipment(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    equipment_query: Query<(&Transform, &EquipmentSprite)>,
    mut selected: ResMut<SelectedEquipment>,
    mut equipment_state: ResMut<EquipmentTreeState>,
    mut equipment_actions: ResMut<EquipmentTreeActions>,
    mut contexts: bevy_egui::EguiContexts,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        // Don't process clicks if hovering over UI
        if contexts.ctx_mut().is_pointer_over_area() {
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

        // Check if we clicked on any equipment
        let mut clicked_id: Option<usize> = None;
        let sprite_size = 64.0; // Equipment sprite click radius (increased for easier clicking)

        for (transform, equipment_sprite) in &equipment_query {
            let sprite_pos = transform.translation.truncate();
            let distance = world_position.distance(sprite_pos);

            if distance < sprite_size {
                clicked_id = Some(equipment_sprite.equipment_id);
                break;
            }
        }

        // Update selection in both resources
        selected.selected_id = clicked_id;

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

/// System to move selected equipment with arrow keys
pub fn move_selected_equipment(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    selected: Res<SelectedEquipment>,
    mut sprite_query: Query<(&mut Transform, &EquipmentSprite)>,
) {
    let Some(selected_id) = selected.selected_id else {
        return;
    };

    let move_speed = 200.0 * time.delta_secs();

    for (mut transform, equipment_sprite) in &mut sprite_query {
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
            break;
        }
    }
}

/// System to manage selection outlines for selected equipment
pub fn update_selection_outlines(
    mut commands: Commands,
    selected: Res<SelectedEquipment>,
    equipment_query: Query<(&Transform, &EquipmentSprite), Without<SelectionOutline>>,
    mut outline_query: Query<
        (Entity, &mut Transform, &SelectionOutline),
        Without<EquipmentSprite>,
    >,
    mut images: ResMut<Assets<Image>>,
) {
    // Get the currently selected equipment ID
    let selected_id = selected.selected_id;

    // Find all existing outlines and check if they should exist
    let mut outlines_to_remove = Vec::new();
    for (entity, _transform, outline) in outline_query.iter() {
        if Some(outline.equipment_id) != selected_id {
            outlines_to_remove.push(entity);
        }
    }

    // Remove outlines that shouldn't exist
    for entity in outlines_to_remove {
        commands.entity(entity).despawn();
    }

    // If we have a selection, make sure it has an outline
    if let Some(id) = selected_id {
        // Check if an outline already exists for this equipment
        let outline_exists = outline_query
            .iter()
            .any(|(_, _, outline)| outline.equipment_id == id);

        if !outline_exists {
            // Find the equipment sprite to get its position
            for (transform, equipment_sprite) in equipment_query.iter() {
                if equipment_sprite.equipment_id == id {
                    // Create a green outline sprite
                    let outline_size = 40;
                    let inner_size = 34; // Inner transparent area
                    let border_thickness = (outline_size - inner_size) / 2;

                    // Create pixel data for the outline
                    let mut pixel_data = Vec::new();
                    for y in 0..outline_size {
                        for x in 0..outline_size {
                            // Check if this pixel is in the border area
                            if x < border_thickness
                                || x >= outline_size - border_thickness
                                || y < border_thickness
                                || y >= outline_size - border_thickness
                            {
                                // Green border
                                pixel_data.extend_from_slice(&[0, 255, 0, 255]);
                            } else {
                                // Transparent center
                                pixel_data.extend_from_slice(&[0, 0, 0, 0]);
                            }
                        }
                    }

                    let outline_image = Image::new(
                        Extent3d {
                            width: outline_size as u32,
                            height: outline_size as u32,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        pixel_data,
                        TextureFormat::Rgba8UnormSrgb,
                        Default::default(),
                    );

                    let outline_handle = images.add(outline_image);

                    // Spawn the outline sprite behind the equipment sprite
                    commands.spawn((
                        Sprite::from_image(outline_handle),
                        Transform::from_translation(
                            transform.translation - Vec3::new(0.0, 0.0, 0.5),
                        ),
                        SelectionOutline { equipment_id: id },
                    ));

                    break;
                }
            }
        }
    }

    // Update outline positions to follow their equipment sprites
    for (equipment_transform, equipment_sprite) in equipment_query.iter() {
        for (_, mut outline_transform, outline) in outline_query.iter_mut() {
            if outline.equipment_id == equipment_sprite.equipment_id {
                outline_transform.translation =
                    equipment_transform.translation - Vec3::new(0.0, 0.0, 0.5);
            }
        }
    }
}

/// System for equipment to mine nearby cells
pub fn equipment_mining_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    equipment_state: Res<EquipmentTreeState>,
    sprite_query: Query<(&Transform, &EquipmentSprite)>,
    mut mineral_map: ResMut<MineralMap>,
) {
    // Press M to mine with active equipment
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }

    println!("M key pressed - checking for mining equipment...");

    // Find all active mining equipment
    for (transform, equipment_sprite) in sprite_query.iter() {
        if let Some(node) = equipment_state.find_node(equipment_sprite.equipment_id) {
            // Check if this is mining equipment
            let can_mine = matches!(
                node.equipment_type(),
                Some(EquipmentType::SurfaceMining) | Some(EquipmentType::DeepMining)
            );

            if !can_mine {
                continue;
            }

            // Get equipment position in world space
            let world_pos = transform.translation.truncate();

            println!(
                "Mining with equipment {} at world pos: {:?}",
                node.name, world_pos
            );

            // Convert to map coordinates (accounting for 2x scale of map sprite)
            // Map is centered at (0, 0) in world space
            // Flip Y because image coordinates go down but world coordinates go up
            let map_x = ((world_pos.x / 2.0) + (MAP_WIDTH as f32 / 2.0)) as i32;
            let map_y = ((MAP_HEIGHT as f32 / 2.0) - (world_pos.y / 2.0)) as i32;

            println!("Map coordinates: x={}, y={}", map_x, map_y);

            // Mining radius (clear a 5x5 area)
            let mining_radius = 10;

            for dy in -mining_radius..=mining_radius {
                for dx in -mining_radius..=mining_radius {
                    let x = map_x + dx;
                    let y = map_y + dy;

                    if x >= 0 && x < MAP_WIDTH as i32 && y >= 0 && y < MAP_HEIGHT as i32 {
                        let idx = y as usize * MAP_WIDTH + x as usize;

                        // Gradually reduce height by mining
                        mineral_map.heightmap[idx] = (mineral_map.heightmap[idx] - 5.0).max(0.0);

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
        }
    }
}