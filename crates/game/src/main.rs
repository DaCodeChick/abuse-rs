//! Debug viewer executable for rendering legacy Abuse level data.
//!
//! This binary wires together runtime viewer modules (assets, camera, HUD, object
//! rendering, and object-driven audio) into an interactive Bevy application.

use std::path::PathBuf;

use abuse_runtime::data::level::LevelData;
use abuse_runtime::viewer::assets::{
    derive_data_root, load_legacy_tile_set, load_object_sprite_library, make_radial_glow_texture,
};
use abuse_runtime::viewer::audio::{
    adjust_audio_volume, spawn_context_audio, sync_audio_volume, toggle_audio, AudioSfxPaths,
    AudioState,
};
use abuse_runtime::viewer::camera::{
    camera_controls, fit_camera_to_level, setup_camera, LevelViewBounds, ViewerCamera,
};
use abuse_runtime::viewer::hud::{spawn_hud, toggle_hud_visibility, update_hud, HudState};
use abuse_runtime::viewer::object_render::ObjectSpritePaths;
use abuse_runtime::viewer::scene::{spawn_bg_tiles, spawn_fg_tiles, spawn_lights, spawn_objects};
use abuse_runtime::AbuseRuntimePlugins;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Default fallback tile size used when archive tile dimensions are unknown.
const FG_TILE_SIZE: f32 = 32.0;

/// Default viewer asset mapping used when no overrides are provided.
#[derive(Resource, Debug, Clone)]
struct ViewerAssetConfig {
    palette_spe_path: String,
    fg_tile_spe_files: Vec<String>,
    bg_tile_spe_files: Vec<String>,
    object_spe_files: Vec<String>,
    object_sprite_paths: ObjectSpritePaths,
    audio_sfx_paths: AudioSfxPaths,
}

impl ViewerAssetConfig {
    fn default_legacy() -> Self {
        Self {
            palette_spe_path: "art/back/backgrnd.spe".to_string(),
            fg_tile_spe_files: vec![
                "art/fore/foregrnd.spe".to_string(),
                "art/fore/techno.spe".to_string(),
                "art/fore/techno2.spe".to_string(),
                "art/fore/techno3.spe".to_string(),
                "art/fore/techno4.spe".to_string(),
                "art/fore/cave.spe".to_string(),
                "art/fore/alien.spe".to_string(),
                "art/fore/trees.spe".to_string(),
                "art/fore/endgame.spe".to_string(),
                "art/fore/trees2.spe".to_string(),
            ],
            bg_tile_spe_files: vec![
                "art/back/backgrnd.spe".to_string(),
                "art/back/intro.spe".to_string(),
                "art/back/city.spe".to_string(),
                "art/back/cave.spe".to_string(),
                "art/back/tech.spe".to_string(),
                "art/back/alienb.spe".to_string(),
                "art/back/green2.spe".to_string(),
                "art/back/galien.spe".to_string(),
            ],
            object_spe_files: vec![
                "art/door.spe".to_string(),
                "art/chars/door.spe".to_string(),
                "art/chars/tdoor.spe".to_string(),
                "art/chars/teleport.spe".to_string(),
                "art/chars/platform.spe".to_string(),
                "art/chars/lightin.spe".to_string(),
                "art/chars/lava.spe".to_string(),
                "art/chars/step.spe".to_string(),
                "art/ball.spe".to_string(),
                "art/compass.spe".to_string(),
                "art/rob2.spe".to_string(),
                "art/misc.spe".to_string(),
            ],
            object_sprite_paths: ObjectSpritePaths {
                door: "art/door.spe".to_string(),
                chars_door: "art/chars/door.spe".to_string(),
                misc: "art/misc.spe".to_string(),
                teleport: "art/chars/teleport.spe".to_string(),
                lava: "art/chars/lava.spe".to_string(),
                ball: "art/ball.spe".to_string(),
                compass: "art/compass.spe".to_string(),
                rob2: "art/rob2.spe".to_string(),
                lightin: "art/chars/lightin.spe".to_string(),
                trap_door: "art/chars/tdoor.spe".to_string(),
                step: "art/chars/step.spe".to_string(),
            },
            audio_sfx_paths: AudioSfxPaths {
                tp_door: "sfx/telept01.wav".to_string(),
                tele2: "sfx/fadeon01.wav".to_string(),
                spring: "sfx/spring03.wav".to_string(),
                lava: "sfx/lava01.wav".to_string(),
                force_field: "sfx/force01.wav".to_string(),
            },
        }
    }

