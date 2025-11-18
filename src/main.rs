use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_arbor::{ActionIcon, DropPosition, IconType, Outliner, OutlinerActions, OutlinerNode, tree_ops::TreeOperations};
use noise::{NoiseFn, Perlin, Fbm};
use rand::{thread_rng, Rng};
use std::collections::HashSet;

const MAP_WIDTH: usize = 512;
const MAP_HEIGHT: usize = 512;
const CA_TICK_RATE: f32 = 1.0 / 30.0; // 30 updates per second

// Physics types for cellular automata
#[derive(Debug, Clone, Copy, PartialEq)]
enum PhysicsType {
    Empty,      // Void/air - materials can move into this
    Solid,      // Structural - doesn't move
    Granular,   // Falls like sand when unsupported
    Flowing,    // Flows like liquid
}

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
        .init_resource::<EquipmentTreeActions>()
        .init_resource::<SelectedEquipment>()
        .init_resource::<CellularAutomataTimer>()
        .add_systems(Startup, (setup, load_equipment_sprites))
        .add_systems(Update, (
            ui_system,
            camera_control_system,
            spawn_equipment_sprites,
            click_select_equipment,
            move_selected_equipment,
            update_equipment_positions,
            update_selection_outlines,
            equipment_mining_system,
            cellular_automata_system,
            update_mineral_map_texture,
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

    fn physics_type(&self) -> PhysicsType {
        match self {
            MineralType::Empty => PhysicsType::Empty,
            MineralType::Diamond | MineralType::Uranium => PhysicsType::Solid,
            MineralType::Coal | MineralType::Iron | MineralType::Copper => PhysicsType::Granular,
            MineralType::Gold | MineralType::Silver => PhysicsType::Flowing,
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
    heightmap: Vec<f32>, // Invisible heightmap for flow simulation
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
        let height_noise = Perlin::new(seed.wrapping_add(1000));

        let mut data = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);
        let mut heightmap = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);

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
                // Empty cells have height 0, filled cells have height based on material density
                let height = if mineral_type == MineralType::Empty {
                    0.0
                } else {
                    // Base height on density plus some variation
                    let height_scale = 0.05;
                    let height_variation = height_noise.get([x as f64 * height_scale, y as f64 * height_scale]);
                    let base_height = density * 100.0; // Material creates height
                    base_height + (height_variation as f32 * 20.0)
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

// Tree node for equipment hierarchy
#[derive(Debug, Clone)]
struct EquipmentTreeNode {
    id: usize,
    name: String,
    node_type: NodeType,
    position: Option<Vec2>,
    active: bool,
    children: Vec<EquipmentTreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeType {
    Container,
    Equipment(EquipmentType),
}

impl EquipmentTreeNode {
    fn container(id: usize, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            node_type: NodeType::Container,
            position: None,
            active: false,
            children: Vec::new(),
        }
    }

    fn equipment(id: usize, name: impl Into<String>, equipment_type: EquipmentType) -> Self {
        Self {
            id,
            name: name.into(),
            node_type: NodeType::Equipment(equipment_type),
            position: None,
            active: false,
            children: Vec::new(),
        }
    }

    fn is_container(&self) -> bool {
        matches!(self.node_type, NodeType::Container)
    }

    fn equipment_type(&self) -> Option<EquipmentType> {
        match self.node_type {
            NodeType::Equipment(eq_type) => Some(eq_type),
            _ => None,
        }
    }

    /// Recursively find and rename a node by ID
    fn rename_node(&mut self, id: usize, new_name: String) -> bool {
        if self.id == id {
            self.name = new_name;
            return true;
        }

        for child in &mut self.children {
            if child.rename_node(id, new_name.clone()) {
                return true;
            }
        }

        false
    }

    /// Recursively find a node by ID and return a reference
    fn find_node(&self, id: usize) -> Option<&EquipmentTreeNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &self.children {
            if let Some(node) = child.find_node(id) {
                return Some(node);
            }
        }

        None
    }

    /// Recursively find a mutable node by ID
    fn find_node_mut(&mut self, id: usize) -> Option<&mut EquipmentTreeNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(node) = child.find_node_mut(id) {
                return Some(node);
            }
        }

        None
    }
}

