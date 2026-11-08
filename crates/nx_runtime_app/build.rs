use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=scripts/odisea_ecs.nx");
    let src = fs::read_to_string("scripts/odisea_ecs.nx").expect("Falta scripts/odisea_ecs.nx");
    let (rust_code, _wgsl) = nxc_lexer_parser::compile_nx_source(&src).expect("Error compilando .nx");
    let out_dir = env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("generated_nx_game.rs"), rust_code).unwrap();
}