    fn from_env_or_default() -> Self {
        fn csv_env(key: &str, fallback: &[String]) -> Vec<String> {
            std::env::var(key)
                .ok()
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| fallback.to_vec())
        }

        fn scalar_env(key: &str, fallback: &str) -> String {
            std::env::var(key)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        }

        let default = Self::default_legacy();
        Self {
            palette_spe_path: scalar_env("ABUSE_PALETTE_SPE", &default.palette_spe_path),
            fg_tile_spe_files: csv_env("ABUSE_FG_TILE_SPES", &default.fg_tile_spe_files),
            bg_tile_spe_files: csv_env("ABUSE_BG_TILE_SPES", &default.bg_tile_spe_files),
            object_spe_files: csv_env("ABUSE_OBJECT_SPES", &default.object_spe_files),
            object_sprite_paths: ObjectSpritePaths {
                door: scalar_env("ABUSE_SPRITE_SPE_DOOR", &default.object_sprite_paths.door),
                chars_door: scalar_env(
                    "ABUSE_SPRITE_SPE_CHARS_DOOR",
                    &default.object_sprite_paths.chars_door,
                ),
                misc: scalar_env("ABUSE_SPRITE_SPE_MISC", &default.object_sprite_paths.misc),
                teleport: scalar_env(
                    "ABUSE_SPRITE_SPE_TELEPORT",
                    &default.object_sprite_paths.teleport,
                ),
                lava: scalar_env("ABUSE_SPRITE_SPE_LAVA", &default.object_sprite_paths.lava),
                ball: scalar_env("ABUSE_SPRITE_SPE_BALL", &default.object_sprite_paths.ball),
                compass: scalar_env(
                    "ABUSE_SPRITE_SPE_COMPASS",
                    &default.object_sprite_paths.compass,
                ),
                rob2: scalar_env("ABUSE_SPRITE_SPE_ROB2", &default.object_sprite_paths.rob2),
                lightin: scalar_env(
                    "ABUSE_SPRITE_SPE_LIGHTIN",
                    &default.object_sprite_paths.lightin,
                ),
                trap_door: scalar_env(
                    "ABUSE_SPRITE_SPE_TRAP_DOOR",
                    &default.object_sprite_paths.trap_door,
                ),
                step: scalar_env("ABUSE_SPRITE_SPE_STEP", &default.object_sprite_paths.step),
            },
            audio_sfx_paths: AudioSfxPaths {
                tp_door: scalar_env("ABUSE_SFX_TP_DOOR", &default.audio_sfx_paths.tp_door),
                tele2: scalar_env("ABUSE_SFX_TELE2", &default.audio_sfx_paths.tele2),
                spring: scalar_env("ABUSE_SFX_SPRING", &default.audio_sfx_paths.spring),
                lava: scalar_env("ABUSE_SFX_LAVA", &default.audio_sfx_paths.lava),
                force_field: scalar_env(
                    "ABUSE_SFX_FORCE_FIELD",
                    &default.audio_sfx_paths.force_field,
                ),
            },
        }
    }
}

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
        .insert_resource(ViewerAssetConfig::from_env_or_default())
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
    assets_cfg: Res<ViewerAssetConfig>,
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

    let fg_tile_refs: Vec<&str> = assets_cfg
        .fg_tile_spe_files
        .iter()
        .map(String::as_str)
        .collect();
    let bg_tile_refs: Vec<&str> = assets_cfg
        .bg_tile_spe_files
        .iter()
        .map(String::as_str)
        .collect();
    let object_spe_refs: Vec<&str> = assets_cfg
        .object_spe_files
        .iter()
        .map(String::as_str)
        .collect();

    let tile_set = load_legacy_tile_set(
        level_path,
        &fg_tile_refs,
        &bg_tile_refs,
        assets_cfg.palette_spe_path.as_str(),
        &mut images,
        FG_TILE_SIZE,
    )
    .inspect_err(|err| warn!("Tile asset loading failed, falling back to debug colors: {err}"))
    .ok();

    let object_sprites = load_object_sprite_library(
        level_path,
        &object_spe_refs,
        assets_cfg.palette_spe_path.as_str(),
        &mut images,
    )
    .inspect_err(|err| warn!("Object sprite library failed to load: {err}"))
    .ok();

    spawn_context_audio(
        &mut commands,
        &asset_server,
        &audio_state,
        &level,
        &assets_cfg.audio_sfx_paths,
    );

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
        &assets_cfg.object_sprite_paths,
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
