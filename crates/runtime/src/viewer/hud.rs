//! Heads-up-display helpers for the level viewer.

use bevy::prelude::*;

use crate::data::level::LevelData;
use crate::viewer::audio::AudioState;
use crate::viewer::camera::ViewerCamera;

/// Marker component for the viewer HUD text entity.
#[derive(Component)]
pub struct ViewerHud;

/// Visibility state for the viewer HUD.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HudState {
    /// Whether HUD should currently be visible.
    pub visible: bool,
}

impl HudState {
    /// Constructs an enabled/visible HUD state.
    pub const fn visible() -> Self {
        Self { visible: true }
    }
}

/// Spawns initial HUD text for the loaded level.
pub fn spawn_hud(commands: &mut Commands, level: &LevelData, audio: &AudioState) {
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

/// Updates HUD zoom/audio readouts from current runtime state.
pub fn update_hud(
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

/// Toggles HUD visibility with the `F1` key.
pub fn toggle_hud_visibility(
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