// Implement OutlinerNode for the tree
impl OutlinerNode for EquipmentTreeNode {
    type Id = usize;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_collection(&self) -> bool {
        self.is_container()
    }

    fn children(&self) -> &[Self] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }

    fn icon(&self) -> Option<IconType> {
        if self.is_container() {
            Some(IconType::Collection)
        } else {
            Some(IconType::Entity)
        }
    }

    fn action_icons(&self) -> Vec<ActionIcon> {
        vec![ActionIcon::Visibility, ActionIcon::Selection]
    }
}

// Implement TreeOperations for drag-drop functionality
impl TreeOperations for EquipmentTreeNode {}

// Resource to manage equipment tree state
#[derive(Resource)]
struct EquipmentTreeState {
    nodes: Vec<EquipmentTreeNode>,
    next_id: usize,
}

impl Default for EquipmentTreeState {
    fn default() -> Self {
        let mut next_id = 0;

        // Create initial container nodes for each equipment type with some sample equipment
        let nodes = vec![
            {
                let mut container = EquipmentTreeNode::container(next_id, "Samplers");
                next_id += 1;

                // Add a sample sampler
                container.children.push(EquipmentTreeNode::equipment(
                    next_id,
                    "Sampler Unit 1",
                    EquipmentType::Sampler
                ));
                next_id += 1;

                container
            },
            {
                let mut container = EquipmentTreeNode::container(next_id, "Surface Mining");
                next_id += 1;

                // Add a sample surface miner
                container.children.push(EquipmentTreeNode::equipment(
                    next_id,
                    "Surface Miner 1",
                    EquipmentType::SurfaceMining
                ));
                next_id += 1;

                container
            },
            {
                let container = EquipmentTreeNode::container(next_id, "Deep Mining");
                next_id += 1;
                container
            },
            {
                let container = EquipmentTreeNode::container(next_id, "Refining");
                next_id += 1;
                container
            },
            {
                let container = EquipmentTreeNode::container(next_id, "Transport");
                next_id += 1;
                container
            },
        ];

        Self {
            nodes,
            next_id,
        }
    }
}

impl EquipmentTreeState {
    fn add_container(&mut self, name: String) {
        let container = EquipmentTreeNode::container(self.next_id, name);
        self.next_id += 1;
        self.nodes.push(container);
    }

    fn add_equipment(&mut self, name: String, equipment_type: EquipmentType) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let equipment = EquipmentTreeNode::equipment(id, name, equipment_type);
        self.nodes.push(equipment);

        id
    }

    fn find_node(&self, id: usize) -> Option<&EquipmentTreeNode> {
        for node in &self.nodes {
            if let Some(found) = node.find_node(id) {
                return Some(found);
            }
        }
        None
    }

    fn find_node_mut(&mut self, id: usize) -> Option<&mut EquipmentTreeNode> {
        for node in &mut self.nodes {
            if let Some(found) = node.find_node_mut(id) {
                return Some(found);
            }
        }
        None
    }
}

// Actions handler for the outliner
#[derive(Resource, Default)]
struct EquipmentTreeActions {
    selected: HashSet<usize>,
    visible: HashSet<usize>,
}

impl EquipmentTreeActions {
    fn new() -> Self {
        Self {
            selected: HashSet::new(),
            visible: HashSet::new(),
        }
    }
}

impl OutlinerActions<EquipmentTreeNode> for EquipmentTreeActions {
    fn on_rename(&mut self, _id: &usize, _new_name: String) {
        // Renaming is handled in the ui_system
    }

    fn on_move(&mut self, _id: &usize, _target: &usize, _position: DropPosition) {
        // Moving is handled in the ui_system
    }

    fn on_select(&mut self, id: &usize, selected: bool) {
        if selected {
            self.selected.insert(*id);
        } else {
            self.selected.remove(id);
        }
    }

