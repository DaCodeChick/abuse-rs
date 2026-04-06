/// Baseline validation tests for level parsing.
///
/// These tests compare the current level parser output against stable baseline
/// dumps generated from known-good legacy level files. This ensures that parser
/// changes don't inadvertently break compatibility with the original Abuse data.
///
/// Baselines are stored as JSON files in `tests/baselines/levels/` and are
/// regenerated when the level format or parser undergoes intentional changes.
use std::env;
use std::fs;
use std::path::PathBuf;

use abuse_runtime::data::level::{LevelData, ObjectVar};
use serde::Serialize;
use serde_json::Value;

/// Serializable level dump structure matching the tools crate format.
#[derive(Debug, Serialize)]
struct LevelDump {
    name: String,
    first_name: Option<String>,
    asset_paths: AssetPathsDump,
    foreground: MapDump,
    background: MapDump,
    bg_scroll_rate: BgScrollRate,
    objects: ObjectsDump,
    lights: LightsDump,
    links: LinksDump,
}

#[derive(Debug, Serialize)]
struct MapDump {
    width: u32,
    height: u32,
    tile_count: usize,
    tile_sample: Vec<u16>,
}

#[derive(Debug, Serialize)]
struct BgScrollRate {
    xmul: u32,
    xdiv: u32,
    ymul: u32,
    ydiv: u32,
}

#[derive(Debug, Serialize)]
struct ObjectsDump {
    object_count: Option<u32>,
    objects_loaded: usize,
    type_names_count: usize,
    state_names_count: usize,
    object_sample: Vec<ObjectDump>,
}

#[derive(Debug, Serialize)]
struct ObjectDump {
    type_id: u16,
    state_id: u16,
    type_name: Option<String>,
    state_name: Option<String>,
    x: i32,
    y: i32,
    hp: i32,
    lvars_count: usize,
}

#[derive(Debug, Serialize)]
struct LightsDump {
    min_light_level: Option<u32>,
    lights_count: usize,
    light_sample: Vec<LightDump>,
}

#[derive(Debug, Serialize)]
struct LightDump {
    light_type: u8,
    x: i32,
    y: i32,
    xshift: i32,
    yshift: i32,
    inner_radius: i32,
    outer_radius: i32,
}

#[derive(Debug, Serialize)]
struct LinksDump {
    object_links_count: usize,
    light_links_count: usize,
    object_link_sample: Vec<ObjectLinkDump>,
    light_link_sample: Vec<LightLinkDump>,
}

#[derive(Debug, Serialize)]
struct ObjectLinkDump {
    from_object: i32,
    to_object: i32,
}

#[derive(Debug, Serialize)]
struct LightLinkDump {
    from_object: i32,
    to_light: i32,
}

#[derive(Debug, Serialize)]
struct AssetPathsDump {
    palette_spe: String,
    fg_tile_spes: Vec<String>,
    bg_tile_spes: Vec<String>,
    object_spes: Vec<String>,
    object_sprite_spes: ObjectSpriteSpeMapDump,
    audio_sfx: AudioSfxDump,
}

#[derive(Debug, Serialize)]
struct ObjectSpriteSpeMapDump {
    door: String,
    chars_door: String,
    misc: String,
    teleport: String,
    lava: String,
    ball: String,
    compass: String,
    rob2: String,
    lightin: String,
    trap_door: String,
    step: String,
}

#[derive(Debug, Serialize)]
struct AudioSfxDump {
    tp_door: String,
    tele2: String,
    spring: String,
    lava: String,
    force_field: String,
}

