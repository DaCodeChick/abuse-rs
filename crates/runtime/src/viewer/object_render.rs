//! Object sprite resolution and placement helpers.

use crate::data::level::{LoadedObject, ObjectVar};

/// Sprite archive path configuration used by object sprite resolution.
#[derive(Debug, Clone)]
pub struct ObjectSpritePaths {
    /// Archive path for standard level doors.
    pub door: String,
    /// Archive path for character door sprites.
    pub chars_door: String,
    /// Archive path for misc UI/pickup sprites.
    pub misc: String,
    /// Archive path for teleport effects.
    pub teleport: String,
    /// Archive path for lava sprites.
    pub lava: String,
    /// Archive path for heart/health sprites.
    pub ball: String,
    /// Archive path for compass sprites.
    pub compass: String,
    /// Archive path for WHO character sprites.
    pub rob2: String,
    /// Archive path for lightning/force effects.
    pub lightin: String,
    /// Archive path for trap doors.
    pub trap_door: String,
    /// Archive path for step platform sprites.
    pub step: String,
}

/// Returns draw offset and depth for known object classes.
pub fn object_render_adjustment(type_name: Option<&str>) -> (f32, f32, f32) {
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

/// Resolves an object's legacy sprite archive path and frame entry name.
pub fn resolve_object_sprite<'a>(
    object: &LoadedObject,
    sprite_paths: &'a ObjectSpritePaths,
) -> Option<(&'a str, String)> {
    let type_name = object.type_name.as_deref()?;
    let state_name = object.state_name.as_deref().unwrap_or("stopped");
    let frame = object.var(ObjectVar::CurFrame).unwrap_or(0).max(0) as usize;

    match type_name {
        "TP_DOOR" => {
            let frame_num = (frame % 5) + 1;
            Some((
                sprite_paths.door.as_str(),
                format!("door{frame_num:04}.pcx"),
            ))
        }
        "SWITCH_DOOR" => {
            let frame_num = match state_name {
                "stopped" => 6,
                "blocking" => 1,
                "running" => 6usize.saturating_sub(frame % 6),
                "walking" => (frame % 6) + 1,
                _ => 1,
            };
            Some((
                sprite_paths.chars_door.as_str(),
                format!("door{frame_num:04}.pcx"),
            ))
        }
        "TP_DOOR_INVIS" => Some((sprite_paths.misc.as_str(), "clone_icon".to_string())),
        "NEXT_LEVEL" => Some((sprite_paths.misc.as_str(), "end_port2".to_string())),
        "NEXT_LEVEL_TOP" => Some((sprite_paths.misc.as_str(), "end_port1".to_string())),
        "TELE_BEAM" => {
            let frame_num = ((frame % 5) + 1) as u32;
            Some((
                sprite_paths.teleport.as_str(),
                format!("beam{frame_num:04}.pcx"),
            ))
        }
        "SPRING" => {
            if state_name == "running" {
                Some((sprite_paths.misc.as_str(), "spri0001.pcx".to_string()))
            } else {
                Some((sprite_paths.misc.as_str(), "spri0004.pcx".to_string()))
            }
        }
        "LAVA" => {
            let frame_num = ((frame % 15) + 1) as u32;
            Some((
                sprite_paths.lava.as_str(),
                format!("lava{frame_num:04}.pcx"),
            ))
        }
        "HEALTH" => Some((sprite_paths.ball.as_str(), "heart".to_string())),
        "POWER_FAST" => Some((sprite_paths.misc.as_str(), "fast".to_string())),
        "POWER_FLY" => Some((sprite_paths.misc.as_str(), "fly".to_string())),
        "POWER_SNEAKY" => Some((sprite_paths.misc.as_str(), "sneaky".to_string())),
        "POWER_HEALTH" => Some((sprite_paths.misc.as_str(), "b_check".to_string())),
        "COMPASS" => Some((sprite_paths.compass.as_str(), "compass".to_string())),
        "WHO" => {
            let entry = match state_name {
                "turn_around" => format!("wtrn{:04}.pcx", (frame % 9) + 1),
                _ => format!("wgo{:04}.pcx", (frame % 3) + 1),
            };
            Some((sprite_paths.rob2.as_str(), entry))
        }
        "FORCE_FIELD" => Some((sprite_paths.misc.as_str(), "force_field".to_string())),
        "LIGHTIN" => {
            let frame_num = ((frame % 9) + 1) as u32;
            Some((
                sprite_paths.lightin.as_str(),
                format!("lite{frame_num:04}.pcx"),
            ))
        }
        "TRAP_DOOR2" => {
            let frame_num = match state_name {
                "stopped" => 1,
                "blocking" => 7,
                "running" => (frame % 7) + 1,
                "walking" => 7usize.saturating_sub(frame % 7),
                _ => 1,
            };
            Some((
                sprite_paths.trap_door.as_str(),
                format!("tdor{frame_num:04}.pcx"),
            ))
        }
        "TRAP_DOOR3" => {
            let frame_num = match state_name {
                "stopped" => 1,
                "blocking" => 7,
                "running" => (frame % 7) + 1,
                "walking" => 7usize.saturating_sub(frame % 7),
                _ => 1,
            };
            Some((
                sprite_paths.trap_door.as_str(),
                format!("cdor{frame_num:04}.pcx"),
            ))
        }
        "TELE2" => {
            if state_name == "running" {
                let frame_num = ((frame % 15) + 1) as u32;
                Some((
                    sprite_paths.teleport.as_str(),
                    format!("elec{frame_num:04}.pcx"),
                ))
            } else {
                Some((sprite_paths.teleport.as_str(), "close".to_string()))
            }
        }
        "STEP" => {
            if state_name == "stopped" {
                Some((sprite_paths.step.as_str(), "step".to_string()))
            } else {
                Some((sprite_paths.step.as_str(), "step_gone".to_string()))
            }
        }
        "SWITCH" | "SWITCH_ONCE" | "SWITCH_DELAY" => {
            let frame_num = ((frame % 18) + 1) as u32;
            Some((
                sprite_paths.misc.as_str(),
                format!("swit{frame_num:04}.pcx"),
            ))
        }
        "SWITCH_BALL" => {
            let frame_num = if state_name == "running" {
                10 + (frame % 9)
            } else {
                1 + (frame % 9)
            };
            Some((
                sprite_paths.misc.as_str(),
                format!("swit{frame_num:04}.pcx"),
            ))
        }
        _ => None,
    }
}
