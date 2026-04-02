use std::path::PathBuf;

use abuse_runtime::AbuseRuntimePlugins;
use abuse_runtime::data::level::{LevelData, VAR_X, VAR_Y};
use abuse_runtime::viewer::assets::{
    derive_data_root, load_legacy_tile_set, load_object_sprite_library, make_radial_glow_texture,
};
use abuse_runtime::viewer::audio::{
    AudioState, adjust_audio_volume, spawn_context_audio, sync_audio_volume, toggle_audio,
};
use abuse_runtime::viewer::constants::FG_TILE_SIZE;
use abuse_runtime::viewer::object_render::{object_render_adjustment, resolve_object_sprite};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::MessageReader;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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
    let asset_root = level_path
        .as_deref()
        .and_then(derive_data_root)
        .unwrap_or_else(|| PathBuf::from("assets"))
        .to_string_lossy()
        .into_owned();

    let plugins = DefaultPlugins.set(bevy::asset::AssetPlugin {
        file_path: asset_root,
        ..default()
    });

    App::new()
        .add_plugins(plugins)
        .insert_resource(ViewerConfig { level_path })
        .insert_resource(HudState { visible: true })
        .insert_resource(AudioState {
            enabled: true,
            volume: 0.45,
        })
        .add_plugins(AbuseRuntimePlugins)
        .add_systems(Startup, (setup_camera, load_level_view).chain())
        .add_systems(
            Update,
            (
                camera_controls,
                toggle_hud_visibility,
                toggle_audio,
                adjust_audio_volume,
                sync_audio_volume,
                update_hud,
            ),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, ViewerCamera));
}

