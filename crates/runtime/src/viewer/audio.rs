use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::data::level::LevelData;

#[derive(Resource, Debug, Clone)]
pub struct AudioState {
    pub enabled: bool,
    pub volume: f32,
}

#[derive(Component)]
pub struct OneShotAudio;

pub fn spawn_context_audio(
    commands: &mut Commands,
    asset_server: &AssetServer,
    audio_state: &AudioState,
    level: &LevelData,
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
            "sfx/telept01.wav",
            audio_state.volume * 0.45,
            Some(1.8),
        );
    }
    if has_tele2 {
        spawn_one_shot(
            commands,
            asset_server,
            "sfx/fadeon01.wav",
            audio_state.volume * 0.38,
            Some(1.6),
        );
    }
    if has_spring {
        spawn_one_shot(
            commands,
            asset_server,
            "sfx/spring03.wav",
            audio_state.volume * 0.35,
            Some(1.0),
        );
    }
    if has_lava {
        spawn_one_shot(
            commands,
            asset_server,
            "sfx/lava01.wav",
            audio_state.volume * 0.3,
            Some(1.4),
        );
    }
    if has_force_field {
        spawn_one_shot(
            commands,
            asset_server,
            "sfx/force01.wav",
            audio_state.volume * 0.32,
            Some(1.2),
        );
    }
}

pub fn spawn_one_shot(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: &'static str,
    volume: f32,
    max_duration_secs: Option<f32>,
) {
    let mut settings =
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume.clamp(0.0, 1.0)));
    if let Some(seconds) = max_duration_secs {
        settings = settings.with_duration(Duration::from_secs_f32(seconds.max(0.05)));
    }

    commands.spawn((
        AudioPlayer::new(asset_server.load(path)),
        settings,
        OneShotAudio,
    ));
}

pub fn toggle_audio(keyboard: Res<ButtonInput<KeyCode>>, mut audio_state: ResMut<AudioState>) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        audio_state.enabled = !audio_state.enabled;
    }
}

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
