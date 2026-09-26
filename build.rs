use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=data/commands.json");
    let json = fs::read("data/commands.json").expect("data/commands.json is missing");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("commands.bin");
    fs::write(out, lz4_flex::compress_prepend_size(&json)).unwrap();
}
