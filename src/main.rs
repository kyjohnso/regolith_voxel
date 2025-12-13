//! Regolith Voxel - A lunar mining game
//! 
//! This game simulates mining operations on the Moon using various equipment types
//! and cellular automata for physics simulation.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::EguiPlugin;

mod camera;
mod equipment;
mod scene;
mod ui;

use camera::camera_control_system;
use equipment::{
    click_select_equipment, equipment_mining_system, load_equipment_sprites,
    move_selected_equipment, spawn_equipment_sprites, update_equipment_positions,
    update_selection_sprites, EquipmentTreeActions, EquipmentTreeState, SelectedEquipment,
};
use scene::{
    cellular_automata_system, update_mineral_map_texture, CellularAutomataTimer, MineralMap,
    MineralMapRenderer, ViewMode, MAP_HEIGHT, MAP_WIDTH,
};
use ui::ui_system;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Regolith Voxel - Mining Game".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        // Scene resources
        .init_resource::<MineralMap>()
        .init_resource::<CellularAutomataTimer>()
        .init_resource::<ViewMode>()
        // Equipment resources
        .init_resource::<EquipmentTreeState>()
        .init_resource::<EquipmentTreeActions>()
        .init_resource::<SelectedEquipment>()
        // Systems
        .add_systems(Startup, (setup, load_equipment_sprites))
        .add_systems(
            Update,
            (
                // UI
                ui_system,
                // Camera
                camera_control_system,
                // Equipment
                spawn_equipment_sprites,
                click_select_equipment,
                move_selected_equipment,
                update_equipment_positions,
                update_selection_sprites,
                equipment_mining_system,
                // Scene physics
                cellular_automata_system,
                update_mineral_map_texture,
            ),
        )
        .run();
}

/// Setup the initial game state - camera and mineral map
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mineral_map: Res<MineralMap>) {
    // Setup 2D camera
    commands.spawn(Camera2d);

    // Create the image from mineral data
    let mut image_data = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT * 4);

    for cell in &mineral_map.data {
        let color = cell.mineral_type.color();
        // Adjust brightness by density
        let brightness = 0.5 + cell.density * 0.5;
        image_data.push((color.to_srgba().red * brightness * 255.0) as u8);
        image_data.push((color.to_srgba().green * brightness * 255.0) as u8);
        image_data.push((color.to_srgba().blue * brightness * 255.0) as u8);
        image_data.push(255);
    }

    let image = Image::new(
        Extent3d {
            width: MAP_WIDTH as u32,
            height: MAP_HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image_data,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    );

    let image_handle = images.add(image);

    // Spawn the mineral map sprite
    commands.spawn((
        Sprite::from_image(image_handle),
        Transform::from_scale(Vec3::splat(2.0)), // Scale up for visibility
        MineralMapRenderer,
    ));
}
