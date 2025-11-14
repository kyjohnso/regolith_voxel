use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

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
        .add_systems(Startup, setup)
        .add_systems(Update, ui_system)
        .run();
}

fn setup(mut commands: Commands) {
    // Setup 2D camera
    commands.spawn(Camera2d);
}

fn ui_system(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Top Panel");
        });
    });

    // Bottom panel
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Bottom Panel");
        });
    });

    // Left panel
    egui::SidePanel::left("left_panel").show(ctx, |ui| {
        ui.label("Left Panel");
    });

    // Right panel
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.label("Right Panel");
    });

    // Central panel (game area)
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("Game Area - 2D Mining World");
    });
}