    fn is_selected(&self, id: &usize) -> bool {
        self.selected.contains(id)
    }

    fn is_visible(&self, id: &usize) -> bool {
        !self.visible.contains(id) // Using "visible" set as "hidden" set - inverted logic
    }

    fn is_locked(&self, _id: &usize) -> bool {
        false
    }

    fn on_visibility_toggle(&mut self, id: &usize) {
        if self.visible.contains(id) {
            self.visible.remove(id);
        } else {
            self.visible.insert(*id);
        }
    }

    fn on_lock_toggle(&mut self, _id: &usize) {}

    fn on_selection_toggle(&mut self, id: &usize) {
        let is_selected = self.is_selected(id);
        self.on_select(id, !is_selected);
    }

    fn on_custom_action(&mut self, _id: &usize, _icon: &str) {}
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

// Component to mark selection outline sprites
#[derive(Component)]
struct SelectionOutline {
    equipment_id: usize,
}

// Resource to track selected equipment
#[derive(Resource, Default)]
struct SelectedEquipment {
    selected_id: Option<usize>,
}

// Timer resource for cellular automata updates
#[derive(Resource)]
struct CellularAutomataTimer {
    timer: Timer,
}

impl Default for CellularAutomataTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(CA_TICK_RATE, TimerMode::Repeating),
        }
    }
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

// Load equipment sprites - generate them programmatically
fn load_equipment_sprites(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
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

    // Helper function to recursively spawn sprites
    fn spawn_for_node(
        node: &EquipmentTreeNode,
        existing_ids: &std::collections::HashSet<usize>,
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

                    commands.spawn((
                        Sprite::from_image(sprite_handle.clone()),
                        Transform::from_translation(position.extend(1.0)),
                        EquipmentSprite {
                            equipment_id: node.id,
                        },
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

// System to update equipment positions in the state when sprites move
fn update_equipment_positions(
    mut equipment_state: ResMut<EquipmentTreeState>,
    sprite_query: Query<(&Transform, &EquipmentSprite), Changed<Transform>>,
) {
    for (transform, equipment_sprite) in &sprite_query {
        // Find the equipment node and update its position
        if let Some(node) = equipment_state.find_node_mut(equipment_sprite.equipment_id) {
            node.position = Some(transform.translation.truncate());
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
        let Ok(world_position) = camera
            .viewport_to_world_2d(camera_transform, cursor_position)
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
    mut equipment_actions: ResMut<EquipmentTreeActions>,
    selected: Res<SelectedEquipment>,
) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Regolith Voxel - Mining Operations");
            ui.separator();
            ui.label("WASD: Pan | Q/E: Zoom | Click: Select | Arrows: Move | M: Mine");

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

    // Right panel - Equipment Tree with Outliner
    egui::SidePanel::right("right_panel").min_width(300.0).show(ctx, |ui| {
        ui.heading("Mining Equipment");
        ui.separator();

        ui.label("Drag to reorganize | Double-click to rename");
        ui.add_space(4.0);

        // Action buttons at the top
        ui.horizontal(|ui| {
            if ui.button("+ New Container").clicked() {
                let id = equipment_state.next_id;
                equipment_state.add_container(format!("Container {}", id));
            }

            ui.menu_button("+ New Equipment", |ui| {
                if ui.button("Sampler").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_equipment(
                        format!("Sampler {}", id),
                        EquipmentType::Sampler
                    );
                    ui.close_menu();
                }
                if ui.button("Surface Mining").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_equipment(
                        format!("Surface Miner {}", id),
                        EquipmentType::SurfaceMining
                    );
                    ui.close_menu();
                }
                if ui.button("Deep Mining").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_equipment(
                        format!("Deep Miner {}", id),
                        EquipmentType::DeepMining
                    );
                    ui.close_menu();
                }
                if ui.button("Refining").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_equipment(
                        format!("Refinery {}", id),
                        EquipmentType::Refining
                    );
                    ui.close_menu();
                }
                if ui.button("Transport").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_equipment(
                        format!("Transport {}", id),
                        EquipmentType::Transport
                    );
                    ui.close_menu();
                }
            });
        });

        ui.separator();

        // Show the outliner with the tree
        egui::ScrollArea::vertical().show(ui, |ui| {
            let response = Outliner::new("equipment_outliner")
                .show(ui, &equipment_state.nodes, &mut *equipment_actions);

            // Handle rename events
            if let Some((node_id, new_name)) = response.renamed() {
                for root in &mut equipment_state.nodes {
                    if root.rename_node(*node_id, new_name.to_string()) {
                        break;
                    }
                }
            }

            // Handle drag-drop events
            if let Some(drop_event) = response.drop_event() {
                let target_id = &drop_event.target;
                let position = drop_event.position;

                // Get all nodes being dragged
                let dragging_ids = response.dragging_nodes();

                if !dragging_ids.is_empty() {
                    // Use TreeOperations to handle the move
                    for drag_id in dragging_ids {
                        // Find and remove the dragged node
                        let mut removed_node = None;

                        // Try to remove from root level
                        if let Some(idx) = equipment_state.nodes.iter().position(|n| n.id == *drag_id) {
                            removed_node = Some(equipment_state.nodes.remove(idx));
                        } else {
                            // Search recursively in children
                            for root in &mut equipment_state.nodes {
                                if let Some(node) = EquipmentTreeNode::remove_node(root, *drag_id) {
                                    removed_node = Some(node);
                                    break;
                                }
                            }
                        }

                        // Insert the node at the new position
                        if let Some(node) = removed_node {
                            let mut inserted = false;

                            // Try to insert relative to target
                            for root in &mut equipment_state.nodes {
                                if EquipmentTreeNode::insert_node(root, *target_id, node.clone(), position) {
                                    inserted = true;
                                    break;
                                }
                            }

                            // If not inserted, add back to root level
                            if !inserted {
                                equipment_state.nodes.push(node);
                            }
                        }
                    }
                }
            }
        });
    });

    // No central panel needed - game renders in the background
    // This allows clicks to reach the game without being intercepted by egui
}

