//! Camera and viewport behavior for the level viewer.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::MessageReader;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Camera pan speed in world units per second.
pub const CAMERA_PAN_SPEED: f32 = 900.0;
/// Mouse wheel zoom sensitivity.
pub const CAMERA_ZOOM_STEP: f32 = 0.1;
/// Minimum zoom scale allowed.
pub const CAMERA_MIN_SCALE: f32 = 0.2;
/// Maximum zoom scale allowed.
pub const CAMERA_MAX_SCALE: f32 = 12.0;

/// Marker for the active 2D viewer camera entity.
#[derive(Component)]
pub struct ViewerCamera;

/// Bounds of the currently loaded level in world space.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LevelViewBounds {
    /// World width.
    pub width: f32,
    /// World height.
    pub height: f32,
}

/// Creates the camera entity used by viewer systems.
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, ViewerCamera));
}

/// Fits the camera to show the entire level bounds with margin.
pub fn fit_camera_to_level(
    window: &Window,
    level_width: f32,
    level_height: f32,
    camera: &mut Transform,
) {
    let required_x = if window.width() > 0.0 {
        level_width / window.width()
    } else {
        1.0
    };
    let required_y = if window.height() > 0.0 {
        level_height / window.height()
    } else {
        1.0
    };
    let fit_scale = (required_x.max(required_y) * 1.1).clamp(CAMERA_MIN_SCALE, CAMERA_MAX_SCALE);

    camera.translation.x = 0.0;
    camera.translation.y = 0.0;
    camera.scale = Vec3::splat(fit_scale);
}

/// Handles pan/zoom controls and clamps camera to level bounds.
pub fn camera_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    bounds: Option<Res<LevelViewBounds>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<&mut Transform, With<ViewerCamera>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec2::ZERO {
        let pan =
            direction.normalize() * CAMERA_PAN_SPEED * camera_transform.scale.x * time.delta_secs();
        camera_transform.translation.x += pan.x;
        camera_transform.translation.y += pan.y;
    }

    let mut zoom_factor = 1.0_f32;
    for event in mouse_wheel.read() {
        zoom_factor *= (1.0_f32 - event.y * CAMERA_ZOOM_STEP).max(0.1);
    }
    if keyboard.pressed(KeyCode::KeyQ) {
        zoom_factor *= 1.02;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        zoom_factor *= 0.98;
    }

    let new_scale =
        (camera_transform.scale.x * zoom_factor).clamp(CAMERA_MIN_SCALE, CAMERA_MAX_SCALE);
    camera_transform.scale = Vec3::splat(new_scale);

    if let Some(bounds) = bounds {
        let half_visible_w = 0.5 * window.width() * camera_transform.scale.x;
        let half_visible_h = 0.5 * window.height() * camera_transform.scale.x;
        let half_level_w = bounds.width * 0.5;
        let half_level_h = bounds.height * 0.5;

        let max_x = (half_level_w - half_visible_w).max(0.0);
        let max_y = (half_level_h - half_visible_h).max(0.0);
        camera_transform.translation.x = camera_transform.translation.x.clamp(-max_x, max_x);
        camera_transform.translation.y = camera_transform.translation.y.clamp(-max_y, max_y);
    }
}