fn default_asset_paths_dump() -> AssetPathsDump {
    AssetPathsDump {
        palette_spe: "art/back/backgrnd.spe".to_string(),
        fg_tile_spes: vec![
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
        bg_tile_spes: vec![
            "art/back/backgrnd.spe".to_string(),
            "art/back/intro.spe".to_string(),
            "art/back/city.spe".to_string(),
            "art/back/cave.spe".to_string(),
            "art/back/tech.spe".to_string(),
            "art/back/alienb.spe".to_string(),
            "art/back/green2.spe".to_string(),
            "art/back/galien.spe".to_string(),
        ],
        object_spes: vec![
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
        object_sprite_spes: ObjectSpriteSpeMapDump {
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
        audio_sfx: AudioSfxDump {
            tp_door: "sfx/telept01.wav".to_string(),
            tele2: "sfx/fadeon01.wav".to_string(),
            spring: "sfx/spring03.wav".to_string(),
            lava: "sfx/lava01.wav".to_string(),
            force_field: "sfx/force01.wav".to_string(),
        },
    }
}

fn create_level_dump(level: &LevelData) -> LevelDump {
    const SAMPLE_SIZE: usize = 10;

    let object_sample = level
        .objects
        .iter()
        .take(SAMPLE_SIZE)
        .map(|obj| ObjectDump {
            type_id: obj.type_id,
            state_id: obj.state_id,
            type_name: obj.type_name.clone(),
            state_name: obj.state_name.clone(),
            x: obj.var(ObjectVar::X).unwrap_or(0),
            y: obj.var(ObjectVar::Y).unwrap_or(0),
            hp: obj.var(ObjectVar::Hp).unwrap_or(0),
            lvars_count: obj.lvars.len(),
        })
        .collect();

    let light_sample = level
        .lights
        .iter()
        .take(SAMPLE_SIZE)
        .map(|light| LightDump {
            light_type: light.light_type,
            x: light.x,
            y: light.y,
            xshift: light.xshift,
            yshift: light.yshift,
            inner_radius: light.inner_radius,
            outer_radius: light.outer_radius,
        })
        .collect();

    let object_link_sample = level
        .object_links
        .iter()
        .take(SAMPLE_SIZE)
        .map(|link| ObjectLinkDump {
            from_object: link.from_object,
            to_object: link.to_object,
        })
        .collect();

    let light_link_sample = level
        .light_links
        .iter()
        .take(SAMPLE_SIZE)
        .map(|link| LightLinkDump {
            from_object: link.from_object,
            to_light: link.to_light,
        })
        .collect();

    LevelDump {
        name: level.name.clone(),
        first_name: level.first_name.clone(),
        asset_paths: default_asset_paths_dump(),
        foreground: MapDump {
            width: level.fg_width,
            height: level.fg_height,
            tile_count: level.fg_tiles.len(),
            tile_sample: level.fg_tiles.iter().take(SAMPLE_SIZE).copied().collect(),
        },
        background: MapDump {
            width: level.bg_width,
            height: level.bg_height,
            tile_count: level.bg_tiles.len(),
            tile_sample: level.bg_tiles.iter().take(SAMPLE_SIZE).copied().collect(),
        },
        bg_scroll_rate: BgScrollRate {
            xmul: level.bg_xmul,
            xdiv: level.bg_xdiv,
            ymul: level.bg_ymul,
            ydiv: level.bg_ydiv,
        },
        objects: ObjectsDump {
            object_count: level.object_count,
            objects_loaded: level.objects.len(),
            type_names_count: level.object_type_names.len(),
            state_names_count: level.object_state_names.len(),
            object_sample,
        },
        lights: LightsDump {
            min_light_level: level.min_light_level,
            lights_count: level.lights.len(),
            light_sample,
        },
        links: LinksDump {
            object_links_count: level.object_links.len(),
            light_links_count: level.light_links.len(),
            object_link_sample,
            light_link_sample,
        },
    }
}

fn get_legacy_root() -> Option<PathBuf> {
    env::var("ABUSE_LEGACY_ROOT").ok().map(PathBuf::from)
}

fn get_baseline_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/baselines/levels")
}

fn validate_level_against_baseline(level_name: &str) {
    let Some(legacy_root) = get_legacy_root() else {
        eprintln!(
            "ABUSE_LEGACY_ROOT not set; skipping baseline validation for {}",
            level_name
        );
        return;
    };

    let level_path = legacy_root.join(format!("data/levels/{}.spe", level_name));
    if !level_path.exists() {
        panic!("Legacy level file not found: {}", level_path.display());
    }

    let baseline_path = get_baseline_dir().join(format!("{}.json", level_name));
    if !baseline_path.exists() {
        panic!(
            "Baseline file not found: {}. Run `cargo run -p abuse-tools -- level-dump {} --format json > {}`",
            baseline_path.display(),
            level_path.display(),
            baseline_path.display()
        );
    }

    let level = LevelData::open(&level_path)
        .unwrap_or_else(|err| panic!("Failed to parse level {}: {}", level_name, err));

    let dump = create_level_dump(&level);
    let current_json = serde_json::to_value(&dump).unwrap();

    let baseline_json: Value = serde_json::from_str(
        &fs::read_to_string(&baseline_path)
            .unwrap_or_else(|err| panic!("Failed to read baseline {}: {}", level_name, err)),
    )
    .unwrap_or_else(|err| panic!("Failed to parse baseline JSON {}: {}", level_name, err));

    // Compare everything except the "name" field, which contains the full path
    let mut current = current_json.clone();
    let mut baseline = baseline_json.clone();

    if let Some(obj) = current.as_object_mut() {
        obj.remove("name");
    }
    if let Some(obj) = baseline.as_object_mut() {
        obj.remove("name");
    }

    if current != baseline {
        panic!(
            "Baseline mismatch for level {}!\n\nExpected:\n{}\n\nGot:\n{}\n",
            level_name,
            serde_json::to_string_pretty(&baseline).unwrap(),
            serde_json::to_string_pretty(&current).unwrap()
        );
    }
}

macro_rules! baseline_test {
    ($name:ident, $level:expr) => {
        #[test]
        fn $name() {
            validate_level_against_baseline($level);
        }
    };
}

// Main campaign levels
baseline_test!(baseline_level00, "level00");
baseline_test!(baseline_level01, "level01");
baseline_test!(baseline_level02, "level02");
baseline_test!(baseline_level03, "level03");
baseline_test!(baseline_level04, "level04");
baseline_test!(baseline_level05, "level05");
baseline_test!(baseline_level06, "level06");
baseline_test!(baseline_level07, "level07");
baseline_test!(baseline_level08, "level08");
baseline_test!(baseline_level09, "level09");
baseline_test!(baseline_level10, "level10");
baseline_test!(baseline_level11, "level11");
baseline_test!(baseline_level12, "level12");
baseline_test!(baseline_level13, "level13");
baseline_test!(baseline_level14, "level14");
baseline_test!(baseline_level15, "level15");
baseline_test!(baseline_level16, "level16");
baseline_test!(baseline_level17, "level17");
baseline_test!(baseline_level18, "level18");
baseline_test!(baseline_level19, "level19");
baseline_test!(baseline_level20, "level20");
baseline_test!(baseline_level21, "level21");

// Frabs addon levels
baseline_test!(baseline_frabs00, "frabs00");
baseline_test!(baseline_frabs01, "frabs01");
baseline_test!(baseline_frabs02, "frabs02");
baseline_test!(baseline_frabs03, "frabs03");
baseline_test!(baseline_frabs04, "frabs04");
baseline_test!(baseline_frabs05, "frabs05");
baseline_test!(baseline_frabs06, "frabs06");
baseline_test!(baseline_frabs07, "frabs07");
baseline_test!(baseline_frabs08, "frabs08");
baseline_test!(baseline_frabs09, "frabs09");
baseline_test!(baseline_frabs10, "frabs10");
baseline_test!(baseline_frabs11, "frabs11");
baseline_test!(baseline_frabs12, "frabs12");
baseline_test!(baseline_frabs13, "frabs13");
baseline_test!(baseline_frabs14, "frabs14");
baseline_test!(baseline_frabs15, "frabs15");
baseline_test!(baseline_frabs17, "frabs17");
baseline_test!(baseline_frabs18, "frabs18");
baseline_test!(baseline_frabs19, "frabs19");
baseline_test!(baseline_frabs20, "frabs20");
baseline_test!(baseline_frabs21, "frabs21");
baseline_test!(baseline_frabs30, "frabs30");
baseline_test!(baseline_frabs70, "frabs70");
baseline_test!(baseline_frabs71, "frabs71");
baseline_test!(baseline_frabs72, "frabs72");
baseline_test!(baseline_frabs73, "frabs73");
baseline_test!(baseline_frabs74, "frabs74");
