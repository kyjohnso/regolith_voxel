use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
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
        .add_systems(Startup, setup)
        .add_systems(Update, (ui_system, camera_control_system))
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

fn ui_system(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Regolith Voxel - Mining Operations");
            ui.separator();
            ui.label("WASD: Pan | Q/E: Zoom");
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

    // Right panel - Resources
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.heading("Resources");
        ui.separator();
        ui.label("Iron: 0");
        ui.label("Copper: 0");
        ui.label("Gold: 0");
    });

    // Central panel is behind the game view
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |_ui| {
            // Game renders here
        });
}
