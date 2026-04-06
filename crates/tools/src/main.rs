use std::path::PathBuf;

use abuse_runtime::data::level::{LevelData, ObjectVar};
use abuse_runtime::data::lisp::LispProgram;
use abuse_runtime::data::spe::SpeDirectory;
use serde::Serialize;

/// Serializable level dump structure for stable machine-comparable output.
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
    /// Sample of first few tiles for validation
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
    /// Sample of first few objects for validation
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
    /// Sample of first few lights for validation
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
    /// Sample of first few object links for validation
    object_link_sample: Vec<ObjectLinkDump>,
    /// Sample of first few light links for validation
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "lisp-loads" => {
            let path = if args.len() >= 3 {
                PathBuf::from(&args[2])
            } else {
                eprintln!("error: missing path to lisp file");
                print_usage();
                std::process::exit(2);
            };

            match LispProgram::load_file(&path) {
                Ok(program) => {
                    println!("Parsed Lisp file: {}", path.display());
                    let loads = program.collect_load_targets();
                    println!("load forms: {}", loads.len());
                    for load in loads {
                        println!("- {load}");
                    }
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    std::process::exit(1);
                }
            }
        }
        "spe-list" => {
            let path = if args.len() >= 3 {
                PathBuf::from(&args[2])
            } else {
                eprintln!("error: missing path to spe file");
                print_usage();
                std::process::exit(2);
            };

            match SpeDirectory::open_lenient(&path) {
                Ok(directory) => {
                    println!("Parsed SPE file: {}", path.display());
                    println!("entries: {}", directory.entries.len());
                    for (idx, entry) in directory.entries.iter().enumerate() {
                        println!(
                            "[{idx:04}] type={:>3} flags={:>3} size={:>8} offset={:>8} name={}",
                            u8::from(entry.spec_type),
                            entry.flags,
                            entry.size,
                            entry.offset,
                            entry.name
                        );
                    }
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    std::process::exit(1);
                }
            }
        }
        "level-summary" => {
            let path = if args.len() >= 3 {
                PathBuf::from(&args[2])
            } else {
                eprintln!("error: missing path to level spe file");
                print_usage();
                std::process::exit(2);
            };

            match LevelData::open(&path) {
                Ok(level) => {
                    println!("Level file: {}", path.display());
                    if let Some(first_name) = &level.first_name {
                        println!("first name: {first_name}");
                    }
                    println!(
                        "fg: {}x{} ({} tiles)",
                        level.fg_width,
                        level.fg_height,
                        level.fg_tiles.len()
                    );
                    println!(
                        "bg: {}x{} ({} tiles)",
                        level.bg_width,
                        level.bg_height,
                        level.bg_tiles.len()
                    );
                    println!(
                        "bg_scroll_rate: x={}/{} y={}/{}",
                        level.bg_xmul, level.bg_xdiv, level.bg_ymul, level.bg_ydiv
                    );
                    match level.object_count {
                        Some(count) => println!("object_count: {count}"),
                        None => println!("object_count: missing"),
                    }

                    println!("objects_loaded: {}", level.objects.len());
                    for (idx, object) in level.objects.iter().take(5).enumerate() {
                        let x = object.var(ObjectVar::X).unwrap_or(0);
                        let y = object.var(ObjectVar::Y).unwrap_or(0);
                        let hp = object.var(ObjectVar::Hp).unwrap_or(0);
                        println!(
                            "  obj[{idx}] type={}({}) state={}({}) x={} y={} hp={} lvars={}",
                            object.type_id,
                            object.type_name.as_deref().unwrap_or("?"),
                            object.state_id,
                            object.state_name.as_deref().unwrap_or("?"),
                            x,
                            y,
                            hp,
                            object.lvars.len()
                        );
                    }

                    println!("object_type_names: {}", level.object_type_names.len());

                    match level.min_light_level {
                        Some(min_level) => println!("min_light_level: {min_level}"),
                        None => println!("min_light_level: missing"),
                    }
                    println!("lights_loaded: {}", level.lights.len());
                    for (idx, light) in level.lights.iter().take(3).enumerate() {
                        println!(
                            "  light[{idx}] type={} x={} y={} inner={} outer={}",
                            light.light_type,
                            light.x,
                            light.y,
                            light.inner_radius,
                            light.outer_radius
                        );
                    }
                    println!("object_links: {}", level.object_links.len());
                    for (idx, link) in level.object_links.iter().take(3).enumerate() {
                        println!(
                            "  obj_link[{idx}] {} -> {}",
                            link.from_object, link.to_object
                        );
                    }
                    println!("light_links: {}", level.light_links.len());
                    for (idx, link) in level.light_links.iter().take(3).enumerate() {
                        println!(
                            "  light_link[{idx}] {} -> {}",
                            link.from_object, link.to_light
                        );
                    }
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    std::process::exit(1);
                }
            }
        }
        "level-dump" => {
            let path = if args.len() >= 3 {
                PathBuf::from(&args[2])
            } else {
                eprintln!("error: missing path to level spe file");
                print_usage();
                std::process::exit(2);
            };

            let format = if args.len() >= 5 && args[3] == "--format" {
                args[4].as_str()
            } else {
                "json"
            };

            match LevelData::open(&path) {
                Ok(level) => {
                    let dump = create_level_dump(&level);
                    match format {
                        "json" => match serde_json::to_string_pretty(&dump) {
                            Ok(json) => println!("{json}"),
                            Err(err) => {
                                eprintln!("error: failed to serialize to JSON: {err}");
                                std::process::exit(1);
                            }
                        },
                        "ron" => match ron::ser::to_string_pretty(&dump, Default::default()) {
                            Ok(ron_str) => println!("{ron_str}"),
                            Err(err) => {
                                eprintln!("error: failed to serialize to RON: {err}");
                                std::process::exit(1);
                            }
                        },
                        _ => {
                            eprintln!("error: unsupported format '{format}' (use 'json' or 'ron')");
                            std::process::exit(2);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("error: {err}");
                    std::process::exit(1);
                }
            }
        }
        _ => print_usage(),
    }
}

fn print_usage() {
    println!("abuse-rs tools");
    println!("Usage:");
    println!("  abuse-tools lisp-loads <path-to-lisp-file>");
    println!("  abuse-tools spe-list <path-to-spe-file>");
    println!("  abuse-tools level-summary <path-to-level-spe>");
    println!("  abuse-tools level-dump <path-to-level-spe> [--format json|ron]");
}

/// Creates a serializable level dump from a LevelData instance.
/// Includes counts and representative samples for validation and comparison.
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
