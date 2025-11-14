use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use noise::{NoiseFn, Perlin, Fbm};
use rand::{thread_rng, Rng};

const MAP_WIDTH: usize = 512;
const MAP_HEIGHT: usize = 512;

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
        .init_resource::<MineralMap>()
        .init_resource::<EquipmentTreeState>()
        .init_resource::<SelectedEquipment>()
        .add_systems(Startup, (setup, load_equipment_sprites))
        .add_systems(Update, (
            ui_system,
            camera_control_system,
            spawn_equipment_sprites,
            click_select_equipment,
            move_selected_equipment,
            update_equipment_positions,
        ))
        .run();
}

// Mineral types with distinct colors
#[derive(Debug, Clone, Copy, PartialEq)]
enum MineralType {
    Empty,      // Black/dark gray
    Iron,       // Rusty orange
    Copper,     // Copper color
    Gold,       // Gold/yellow
    Silver,     // Light gray/silver
    Uranium,    // Green
    Diamond,    // Cyan/blue
    Coal,       // Dark gray
}

impl MineralType {
    fn color(&self) -> Color {
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

    fn from_noise_value(value: f64, depth: f64) -> Self {
        // Depth affects mineral distribution (deeper = rarer minerals)
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

// Data for each cell/pixel in the map
#[derive(Debug, Clone)]
struct MineralCell {
    mineral_type: MineralType,
    density: f32,      // 0.0 to 1.0, how much mineral is present
    sampled: bool,     // Has this cell been sampled?
    mined: bool,       // Has this cell been mined?
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

// The main mineral map resource
#[derive(Resource)]
struct MineralMap {
    width: usize,
    height: usize,
    data: Vec<MineralCell>,
}

impl Default for MineralMap {
    fn default() -> Self {
        Self::generate()
    }
}

impl MineralMap {
    fn generate() -> Self {
        let mut rng = thread_rng();
        let seed: u32 = rng.gen();

        // Create noise generators
        let perlin = Perlin::new(seed);
        let fbm = Fbm::<Perlin>::new(seed);

        let mut data = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);

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
            }
        }

        Self {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            data,
        }
    }

    fn get(&self, x: usize, y: usize) -> Option<&MineralCell> {
        if x < self.width && y < self.height {
            Some(&self.data[y * self.width + x])
        } else {
            None
        }
    }

    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut MineralCell> {
        if x < self.width && y < self.height {
            Some(&mut self.data[y * self.width + x])
        } else {
            None
        }
    }
}

// Component to mark the mineral map sprite
#[derive(Component)]
struct MineralMapRenderer;

// Mining equipment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EquipmentType {
    Sampler,
    SurfaceMining,
    DeepMining,
    Refining,
    Transport,
}

impl EquipmentType {
    fn name(&self) -> &str {
        match self {
            EquipmentType::Sampler => "Sampler",
            EquipmentType::SurfaceMining => "Surface Mining",
            EquipmentType::DeepMining => "Deep Mining",
            EquipmentType::Refining => "Refining",
            EquipmentType::Transport => "Transport",
        }
    }

    fn description(&self) -> &str {
        match self {
            EquipmentType::Sampler => "Analyzes mineral composition without extraction",
            EquipmentType::SurfaceMining => "Extracts minerals from the upper layers",
            EquipmentType::DeepMining => "Extracts minerals from deep deposits",
            EquipmentType::Refining => "Processes raw minerals into refined materials",
            EquipmentType::Transport => "Moves resources between locations",
        }
    }

    fn sprite_path(&self) -> &str {
        match self {
            EquipmentType::Sampler => "sprites/sampler.png",
            EquipmentType::SurfaceMining => "sprites/surface_mining.png",
            EquipmentType::DeepMining => "sprites/deep_mining.png",
            EquipmentType::Refining => "sprites/refining.png",
            EquipmentType::Transport => "sprites/transport.png",
        }
    }
}

// Equipment instance
#[derive(Debug, Clone)]
struct Equipment {
    id: usize,
    equipment_type: EquipmentType,
    position: Option<Vec2>,
    active: bool,
}

// Tree node for equipment hierarchy
#[derive(Debug, Clone)]
struct EquipmentNode {
    equipment: Equipment,
    children: Vec<EquipmentNode>,
}

// Resource to manage equipment tree state
#[derive(Resource)]
struct EquipmentTreeState {
    equipment_categories: Vec<(EquipmentType, Vec<Equipment>)>,
    next_id: usize,
    expanded_categories: Vec<EquipmentType>,
}

impl Default for EquipmentTreeState {
    fn default() -> Self {
        let categories = vec![
            (EquipmentType::Sampler, vec![]),
            (EquipmentType::SurfaceMining, vec![]),
            (EquipmentType::DeepMining, vec![]),
            (EquipmentType::Refining, vec![]),
            (EquipmentType::Transport, vec![]),
        ];

        Self {
            equipment_categories: categories,
            next_id: 0,
            expanded_categories: vec![],
        }
    }
}

