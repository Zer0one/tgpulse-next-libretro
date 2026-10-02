//! Select the embedded catalogue from the single upstream source at build time.
use std::{env, fs, path::PathBuf};

fn main() {
    const SOURCE: &str = "src/roms_db.dat";
    println!("cargo:rerun-if-changed={SOURCE}");
    let model1 = env::var_os("CARGO_FEATURE_MODEL1").is_some();
    let model2 = env::var_os("CARGO_FEATURE_MODEL2").is_some();
    let source = fs::read_to_string(SOURCE).expect("read canonical ROM catalogue");
    let mut selected = String::new();
    let mut include = false;
    for line in source.split_inclusive('\n') {
        if line.starts_with("G ") {
            let board = line.split_whitespace().nth(2).expect("catalogue board tag");
            include = match board {
                "m1" => model1,
                "m2o" | "m2a" | "m2b" | "m2c" => model2,
                _ => panic!("unknown ROM catalogue board: {board}"),
            };
        }
        if include {
            selected.push_str(line);
        }
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    fs::write(output.join("roms_db.dat"), selected).expect("write selected ROM catalogue");
}
