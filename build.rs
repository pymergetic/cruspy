use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Manifest {
    models: Vec<ManifestModel>,
}

#[derive(Debug, Deserialize)]
struct ManifestModel {
    name: String,
    bridge_rs: String,
    cpp: String,
}

fn main() {
    let manifest = run_codegen();
    compile_cpp();
    compile_bridges(&manifest);
}

fn run_codegen() -> Manifest {
    let crate_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let python = resolve_python(&crate_root);
    let mut command = if let Some(cli) = resolve_codegen_cli(&crate_root) {
        let mut cmd = Command::new(cli);
        cmd
    } else {
        let mut cmd = Command::new(&python);
        cmd.args(["-m", "pymergetic.easybind.rust_codegen.cli"]);
        cmd
    };
    let status = command
        .args([
            "--source",
            "src/pymergetic/cruspy",
            "--namespace",
            "pymergetic::cruspy",
            "--output",
            "generated",
        ])
        .current_dir(&crate_root)
        .status()
        .expect("run easybind-rust-module");
    if !status.success() {
        panic!("easybind-rust-module failed");
    }

    println!("cargo:rerun-if-changed=src/pymergetic/cruspy");
    println!("cargo:rerun-if-changed=generated/manifest.json");
    for entry in glob::glob("src/pymergetic/cruspy/**/*.hpp").expect("glob") {
        if let Ok(path) = entry {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let manifest_path = crate_root.join("generated/manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    serde_json::from_str(&manifest_text).expect("parse manifest.json")
}

fn compile_cpp() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let version_define = format!("\"{version}\"");
    println!("cargo:rerun-if-changed=Cargo.toml");

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .define("CRUSPY_PKG_VERSION", version_define.as_str())
        .file("src/pymergetic/cruspy/runtime/mod.cpp")
        .file("src/pymergetic/cruspy/allocator/mod.cpp")
        .include("src/pymergetic/cruspy")
        .compile("cruspy-cpp");
}

fn compile_bridges(manifest: &Manifest) {
    for model in &manifest.models {
        let bridge_path = Path::new(&model.bridge_rs);
        let mut builder = cxx_build::bridge(bridge_path);
        builder
            .std("c++20")
            .file(&model.cpp)
            .include("src/pymergetic/cruspy");
        let lib_name = format!("cruspy-bridge-{}", model.name.to_lowercase());
        builder.compile(&lib_name);
        println!("cargo:rerun-if-changed={}", bridge_path.display());
        println!("cargo:rerun-if-changed={}", model.cpp);
    }
}

fn resolve_python(crate_root: &Path) -> String {
    if let Ok(python) = std::env::var("CRUSPY_CODEGEN_PYTHON") {
        return python;
    }
    let monorepo_venv = crate_root.join("../../.venv/bin/python");
    if monorepo_venv.is_file() {
        return monorepo_venv.to_string_lossy().into_owned();
    }
    "python3".to_string()
}

fn resolve_codegen_cli(crate_root: &Path) -> Option<String> {
    if let Ok(cli) = std::env::var("EASYBIND_RUST_MODULE") {
        return Some(cli);
    }
    let monorepo_cli = crate_root.join("../../.venv/bin/easybind-rust-module");
    if monorepo_cli.is_file() {
        return Some(monorepo_cli.to_string_lossy().into_owned());
    }
    None
}
