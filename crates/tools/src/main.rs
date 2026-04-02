use std::path::PathBuf;

use abuse_runtime::data::level::{LevelData, VAR_HP, VAR_X, VAR_Y};
use abuse_runtime::data::lisp::LispProgram;
use abuse_runtime::data::spe::SpeDirectory;

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
                            entry.spec_type.as_u8(),
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
                        let x = object.var(VAR_X).unwrap_or(0);
                        let y = object.var(VAR_Y).unwrap_or(0);
                        let hp = object.var(VAR_HP).unwrap_or(0);
                        println!(
                            "  obj[{idx}] type={} state={} x={} y={} hp={} lvars={}",
                            object.type_id,
                            object.state_id,
                            x,
                            y,
                            hp,
                            object.lvars.len()
                        );
                    }

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
        _ => print_usage(),
    }
}

fn print_usage() {
    println!("abuse-rs tools");
    println!("Usage:");
    println!("  abuse-tools lisp-loads <path-to-lisp-file>");
    println!("  abuse-tools spe-list <path-to-spe-file>");
    println!("  abuse-tools level-summary <path-to-level-spe>");
}
