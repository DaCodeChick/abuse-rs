use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs::File, io::Read, io::Seek, io::SeekFrom, path::Path};

use abuse_runtime::AbuseRuntimePlugins;
use abuse_runtime::data::level::{LevelData, VAR_CUR_FRAME, VAR_X, VAR_Y};
use abuse_runtime::data::spe::{SpeDirectory, SpecType};
use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::MessageReader;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use byteorder::{LittleEndian, ReadBytesExt};

const FG_TILE_SIZE: f32 = 32.0;
const CAMERA_PAN_SPEED: f32 = 900.0;
const CAMERA_ZOOM_STEP: f32 = 0.1;
const CAMERA_MIN_SCALE: f32 = 0.2;
const CAMERA_MAX_SCALE: f32 = 12.0;

const FG_TILE_SPE_FILES: &[&str] = &[
    "art/fore/foregrnd.spe",
    "art/fore/techno.spe",
    "art/fore/techno2.spe",
    "art/fore/techno3.spe",
    "art/fore/techno4.spe",
    "art/fore/cave.spe",
    "art/fore/alien.spe",
    "art/fore/trees.spe",
    "art/fore/endgame.spe",
    "art/fore/trees2.spe",
];

const BG_TILE_SPE_FILES: &[&str] = &[
    "art/back/backgrnd.spe",
    "art/back/intro.spe",
    "art/back/city.spe",
    "art/back/cave.spe",
    "art/back/tech.spe",
    "art/back/alienb.spe",
    "art/back/green2.spe",
    "art/back/galien.spe",
];

const OBJECT_SPE_FILES: &[&str] = &[
    "art/door.spe",
    "art/chars/door.spe",
    "art/chars/tdoor.spe",
    "art/chars/teleport.spe",
    "art/chars/platform.spe",
    "art/chars/lightin.spe",
    "art/chars/lava.spe",
    "art/chars/step.spe",
    "art/ball.spe",
    "art/compass.spe",
    "art/rob2.spe",
    "art/misc.spe",
];

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

#[derive(Debug, Clone)]
struct LegacyTileSet {
    fg_tiles: HashMap<u16, Handle<Image>>,
    bg_tiles: HashMap<u16, Handle<Image>>,
    fg_tile_size: Vec2,
    bg_tile_size: Vec2,
}

#[derive(Debug, Clone)]
struct ObjectSpriteLibrary {
    sprites: HashMap<(String, String), Handle<Image>>,
}

impl ObjectSpriteLibrary {
    fn get(&self, spe_path: &str, entry_name: &str) -> Option<Handle<Image>> {
        self.sprites
            .get(&(
                spe_path.to_ascii_lowercase(),
                entry_name.to_ascii_lowercase(),
            ))
            .cloned()
    }
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
    let level_name = std::path::Path::new(&level.name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&level.name);

