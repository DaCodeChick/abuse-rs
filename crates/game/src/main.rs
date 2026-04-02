use std::path::PathBuf;
use std::{collections::HashMap, fs::File, io::Read, io::Seek, io::SeekFrom, path::Path};

use abuse_runtime::AbuseRuntimePlugins;
use abuse_runtime::data::level::{LevelData, VAR_X, VAR_Y};
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

#[derive(Component)]
struct ViewerCamera;

#[derive(Component)]
struct ViewerHud;

#[derive(Component)]
struct DebugMarker;

#[derive(Resource, Debug, Clone, Copy)]
struct HudState {
    visible: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
struct DebugOverlayState {
    markers_visible: bool,
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

fn main() {
    let level_path = std::env::args().nth(1).map(PathBuf::from);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ViewerConfig { level_path })
        .insert_resource(HudState { visible: true })
        .insert_resource(DebugOverlayState {
            markers_visible: true,
        })
        .add_plugins(AbuseRuntimePlugins)
        .add_systems(Startup, (setup_camera, load_level_view).chain())
        .add_systems(
            Update,
            (
                camera_controls,
                toggle_hud_visibility,
                toggle_debug_markers,
                update_hud,
            ),
        )
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
            "level: {}\nzoom: 100%\nobjects: {}\nlights: {}\ncontrols: WASD/arrows pan, wheel/Q/E zoom, F1 HUD, F2 markers",
            level_name,
            level.objects.len(),
            level.lights.len()
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
            let tile_id = tile;
            if tile_id == 0 {
                continue;
            }

            let x = col as f32 * bg_tile_size.x - bg_world_w * 0.5 + bg_tile_size.x * 0.5;
            let y = bg_world_h * 0.5 - row as f32 * bg_tile_size.y - bg_tile_size.y * 0.5;

            if let Some(texture) = tile_set.as_ref().and_then(|set| set.bg_tiles.get(&tile_id)) {
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
                    Sprite::from_color(tile_color(tile_id, false), bg_tile_size),
                    Transform::from_xyz(x, y, 0.0),
                ));
            }
        }
    }

    for object in &level.objects {
        let x = object.var(VAR_X).unwrap_or(0) as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - object.var(VAR_Y).unwrap_or(0) as f32;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.92, 0.28, 0.2, 0.9), Vec2::splat(7.0)),
            Transform::from_xyz(x, y, 3.0),
            DebugMarker,
        ));
    }

    for light in &level.lights {
        let x = light.x as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - light.y as f32;
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.95, 0.25, 0.65), Vec2::splat(10.0)),
            Transform::from_xyz(x, y, 4.0),
            DebugMarker,
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
        .unwrap_or("controls: WASD/arrows pan, wheel/Q/E zoom, F1 HUD, F2 markers");

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

fn toggle_debug_markers(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut overlay_state: ResMut<DebugOverlayState>,
    mut marker_query: Query<&mut Visibility, With<DebugMarker>>,
) {
    if !keyboard.just_pressed(KeyCode::F2) {
        return;
    }

    overlay_state.markers_visible = !overlay_state.markers_visible;
    for mut visibility in &mut marker_query {
        *visibility = if overlay_state.markers_visible {
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

        file.seek(SeekFrom::Start(u64::from(entry.offset)))
            .map_err(|err| err.to_string())?;
        let width = file
            .read_u16::<LittleEndian>()
            .map_err(|err| err.to_string())? as u32;
        let height = file
            .read_u16::<LittleEndian>()
            .map_err(|err| err.to_string())? as u32;
        let pixel_count = usize::try_from(width.saturating_mul(height)).map_err(|_| {
            format!(
                "tile image too large in {} entry {}",
                path.display(),
                entry.name
            )
        })?;

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

        out.push((tile_id, rgba, width, height));
    }

    Ok(out)
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