fn spawn_hud(commands: &mut Commands, level: &LevelData, audio: &AudioState) {
    let level_name = std::path::Path::new(&level.name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&level.name);

    commands.spawn((
        Text::new(format!(
            "level: {}\nzoom: 100%\nobjects: {}\nlights: {}\naudio: {} ({}%)\ncontrols: WASD/arrows pan, wheel/Q/E zoom, F1 HUD, M mute, -/+ volume",
            level_name,
            level.objects.len(),
            level.lights.len(),
            if audio.enabled { "on" } else { "off" },
            (audio.volume * 100.0).round() as i32,
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(10),
            padding: UiRect::axes(px(10), px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.7)),
        Visibility::Visible,
        ViewerHud,
    ));
}

fn load_level_view(
    mut commands: Commands,
    config: Res<ViewerConfig>,
    audio_state: Res<AudioState>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
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

    let tile_set = load_legacy_tile_set(level_path, &mut images, FG_TILE_SIZE)
        .inspect_err(|err| warn!("Tile asset loading failed, falling back to debug colors: {err}"))
        .ok();

    let object_sprites = load_object_sprite_library(level_path, &mut images)
        .inspect_err(|err| warn!("Object sprite library failed to load: {err}"))
        .ok();

    spawn_context_audio(&mut commands, &asset_server, &audio_state, &level);

    let fg_tile_size = tile_set
        .as_ref()
        .map(|set| set.fg_tile_size)
        .unwrap_or(Vec2::splat(FG_TILE_SIZE));
    let bg_tile_size = tile_set
        .as_ref()
        .map(|set| set.bg_tile_size)
        .unwrap_or(Vec2::splat(FG_TILE_SIZE));

    let fg_world_w = level.fg_width as f32 * fg_tile_size.x;
    let fg_world_h = level.fg_height as f32 * fg_tile_size.y;

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
            let tile_id = tile & 0x3fff;
            if tile_id == 0 {
                continue;
            }

            let x = col as f32 * fg_tile_size.x - fg_world_w * 0.5 + fg_tile_size.x * 0.5;
            let y = fg_world_h * 0.5 - row as f32 * fg_tile_size.y - fg_tile_size.y * 0.5;

            if let Some(texture) = tile_set.as_ref().and_then(|set| set.fg_tiles.get(&tile_id)) {
                commands.spawn((
                    Sprite::from_image(texture.clone()),
                    Transform::from_xyz(x, y, 1.0),
                ));
            } else {
                commands.spawn((
                    Sprite::from_color(tile_color(tile_id, true), fg_tile_size),
                    Transform::from_xyz(x, y, 1.0),
                ));
            }
        }
    }

    let bg_world_w = level.bg_width as f32 * bg_tile_size.x;
    let bg_world_h = level.bg_height as f32 * bg_tile_size.y;

    for row in 0..level.bg_height as usize {
        for col in 0..level.bg_width as usize {
            let idx = row * level.bg_width as usize + col;
            let tile = level.bg_tiles[idx];
            if tile == 0 {
                continue;
            }

            let x = col as f32 * bg_tile_size.x - bg_world_w * 0.5 + bg_tile_size.x * 0.5;
            let y = bg_world_h * 0.5 - row as f32 * bg_tile_size.y - bg_tile_size.y * 0.5;

            if let Some(texture) = tile_set.as_ref().and_then(|set| set.bg_tiles.get(&tile)) {
                commands.spawn((
                    Sprite {
                        image: texture.clone(),
                        color: Color::srgba(1.0, 1.0, 1.0, 0.55),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 0.0),
                ));
            } else {
                commands.spawn((
                    Sprite::from_color(tile_color(tile, false), bg_tile_size),
                    Transform::from_xyz(x, y, 0.0),
                ));
            }
        }
    }

    if let Some(sprite_lib) = &object_sprites {
        for object in &level.objects {
            if let Some((spe_rel, entry_name)) = resolve_object_sprite(object) {
                if let Some(texture) = sprite_lib.get(spe_rel, &entry_name) {
                    let x = object.var(VAR_X).unwrap_or(0) as f32 - fg_world_w * 0.5;
                    let mut y = fg_world_h * 0.5 - object.var(VAR_Y).unwrap_or(0) as f32;
                    if let Some(image) = images.get(&texture) {
                        y += image.height() as f32 * 0.5;
                    }
                    let (dx, dy, z) = object_render_adjustment(object.type_name.as_deref());
                    commands.spawn((
                        Sprite::from_image(texture),
                        Transform::from_xyz(x + dx, y + dy, z),
                    ));
                }
            }
        }
    }

    let light_glow = images.add(make_radial_glow_texture(96));
    for light in &level.lights {
        let x = light.x as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - light.y as f32;
        let radius = light.outer_radius.max(8) as f32;
        let tint = match light.light_type {
            1 => Color::srgba(0.92, 0.78, 0.44, 0.42),
            3 => Color::srgba(0.73, 0.42, 0.96, 0.45),
            _ => Color::srgba(0.86, 0.84, 0.78, 0.32),
        };

        commands.spawn((
            Sprite {
                image: light_glow.clone(),
                custom_size: Some(Vec2::splat(radius * 2.6)),
                color: tint,
                ..default()
            },
            Transform::from_xyz(x, y, 2.2),
        ));

        commands.spawn((
            Sprite {
                image: light_glow.clone(),
                custom_size: Some(Vec2::splat(radius * 1.2)),
                color: Color::srgba(1.0, 1.0, 1.0, 0.18),
                ..default()
            },
            Transform::from_xyz(x, y, 2.21),
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

    spawn_hud(&mut commands, &level, &audio_state);
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
    audio_state: Res<AudioState>,
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
    let _old_audio = lines.next();
    let controls_line = lines
        .next()
        .unwrap_or("controls: WASD/arrows pan, wheel/Q/E zoom, F1 HUD, M mute, -/+ volume");

    text.0 = format!(
        "{}\nzoom: {}%\n{}\n{}\naudio: {} ({}%)\n{}",
        level_line,
        zoom_percent,
        objects_line,
        lights_line,
        if audio_state.enabled { "on" } else { "off" },
        (audio_state.volume * 100.0).round() as i32,
        controls_line,
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
    let seed = u32::from(tile & 0x7fff);
    let tone = ((seed.wrapping_mul(37) % 120) as f32 + 70.0) / 255.0;
    if foreground {
        Color::srgb(tone * 0.85, tone, tone * 0.9)
    } else {
        Color::srgba(tone * 0.35, tone * 0.4, tone * 0.5, 0.35)
    }
}
