//! Camera control system

use bevy::prelude::*;

/// Camera controls: WASD to pan, Q/E to zoom
pub fn camera_control_system(
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