impl EquipmentTreeState {
    fn add_equipment(&mut self, equipment_type: EquipmentType) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let equipment = Equipment {
            id,
            equipment_type,
            position: None,
            active: false,
        };

        for (cat_type, equipments) in &mut self.equipment_categories {
            if *cat_type == equipment_type {
                equipments.push(equipment);
                break;
            }
        }

        id
    }

    fn is_expanded(&self, equipment_type: EquipmentType) -> bool {
        self.expanded_categories.contains(&equipment_type)
    }

    fn toggle_expanded(&mut self, equipment_type: EquipmentType) {
        if let Some(pos) = self.expanded_categories.iter().position(|&t| t == equipment_type) {
            self.expanded_categories.remove(pos);
        } else {
            self.expanded_categories.push(equipment_type);
        }
    }
}

// Resource to store equipment sprites
#[derive(Resource, Default)]
struct EquipmentSprites {
    sprites: std::collections::HashMap<EquipmentType, Handle<Image>>,
}

// Component to mark equipment sprite entities
#[derive(Component)]
struct EquipmentSprite {
    equipment_id: usize,
}

// Resource to track selected equipment
#[derive(Resource, Default)]
struct SelectedEquipment {
    selected_id: Option<usize>,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mineral_map: Res<MineralMap>,
) {
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

// Camera controls: WASD to pan, Q/E to zoom
fn camera_control_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let Ok(mut camera_transform) = query.single_mut() else {
        return;
    };

    let pan_speed = 300.0 * time.delta_secs();
    let zoom_speed = 2.0 * time.delta_secs();

    // Pan with WASD
    if keyboard.pressed(KeyCode::KeyW) {
        camera_transform.translation.y += pan_speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        camera_transform.translation.y -= pan_speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        camera_transform.translation.x -= pan_speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        camera_transform.translation.x += pan_speed;
    }

    // Zoom with Q/E
    if keyboard.pressed(KeyCode::KeyQ) {
        camera_transform.scale *= 1.0 + zoom_speed;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        camera_transform.scale *= 1.0 - zoom_speed;
        // Prevent zooming in too far
        camera_transform.scale.x = camera_transform.scale.x.max(0.1);
        camera_transform.scale.y = camera_transform.scale.y.max(0.1);
    }
}

// Load equipment sprites
fn load_equipment_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut sprites = std::collections::HashMap::new();

    sprites.insert(
        EquipmentType::Sampler,
        asset_server.load(EquipmentType::Sampler.sprite_path()),
    );
    sprites.insert(
        EquipmentType::SurfaceMining,
        asset_server.load(EquipmentType::SurfaceMining.sprite_path()),
    );
    sprites.insert(
        EquipmentType::DeepMining,
        asset_server.load(EquipmentType::DeepMining.sprite_path()),
    );
    sprites.insert(
        EquipmentType::Refining,
        asset_server.load(EquipmentType::Refining.sprite_path()),
    );
    sprites.insert(
        EquipmentType::Transport,
        asset_server.load(EquipmentType::Transport.sprite_path()),
    );

    commands.insert_resource(EquipmentSprites { sprites });
}

// System to spawn sprite entities for equipment that doesn't have one yet
fn spawn_equipment_sprites(
    mut commands: Commands,
    equipment_state: Res<EquipmentTreeState>,
    equipment_sprites: Res<EquipmentSprites>,
    existing_sprites: Query<&EquipmentSprite>,
) {
    // Get all existing equipment IDs that already have sprites
    let existing_ids: std::collections::HashSet<usize> = existing_sprites
        .iter()
        .map(|sprite| sprite.equipment_id)
        .collect();

    // Spawn sprites for equipment that doesn't have one
    for (equipment_type, equipments) in &equipment_state.equipment_categories {
        for equipment in equipments {
            if !existing_ids.contains(&equipment.id) {
                // Equipment needs a sprite
                if let Some(sprite_handle) = equipment_sprites.sprites.get(equipment_type) {
                    let position = equipment.position.unwrap_or_else(|| {
                        // Random position on map if not set
                        let mut rng = thread_rng();
                        Vec2::new(
                            rng.gen_range(-400.0..400.0),
                            rng.gen_range(-300.0..300.0),
                        )
                    });

                    commands.spawn((
                        Sprite::from_image(sprite_handle.clone()),
                        Transform::from_translation(position.extend(1.0)),
                        EquipmentSprite {
                            equipment_id: equipment.id,
                        },
                    ));
                }
            }
        }
    }
}

// System to update equipment positions in the state when sprites move
fn update_equipment_positions(
    mut equipment_state: ResMut<EquipmentTreeState>,
    sprite_query: Query<(&Transform, &EquipmentSprite), Changed<Transform>>,
) {
    for (transform, equipment_sprite) in &sprite_query {
        // Find the equipment and update its position
        for (_equipment_type, equipments) in &mut equipment_state.equipment_categories {
            for equipment in equipments {
                if equipment.id == equipment_sprite.equipment_id {
                    equipment.position = Some(transform.translation.truncate());
                    break;
                }
            }
        }
    }
}

