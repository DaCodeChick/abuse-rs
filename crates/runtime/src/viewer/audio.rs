//! Object-driven viewer audio utilities.
//!
//! The viewer intentionally avoids heuristic ambient loops and instead emits short
//! one-shot sounds when specific object classes are present in the loaded level.

use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::data::level::LevelData;

/// Configurable one-shot SFX paths used for object-context audio cues.
#[derive(Debug, Clone)]
pub struct AudioSfxPaths {
    /// Teleporter/door cue.
    pub tp_door: String,
    /// Teleport beam/electric cue.
    pub tele2: String,
    /// Spring cue.
    pub spring: String,
    /// Lava ambience cue.
    pub lava: String,
    /// Force-field/lightning cue.
    pub force_field: String,
}

/// Runtime audio controls exposed to viewer systems.
#[derive(Resource, Debug, Clone)]
pub struct AudioState {
    /// Whether viewer SFX are currently enabled.
    pub enabled: bool,
    /// Master SFX volume in range `[0.0, 1.0]`.
    pub volume: f32,
}

impl AudioState {
    /// Default interactive viewer audio state.
    pub const fn default_enabled() -> Self {
        Self {
            enabled: true,
            volume: 0.45,
        }
    }
}

/// Marker component for one-shot audio entities.
#[derive(Component)]
pub struct OneShotAudio;

/// Spawns object-context one-shot sounds for the current level.
pub fn spawn_context_audio(
    commands: &mut Commands,
    asset_server: &AssetServer,
    audio_state: &AudioState,
    level: &LevelData,
    sfx_paths: &AudioSfxPaths,
) {
    if !audio_state.enabled || audio_state.volume <= 0.0 {
        return;
    }

    let mut has_tp_door = false;
    let mut has_tele2 = false;
    let mut has_spring = false;
    let mut has_lava = false;
    let mut has_force_field = false;

    for object in &level.objects {
        if let Some(name) = object.type_name.as_deref() {
            match name {
                "TP_DOOR" | "NEXT_LEVEL" | "NEXT_LEVEL_TOP" => has_tp_door = true,
                "TELE2" | "TELE_BEAM" => has_tele2 = true,
                "SPRING" => has_spring = true,
                "LAVA" => has_lava = true,
                "FORCE_FIELD" | "LIGHTIN" => has_force_field = true,
                _ => {}
            }
        }
    }

    if has_tp_door {
        spawn_one_shot(
            commands,
            asset_server,
            sfx_paths.tp_door.as_str(),
            audio_state.volume * 0.45,
            Some(1.8),
        );
    }
    if has_tele2 {
        spawn_one_shot(
            commands,
            asset_server,
            sfx_paths.tele2.as_str(),
            audio_state.volume * 0.38,
            Some(1.6),
        );
    }
    if has_spring {
        spawn_one_shot(
            commands,
            asset_server,
            sfx_paths.spring.as_str(),
            audio_state.volume * 0.35,
            Some(1.0),
        );
    }
    if has_lava {
        spawn_one_shot(
            commands,
            asset_server,
            sfx_paths.lava.as_str(),
            audio_state.volume * 0.3,
            Some(1.4),
        );
    }
    if has_force_field {
        spawn_one_shot(
            commands,
            asset_server,
            sfx_paths.force_field.as_str(),
            audio_state.volume * 0.32,
            Some(1.2),
        );
    }
}

/// Spawns one-shot SFX with optional max playback duration.
pub fn spawn_one_shot(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: &str,
    volume: f32,
    max_duration_secs: Option<f32>,
) {
    let mut settings =
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume.clamp(0.0, 1.0)));
    if let Some(seconds) = max_duration_secs {
        settings = settings.with_duration(Duration::from_secs_f32(seconds.max(0.05)));
    }

    commands.spawn((
        AudioPlayer::new(asset_server.load(path.to_string())),
        settings,
        OneShotAudio,
    ));
}

/// Toggles global viewer SFX enabled state with `M`.
pub fn toggle_audio(keyboard: Res<ButtonInput<KeyCode>>, mut audio_state: ResMut<AudioState>) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        audio_state.enabled = !audio_state.enabled;
    }
}

/// Adjusts viewer SFX volume with `-` and `+` keys.
pub fn adjust_audio_volume(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut audio_state: ResMut<AudioState>,
) {
    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        audio_state.volume = (audio_state.volume - 0.05).clamp(0.0, 1.0);
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        audio_state.volume = (audio_state.volume + 0.05).clamp(0.0, 1.0);
        changed = true;
    }

    if changed && audio_state.volume <= 0.0 {
        audio_state.enabled = false;
    } else if changed {
        audio_state.enabled = true;
    }
}

/// Applies current audio state to active one-shot sinks.
pub fn sync_audio_volume(
    audio_state: Res<AudioState>,
    mut oneshots: Query<&mut AudioSink, With<OneShotAudio>>,
) {
    if !audio_state.is_changed() {
        return;
    }

    for mut sink in &mut oneshots {
        sink.set_volume(if audio_state.enabled {
            Volume::Linear((audio_state.volume * 0.5).clamp(0.0, 1.0))
        } else {
            Volume::SILENT
        });
    }
}