    commands.spawn((
        Text::new(format!(
            "level: {}\nzoom: 100%\nobjects: {}\nlights: {}\ncontrols: WASD/arrows pan, wheel/Q/E zoom, F1 HUD",
            level_name,
            level.objects.len(),
            level.lights.len(),
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

    let tile_set = load_legacy_tile_set(level_path, &mut images)
        .inspect_err(|err| warn!("Tile asset loading failed, falling back to debug colors: {err}"))
        .ok();

    let object_sprites = load_object_sprite_library(level_path, &mut images)
        .inspect_err(|err| warn!("Object sprite library failed to load: {err}"))
        .ok();

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

    spawn_hud(&mut commands, &level);
}

fn object_render_adjustment(type_name: Option<&str>) -> (f32, f32, f32) {
    match type_name.unwrap_or_default() {
        "NEXT_LEVEL" => (0.0, -4.0, 2.7),
        "NEXT_LEVEL_TOP" => (0.0, -8.0, 2.71),
        "TELE_BEAM" => (0.0, -6.0, 2.72),
        "TP_DOOR" | "SWITCH_DOOR" | "TRAP_DOOR2" | "TRAP_DOOR3" => (0.0, -3.0, 2.65),
        "SPRING" => (0.0, -2.0, 2.6),
        "LAVA" => (0.0, -1.0, 2.55),
        "HEALTH" | "POWER_FAST" | "POWER_FLY" | "POWER_SNEAKY" | "POWER_HEALTH" => (0.0, 6.0, 2.9),
        "WHO" => (0.0, -4.0, 2.8),
        _ => (0.0, 0.0, 2.5),
    }
}

fn resolve_object_sprite(
    object: &abuse_runtime::data::level::LoadedObject,
) -> Option<(&'static str, String)> {
    let type_name = object.type_name.as_deref()?;
    let state_name = object.state_name.as_deref().unwrap_or("stopped");
    let frame = object.var(VAR_CUR_FRAME).unwrap_or(0).max(0) as usize;

    match type_name {
        "TP_DOOR" => {
            let frame_num = (frame % 5) + 1;
            Some(("art/door.spe", format!("door{frame_num:04}.pcx")))
        }
        "SWITCH_DOOR" => {
            let frame_num = match state_name {
                "stopped" => 6,
                "blocking" => 1,
                "running" => 6usize.saturating_sub(frame % 6),
                "walking" => (frame % 6) + 1,
                _ => 1,
            };
            Some(("art/chars/door.spe", format!("door{frame_num:04}.pcx")))
        }
        "TP_DOOR_INVIS" => Some(("art/misc.spe", "clone_icon".to_string())),
        "NEXT_LEVEL" => Some(("art/misc.spe", "end_port2".to_string())),
        "NEXT_LEVEL_TOP" => Some(("art/misc.spe", "end_port1".to_string())),
        "TELE_BEAM" => {
            let frame_num = ((frame % 5) + 1) as u32;
            Some(("art/chars/teleport.spe", format!("beam{frame_num:04}.pcx")))
        }
        "SPRING" => {
            if state_name == "running" {
                Some(("art/misc.spe", "spri0001.pcx".to_string()))
            } else {
                Some(("art/misc.spe", "spri0004.pcx".to_string()))
            }
        }
        "LAVA" => {
            let frame_num = ((frame % 15) + 1) as u32;
            Some(("art/chars/lava.spe", format!("lava{frame_num:04}.pcx")))
        }
        "HEALTH" => Some(("art/ball.spe", "heart".to_string())),
        "POWER_FAST" => Some(("art/misc.spe", "fast".to_string())),
        "POWER_FLY" => Some(("art/misc.spe", "fly".to_string())),
        "POWER_SNEAKY" => Some(("art/misc.spe", "sneaky".to_string())),
        "POWER_HEALTH" => Some(("art/misc.spe", "b_check".to_string())),
        "COMPASS" => Some(("art/compass.spe", "compass".to_string())),
        "WHO" => {
            let entry = match state_name {
                "turn_around" => format!("wtrn{:04}.pcx", (frame % 9) + 1),
                _ => format!("wgo{:04}.pcx", (frame % 3) + 1),
            };
            Some(("art/rob2.spe", entry))
        }
        "FORCE_FIELD" => Some(("art/misc.spe", "force_field".to_string())),
        "LIGHTIN" => {
            let frame_num = ((frame % 9) + 1) as u32;
            Some(("art/chars/lightin.spe", format!("lite{frame_num:04}.pcx")))
        }
        "TRAP_DOOR2" => {
            let frame_num = match state_name {
                "stopped" => 1,
                "blocking" => 7,
                "running" => (frame % 7) + 1,
                "walking" => 7usize.saturating_sub(frame % 7),
                _ => 1,
            };
            Some(("art/chars/tdoor.spe", format!("tdor{frame_num:04}.pcx")))
        }
        "TRAP_DOOR3" => {
            let frame_num = match state_name {
                "stopped" => 1,
                "blocking" => 7,
                "running" => (frame % 7) + 1,
                "walking" => 7usize.saturating_sub(frame % 7),
                _ => 1,
            };
            Some(("art/chars/tdoor.spe", format!("cdor{frame_num:04}.pcx")))
        }
        "TELE2" => {
            if state_name == "running" {
                let frame_num = ((frame % 15) + 1) as u32;
                Some(("art/chars/teleport.spe", format!("elec{frame_num:04}.pcx")))
            } else {
                Some(("art/chars/teleport.spe", "close".to_string()))
            }
        }
        "STEP" => {
            if state_name == "stopped" {
                Some(("art/chars/step.spe", "step".to_string()))
            } else {
                Some(("art/chars/step.spe", "step_gone".to_string()))
            }
        }
        "SWITCH" | "SWITCH_ONCE" | "SWITCH_DELAY" => {
            let frame_num = ((frame % 18) + 1) as u32;
            Some(("art/misc.spe", format!("swit{frame_num:04}.pcx")))
        }
        "SWITCH_BALL" => {
            let frame_num = if state_name == "running" {
                10 + (frame % 9)
            } else {
                1 + (frame % 9)
            };
            Some(("art/misc.spe", format!("swit{frame_num:04}.pcx")))
        }
        _ => None,
    }
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
        .unwrap_or("controls: WASD/arrows pan, wheel/Q/E zoom, F1 HUD");

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
    let seed = u32::from(tile & 0x7fff);
    let tone = ((seed.wrapping_mul(37) % 120) as f32 + 70.0) / 255.0;
    if foreground {
        Color::srgb(tone * 0.85, tone, tone * 0.9)
    } else {
        Color::srgba(tone * 0.35, tone * 0.4, tone * 0.5, 0.35)
    }
}

fn load_legacy_tile_set(
    level_path: &Path,
    images: &mut Assets<Image>,
) -> Result<LegacyTileSet, String> {
    let data_root = level_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| format!("could not derive data root from {}", level_path.display()))?;

    let palette = read_palette(&data_root.join("art/back/backgrnd.spe"))?;

    let mut fg_tiles = HashMap::new();
    let mut bg_tiles = HashMap::new();
    let mut fg_tile_size = Vec2::new(FG_TILE_SIZE, FG_TILE_SIZE);
    let mut bg_tile_size = Vec2::new(FG_TILE_SIZE, FG_TILE_SIZE);

    for rel_path in FG_TILE_SPE_FILES {
        let path = data_root.join(rel_path);
        if !path.exists() {
            continue;
        }
        let loaded = read_tile_images_from_spe(&path, SpecType::ForeTile, &palette)?;
        for (tile_id, rgba, width, height) in loaded {
            fg_tile_size = Vec2::new(width as f32, height as f32);
            let texture = image_from_rgba(width, height, rgba);
            fg_tiles.insert(tile_id, images.add(texture));
        }
    }

    for rel_path in BG_TILE_SPE_FILES {
        let path = data_root.join(rel_path);
        if !path.exists() {
            continue;
        }
        let loaded = read_tile_images_from_spe(&path, SpecType::BackTile, &palette)?;
        for (tile_id, rgba, width, height) in loaded {
            bg_tile_size = Vec2::new(width as f32, height as f32);
            let texture = image_from_rgba(width, height, rgba);
            bg_tiles.insert(tile_id, images.add(texture));
        }
    }

    if fg_tiles.is_empty() && bg_tiles.is_empty() {
        return Err("no foreground/background tile textures found".to_string());
    }

    Ok(LegacyTileSet {
        fg_tiles,
        bg_tiles,
        fg_tile_size,
        bg_tile_size,
    })
}

fn load_object_sprite_library(
    level_path: &Path,
    images: &mut Assets<Image>,
) -> Result<ObjectSpriteLibrary, String> {
    let data_root = level_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| format!("could not derive data root from {}", level_path.display()))?;
    let fallback_palette = read_palette(&data_root.join("art/back/backgrnd.spe"))?;

    let mut sprites = HashMap::new();
    for rel in OBJECT_SPE_FILES {
        let path = data_root.join(rel);
        if !path.exists() {
            continue;
        }

        let directory = SpeDirectory::open_lenient(&path).map_err(|err| err.to_string())?;
        let palette = read_palette(&path).unwrap_or_else(|_| fallback_palette.clone());
        let mut file = File::open(&path).map_err(|err| err.to_string())?;

        for entry in directory.entries.iter().filter(|entry| {
            matches!(
                entry.spec_type,
                SpecType::Image | SpecType::Character | SpecType::Character2
            )
        }) {
            let (rgba, width, height) = read_image_entry(&mut file, entry.offset, &palette)?;
            let handle = images.add(image_from_rgba(width, height, rgba));
            sprites.insert(
                (rel.to_ascii_lowercase(), entry.name.to_ascii_lowercase()),
                handle,
            );
        }
    }

    Ok(ObjectSpriteLibrary { sprites })
}

fn read_palette(path: &Path) -> Result<Vec<[u8; 3]>, String> {
    let directory = SpeDirectory::open_lenient(path).map_err(|err| err.to_string())?;
    let palette_entry = directory
        .entries
        .iter()
        .find(|e| e.spec_type == SpecType::Palette && e.name == "palette")
        .or_else(|| {
            directory
                .entries
                .iter()
                .find(|e| e.spec_type == SpecType::Palette)
        })
        .ok_or_else(|| format!("no palette entry in {}", path.display()))?;

    let mut file = File::open(path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(u64::from(palette_entry.offset)))
        .map_err(|err| err.to_string())?;

    let count = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as usize;

    let mut colors = Vec::with_capacity(count);
    let mut max_component = 0_u8;
    for _ in 0..count {
        let r = file.read_u8().map_err(|err| err.to_string())?;
        let g = file.read_u8().map_err(|err| err.to_string())?;
        let b = file.read_u8().map_err(|err| err.to_string())?;
        max_component = max_component.max(r).max(g).max(b);
        colors.push([r, g, b]);
    }

    if max_component <= 63 {
        for color in &mut colors {
            color[0] = color[0].saturating_mul(4);
            color[1] = color[1].saturating_mul(4);
            color[2] = color[2].saturating_mul(4);
        }
    }

    Ok(colors)
}

fn read_tile_images_from_spe(
    path: &Path,
    tile_type: SpecType,
    palette: &[[u8; 3]],
) -> Result<Vec<(u16, Vec<u8>, u32, u32)>, String> {
    let directory = SpeDirectory::open_lenient(path).map_err(|err| err.to_string())?;
    let mut file = File::open(path).map_err(|err| err.to_string())?;

    let mut out = Vec::new();
    for entry in directory
        .entries
        .iter()
        .filter(|entry| entry.spec_type == tile_type)
    {
        let Ok(tile_id) = entry.name.parse::<u16>() else {
            continue;
        };

        let (rgba, width, height) = read_image_entry(&mut file, entry.offset, palette)?;
        out.push((tile_id, rgba, width, height));
    }

    Ok(out)
}

fn read_image_entry(
    file: &mut File,
    offset: u32,
    palette: &[[u8; 3]],
) -> Result<(Vec<u8>, u32, u32), String> {
    file.seek(SeekFrom::Start(u64::from(offset)))
        .map_err(|err| err.to_string())?;

    let width = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as u32;
    let height = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as u32;
    let pixel_count = usize::try_from(width.saturating_mul(height))
        .map_err(|_| format!("image too large: {}x{}", width, height))?;

    let mut indexed = vec![0_u8; pixel_count];
    file.read_exact(&mut indexed)
        .map_err(|err| err.to_string())?;

    let mut rgba = vec![0_u8; pixel_count * 4];
    for (i, idx) in indexed.into_iter().enumerate() {
        let color = palette.get(idx as usize).copied().unwrap_or([0, 0, 0]);
        let base = i * 4;
        rgba[base] = color[0];
        rgba[base + 1] = color[1];
        rgba[base + 2] = color[2];
        rgba[base + 3] = if idx == 0 { 0 } else { 255 };
    }

    Ok((rgba, width, height))
}

fn image_from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn make_radial_glow_texture(size: u32) -> Image {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    let c = size as f32 * 0.5;
    let max_r = c - 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (1.0 - dist / max_r).clamp(0.0, 1.0);
            let a = (t * t * 255.0) as u8;
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 255;
            rgba[i + 2] = 255;
            rgba[i + 3] = a;
        }
    }

    image_from_rgba(size, size, rgba)
}
