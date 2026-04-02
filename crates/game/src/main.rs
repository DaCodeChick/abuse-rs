//! Debug viewer executable for rendering legacy Abuse level data.
//!
//! This binary wires together runtime viewer modules (assets, camera, HUD, object
//! rendering, and object-driven audio) into an interactive Bevy application.

use std::path::PathBuf;

use abuse_runtime::AbuseRuntimePlugins;
use abuse_runtime::data::level::LevelData;
use abuse_runtime::viewer::assets::{
    derive_data_root, load_legacy_tile_set, load_object_sprite_library, make_radial_glow_texture,
};
use abuse_runtime::viewer::audio::{
    AudioState, adjust_audio_volume, spawn_context_audio, sync_audio_volume, toggle_audio,
};
use abuse_runtime::viewer::camera::{
    LevelViewBounds, ViewerCamera, camera_controls, fit_camera_to_level, setup_camera,
};
use abuse_runtime::viewer::constants::FG_TILE_SIZE;
use abuse_runtime::viewer::hud::{HudState, spawn_hud, toggle_hud_visibility, update_hud};
use abuse_runtime::viewer::scene::{spawn_bg_tiles, spawn_fg_tiles, spawn_lights, spawn_objects};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Command-line configuration for the viewer executable.
#[derive(Resource, Debug, Clone)]
struct ViewerConfig {
    /// Optional path to a legacy level SPE file.
    level_path: Option<PathBuf>,
}

/// Application entrypoint for the viewer binary.
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
        .insert_resource(HudState::visible())
        .insert_resource(AudioState::default_enabled())
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

/// Loads level data, spawns scene visuals, and initializes HUD/audio state.
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

    let bg_world_w = level.bg_width as f32 * bg_tile_size.x;
    let bg_world_h = level.bg_height as f32 * bg_tile_size.y;

    spawn_fg_tiles(
        &mut commands,
        &level,
        tile_set.as_ref(),
        fg_tile_size,
        fg_world_w,
        fg_world_h,
    );

    spawn_bg_tiles(
        &mut commands,
        &level,
        tile_set.as_ref(),
        bg_tile_size,
        bg_world_w,
        bg_world_h,
    );

    spawn_objects(
        &mut commands,
        &level,
        object_sprites.as_ref(),
        &images,
        fg_world_w,
        fg_world_h,
    );

    let light_glow = images.add(make_radial_glow_texture(96));
    spawn_lights(&mut commands, &level, light_glow, fg_world_w, fg_world_h);

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