// Helper methods for EquipmentTreeNode to support drag-drop
impl EquipmentTreeNode {
    fn remove_node(parent: &mut EquipmentTreeNode, id: usize) -> Option<EquipmentTreeNode> {
        // Check direct children
        if let Some(idx) = parent.children.iter().position(|n| n.id == id) {
            return Some(parent.children.remove(idx));
        }

        // Search recursively
        for child in &mut parent.children {
            if let Some(node) = Self::remove_node(child, id) {
                return Some(node);
            }
        }

        None
    }

    fn insert_node(
        parent: &mut EquipmentTreeNode,
        target_id: usize,
        node: EquipmentTreeNode,
        position: DropPosition,
    ) -> bool {
        // If this is the target
        if parent.id == target_id {
            match position {
                DropPosition::Inside => {
                    if parent.is_container() {
                        parent.children.push(node);
                        return true;
                    }
                }
                _ => {
                    // Can't insert before/after root
                    return false;
                }
            }
        }

        // Check if target is in direct children
        if let Some(idx) = parent.children.iter().position(|n| n.id == target_id) {
            match position {
                DropPosition::Before => {
                    parent.children.insert(idx, node);
                    return true;
                }
                DropPosition::After => {
                    parent.children.insert(idx + 1, node);
                    return true;
                }
                DropPosition::Inside => {
                    if parent.children[idx].is_container() {
                        parent.children[idx].children.push(node);
                        return true;
                    }
                }
            }
        }

        // Search recursively
        for child in &mut parent.children {
            if Self::insert_node(child, target_id, node.clone(), position) {
                return true;
            }
        }

        false
    }
}

