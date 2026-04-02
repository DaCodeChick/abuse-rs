use std::path::PathBuf;

use abuse_data::lisp::LispProgram;

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
        _ => print_usage(),
    }
}

fn print_usage() {
    println!("abuse-rs tools");
    println!("Usage:");
    println!("  abuse-tools lisp-loads <path-to-lisp-file>");
}
