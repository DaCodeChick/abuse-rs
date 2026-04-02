use std::path::PathBuf;

use abuse_runtime::AbuseRuntimePlugins;
use abuse_runtime::data::level::{LevelData, VAR_X, VAR_Y};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::MessageReader;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const FG_TILE_SIZE: f32 = 32.0;
const CAMERA_PAN_SPEED: f32 = 900.0;
const CAMERA_ZOOM_STEP: f32 = 0.1;
const CAMERA_MIN_SCALE: f32 = 0.2;
const CAMERA_MAX_SCALE: f32 = 12.0;

#[derive(Component)]
struct ViewerCamera;

#[derive(Component)]
struct ViewerHud;

#[derive(Resource, Debug, Clone, Copy)]
struct HudState {
    visible: bool,
}

#[derive(Resource, Debug, Clone)]
struct ViewerConfig {
    level_path: Option<PathBuf>,
}

#[derive(Resource, Debug, Clone, Copy)]
struct LevelViewBounds {
    width: f32,
    height: f32,
}

fn main() {
    let level_path = std::env::args().nth(1).map(PathBuf::from);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ViewerConfig { level_path })
        .insert_resource(HudState { visible: true })
        .add_plugins(AbuseRuntimePlugins)
        .add_systems(Startup, (setup_camera, load_level_view).chain())
        .add_systems(Update, (camera_controls, toggle_hud_visibility, update_hud))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, ViewerCamera));
}

fn spawn_hud(commands: &mut Commands, level: &LevelData) {
    commands.spawn((
        Text::new(format!(
            "level: {}\nzoom: 100%\nobjects: {}\nlights: {}\ncontrols: WASD/arrows pan, mouse wheel zoom, Q/E zoom",
            level.name,
            level.objects.len(),
            level.lights.len()
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(10),
            ..default()
        },
        Visibility::Visible,
        ViewerHud,
    ));
}

fn load_level_view(
    mut commands: Commands,
    config: Res<ViewerConfig>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<&mut Transform, With<ViewerCamera>>,
) {
    let Some(level_path) = &config.level_path else {
        info!("No level path provided. Usage: cargo run -p abuse-game -- /path/to/level00.spe");
        return;
    };

    let level = match LevelData::open(level_path) {
        Ok(level) => level,
        Err(err) => {
            error!("Failed to load level {}: {err}", level_path.display());
            return;
        }
    };

    let fg_world_w = level.fg_width as f32 * FG_TILE_SIZE;
    let fg_world_h = level.fg_height as f32 * FG_TILE_SIZE;

    commands.insert_resource(LevelViewBounds {
        width: fg_world_w,
        height: fg_world_h,
    });

    if let (Ok(window), Ok(mut camera_transform)) =
        (window_query.single(), camera_query.single_mut())
    {
        fit_camera_to_level(window, fg_world_w, fg_world_h, &mut camera_transform);
    }

    for row in 0..level.fg_height as usize {
        for col in 0..level.fg_width as usize {
            let idx = row * level.fg_width as usize + col;
            let tile = level.fg_tiles[idx];
            if tile == 0 {
                continue;
            }

            let x = col as f32 * FG_TILE_SIZE - fg_world_w * 0.5 + FG_TILE_SIZE * 0.5;
            let y = fg_world_h * 0.5 - row as f32 * FG_TILE_SIZE - FG_TILE_SIZE * 0.5;

            commands.spawn((
                Sprite::from_color(tile_color(tile, true), Vec2::splat(FG_TILE_SIZE)),
                Transform::from_xyz(x, y, 1.0),
            ));
        }
    }

    let bg_tile_w = if level.bg_width > 0 {
        fg_world_w / level.bg_width as f32
    } else {
        FG_TILE_SIZE
    };
    let bg_tile_h = if level.bg_height > 0 {
        fg_world_h / level.bg_height as f32
    } else {
        FG_TILE_SIZE
    };

    for row in 0..level.bg_height as usize {
        for col in 0..level.bg_width as usize {
            let idx = row * level.bg_width as usize + col;
            let tile = level.bg_tiles[idx];
            if tile == 0 {
                continue;
            }

            let x = col as f32 * bg_tile_w - fg_world_w * 0.5 + bg_tile_w * 0.5;
            let y = fg_world_h * 0.5 - row as f32 * bg_tile_h - bg_tile_h * 0.5;

            commands.spawn((
                Sprite::from_color(tile_color(tile, false), Vec2::new(bg_tile_w, bg_tile_h)),
                Transform::from_xyz(x, y, 0.0),
            ));
        }
    }

    for object in &level.objects {
        let x = object.var(VAR_X).unwrap_or(0) as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - object.var(VAR_Y).unwrap_or(0) as f32;
        commands.spawn((
            Sprite::from_color(Color::srgb(0.95, 0.2, 0.2), Vec2::splat(10.0)),
            Transform::from_xyz(x, y, 3.0),
        ));
    }

    for light in &level.lights {
        let x = light.x as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - light.y as f32;
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.95, 0.25, 0.8), Vec2::splat(14.0)),
            Transform::from_xyz(x, y, 4.0),
        ));
    }

    info!(
        "Loaded {} (fg {}x{}, bg {}x{}, objects {}, lights {})",
        level.name,
        level.fg_width,
        level.fg_height,
        level.bg_width,
        level.bg_height,
        level.objects.len(),
        level.lights.len()
    );

    spawn_hud(&mut commands, &level);
}

fn fit_camera_to_level(
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

fn camera_controls(
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

fn update_hud(
    hud_state: Res<HudState>,
    camera_query: Query<&Transform, With<ViewerCamera>>,
    mut hud_query: Query<&mut Text, With<ViewerHud>>,
) {
    if !hud_state.visible {
        return;
    }

    let Ok(camera) = camera_query.single() else {
        return;
    };
    let Ok(mut text) = hud_query.single_mut() else {
        return;
    };

    let scale = camera.scale.x.max(0.0001);
    let zoom_percent = (100.0 / scale).round();

    let mut lines = text.0.lines();
    let level_line = lines.next().unwrap_or("level: unknown");
    let _old_zoom = lines.next();
    let objects_line = lines.next().unwrap_or("objects: ?");
    let lights_line = lines.next().unwrap_or("lights: ?");
    let controls_line = lines
        .next()
        .unwrap_or("controls: WASD/arrows pan, mouse wheel zoom, Q/E zoom");

    text.0 = format!(
        "{}\nzoom: {}%\n{}\n{}\n{}",
        level_line, zoom_percent, objects_line, lights_line, controls_line
    );
}

fn toggle_hud_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut hud_state: ResMut<HudState>,
    mut hud_query: Query<&mut Visibility, With<ViewerHud>>,
) {
    if !keyboard.just_pressed(KeyCode::F1) {
        return;
    }

    hud_state.visible = !hud_state.visible;
    if let Ok(mut visibility) = hud_query.single_mut() {
        *visibility = if hud_state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn tile_color(tile: u16, foreground: bool) -> Color {
    let seed = tile as u32;
    let r = ((seed.wrapping_mul(97) % 200) as f32 + 30.0) / 255.0;
    let g = ((seed.wrapping_mul(57) % 200) as f32 + 30.0) / 255.0;
    let b = ((seed.wrapping_mul(31) % 200) as f32 + 30.0) / 255.0;
    if foreground {
        Color::srgb(r, g, b)
    } else {
        Color::srgba(r * 0.7, g * 0.7, b * 0.9, 0.4)
    }
}