// System to manage selection outlines for selected equipment
fn update_selection_outlines(
    mut commands: Commands,
    selected: Res<SelectedEquipment>,
    equipment_query: Query<(&Transform, &EquipmentSprite), Without<SelectionOutline>>,
    mut outline_query: Query<(Entity, &mut Transform, &SelectionOutline), Without<EquipmentSprite>>,
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
                            if x < border_thickness || x >= outline_size - border_thickness ||
                               y < border_thickness || y >= outline_size - border_thickness {
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
                        Transform::from_translation(transform.translation - Vec3::new(0.0, 0.0, 0.5)),
                        SelectionOutline {
                            equipment_id: id,
                        },
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
                outline_transform.translation = equipment_transform.translation - Vec3::new(0.0, 0.0, 0.5);
            }
        }
    }
}

// System for equipment to mine nearby cells
fn equipment_mining_system(
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

            println!("Mining with equipment {} at world pos: {:?}", node.name, world_pos);

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
                        if let Some(cell) = mineral_map.get_mut(x as usize, y as usize) {
                            // Mine the cell (set to empty)
                            cell.mineral_type = MineralType::Empty;
                            cell.mined = true;
                            cell.density = 0.0;

                            // Update heightmap - empty cells have 0 height (creates void)
                            let idx = y as usize * MAP_WIDTH + x as usize;
                            mineral_map.heightmap[idx] = 0.0;
                        }
                    }
                }
            }
        }
    }
}

// Cellular automata system - updates mineral cells based on physics rules
fn cellular_automata_system(
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
                let neighbor_physics = mineral_map.data[neighbor_idx].mineral_type.physics_type();

                // Only move if target is already processed (in next_data) and is empty
                if next_data[neighbor_idx].mineral_type.physics_type() != PhysicsType::Empty {
                    continue;
                }

                // Calculate height difference threshold
                let height_diff = current_height - neighbor_height;

                // GRANULAR PHYSICS - only move to much lower areas
                if physics == PhysicsType::Granular {
                    if height_diff > 20.0 && rng.gen_bool(0.3) {
                        candidates.push((nx, ny, neighbor_height));
                    }
                }
                // FLOWING PHYSICS - move to moderately lower areas
                else if physics == PhysicsType::Flowing {
                    if height_diff > 10.0 && rng.gen_bool(0.5) {
                        candidates.push((nx, ny, neighbor_height));
                    }
                }
            }

            // Pick a random candidate (don't always pick lowest for variety)
            if !candidates.is_empty() && rng.gen_bool(0.3) {
                let chosen = candidates[rng.gen_range(0..candidates.len())];
                let (nx, ny, target_height) = chosen;
                let target_idx = ny * width + nx;

                // Move material to target (carry full height)
                next_data[target_idx] = cell.clone();
                next_data[idx] = MineralCell {
                    mineral_type: MineralType::Empty,
                    density: 0.0,
                    sampled: cell.sampled,
                    mined: true,
                };

                // Material carries its full height to destination
                next_heightmap[target_idx] = current_height;
                next_heightmap[idx] = 0.0; // Source becomes void
            }
        }
    }

    // Update the mineral map with the new state
    mineral_map.data = next_data;
    mineral_map.heightmap = next_heightmap;
}

// System to update the mineral map texture after CA updates
fn update_mineral_map_texture(
    mineral_map: Res<MineralMap>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&Sprite, With<MineralMapRenderer>>,
) {
    // Only update if the mineral map changed
    if !mineral_map.is_changed() {
        return;
    }

    // Find the mineral map sprite
    for sprite in query.iter() {
        if let Some(image) = images.get_mut(&sprite.image) {
            // Update the texture data
            let mut new_data = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT * 4);

            for cell in &mineral_map.data {
                let color = cell.mineral_type.color();
                let brightness = 0.5 + cell.density * 0.5;
                new_data.push((color.to_srgba().red * brightness * 255.0) as u8);
                new_data.push((color.to_srgba().green * brightness * 255.0) as u8);
                new_data.push((color.to_srgba().blue * brightness * 255.0) as u8);
                new_data.push(255);
            }

            image.data = Some(new_data);
        }
    }
}
