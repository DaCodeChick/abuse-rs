//! Object sprite resolution and placement helpers.

use crate::data::level::{LoadedObject, VAR_CUR_FRAME};

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
pub fn resolve_object_sprite(object: &LoadedObject) -> Option<(&'static str, String)> {
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