// System to select equipment by clicking on them
fn click_select_equipment(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    equipment_query: Query<(&Transform, &EquipmentSprite)>,
    mut selected: ResMut<SelectedEquipment>,
    mut equipment_state: ResMut<EquipmentTreeState>,
    mut contexts: bevy_egui::EguiContexts,
) {
    // Don't process clicks if hovering over UI
    if contexts.ctx_mut().is_pointer_over_area() {
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
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
        let Ok(world_position) = camera
            .viewport_to_world_2d(camera_transform, cursor_position)
        else {
            return;
        };

        // Check if we clicked on any equipment
        let mut clicked_id: Option<usize> = None;
        let sprite_size = 32.0; // Equipment sprite size

        for (transform, equipment_sprite) in &equipment_query {
            let sprite_pos = transform.translation.truncate();
            let distance = world_position.distance(sprite_pos);

            if distance < sprite_size {
                clicked_id = Some(equipment_sprite.equipment_id);
                break;
            }
        }

        // Update selection
        selected.selected_id = clicked_id;

        // Activate/deactivate equipment
        if let Some(id) = clicked_id {
            for (_equipment_type, equipments) in &mut equipment_state.equipment_categories {
                for equipment in equipments {
                    equipment.active = equipment.id == id;
                }
            }
        }
    }
}

// System to move selected equipment with arrow keys
fn move_selected_equipment(
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

fn ui_system(
    mut contexts: EguiContexts,
    mut equipment_state: ResMut<EquipmentTreeState>,
    selected: Res<SelectedEquipment>,
) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Regolith Voxel - Mining Operations");
            ui.separator();
            ui.label("WASD: Pan | Q/E: Zoom | Click: Select | Arrows: Move");

            if let Some(selected_id) = selected.selected_id {
                ui.separator();
                ui.label(format!("Selected: Unit #{}", selected_id));
            }
        });
    });

    // Bottom panel
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Status: Ready");
        });
    });

    // Left panel - Legend
    egui::SidePanel::left("left_panel").show(ctx, |ui| {
        ui.heading("Minerals");
        ui.separator();

        ui.label("Legend:");
        ui.colored_label(egui::Color32::from_rgb(204, 102, 51), "■ Iron");
        ui.colored_label(egui::Color32::from_rgb(184, 115, 51), "■ Copper");
        ui.colored_label(egui::Color32::from_rgb(255, 215, 0), "■ Gold");
        ui.colored_label(egui::Color32::from_rgb(192, 192, 192), "■ Silver");
        ui.colored_label(egui::Color32::from_rgb(51, 204, 51), "■ Uranium");
        ui.colored_label(egui::Color32::from_rgb(102, 204, 255), "■ Diamond");
        ui.colored_label(egui::Color32::from_rgb(51, 51, 51), "■ Coal");
    });

    // Right panel - Equipment Tree
    egui::SidePanel::right("right_panel").min_width(250.0).show(ctx, |ui| {
        ui.heading("Mining Equipment");
        ui.separator();

        // Collect actions to avoid borrowing issues
        let mut toggle_type: Option<EquipmentType> = None;
        let mut add_type: Option<EquipmentType> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let categories = equipment_state.equipment_categories.clone();

            for (equipment_type, equipments) in &categories {
                let is_expanded = equipment_state.is_expanded(*equipment_type);

                // Category header with expand/collapse button
                ui.horizontal(|ui| {
                    let icon = if is_expanded { "▼" } else { "▶" };
                    if ui.button(icon).clicked() {
                        toggle_type = Some(*equipment_type);
                    }
                    ui.label(equipment_type.name());
                });

                // Show equipment instances when expanded
                if is_expanded {
                    ui.indent(equipment_type.name(), |ui| {
                        // Show description
                        ui.label(egui::RichText::new(equipment_type.description())
                            .size(10.0)
                            .italics()
                            .color(egui::Color32::GRAY));

                        ui.separator();

                        // List all equipment instances
                        if equipments.is_empty() {
                            ui.label(egui::RichText::new("No equipment deployed")
                                .size(10.0)
                                .color(egui::Color32::DARK_GRAY));
                        } else {
                            for equipment in equipments {
                                ui.horizontal(|ui| {
                                    let status = if equipment.active { "🟢" } else { "⚪" };
                                    ui.label(format!("{} Unit #{}", status, equipment.id));
                                    if let Some(pos) = equipment.position {
                                        ui.label(format!("({:.0}, {:.0})", pos.x, pos.y));
                                    }
                                });
                            }
                        }

                        ui.separator();

                        // Add new equipment button
                        if ui.button(format!("+ Add {}", equipment_type.name())).clicked() {
                            add_type = Some(*equipment_type);
                        }
                    });
                }

                ui.add_space(5.0);
            }
        });

        // Apply actions after rendering
        if let Some(eq_type) = toggle_type {
            equipment_state.toggle_expanded(eq_type);
        }
        if let Some(eq_type) = add_type {
            equipment_state.add_equipment(eq_type);
        }
    });

    // Central panel is behind the game view
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |_ui| {
            // Game renders here
        });
}
