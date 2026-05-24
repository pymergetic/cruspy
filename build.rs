use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
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
    let crate_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let manifest = run_codegen(&crate_root);
    if inputs_changed(&crate_root) {
        compile_core_static(&crate_root);
        write_input_stamp(&crate_root);
    }
    compile_bridges(&manifest, &crate_root);
}

fn inputs_changed(crate_root: &Path) -> bool {
    let stamp = crate_root.join("target/.cruspy_input_hash");
    fs::read_to_string(&stamp).ok().as_deref() != Some(&hash_inputs(crate_root))
}

fn write_input_stamp(crate_root: &Path) {
    let stamp = crate_root.join("target/.cruspy_input_hash");
    let _ = fs::create_dir_all(crate_root.join("target"));
    let _ = fs::write(stamp, hash_inputs(crate_root));
}

fn hash_inputs(crate_root: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    for pattern in ["src/pymergetic/cruspy/**/*.hpp", "src/pymergetic/cruspy/**/*.cpp"] {
        for entry in glob::glob(&format!("{}/{pattern}", crate_root.display())).expect("glob") {
            if let Ok(path) = entry {
                if let Ok(bytes) = fs::read(&path) {
                    path.display().to_string().hash(&mut hasher);
                    bytes.hash(&mut hasher);
                }
            }
        }
    }
    format!("{:016x}", hasher.finish())
}

fn run_codegen(crate_root: &Path) -> Manifest {
    let python = resolve_python(crate_root);
    let mut command = if let Some(cli) = resolve_codegen_cli(crate_root) {
        Command::new(cli)
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
        .current_dir(crate_root)
        .status()
        .expect("run easybind-rust-module");
    if !status.success() {
        panic!("easybind-rust-module failed");
    }

    println!("cargo:rerun-if-changed=src/pymergetic/cruspy");
    println!("cargo:rerun-if-changed=generated/manifest.json");

    let manifest_path = crate_root.join("generated/manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest.json");
    serde_json::from_str(&manifest_text).expect("parse manifest.json")
}

fn compile_core_static(crate_root: &Path) {
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let version_define = format!("\"{version}\"");

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .define("CRUSPY_PKG_VERSION", version_define.as_str())
        .file(crate_root.join("src/pymergetic/cruspy/core/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/core/registry.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/runtime/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/shm/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/functions/mod.cpp"))
        .include(crate_root.join("src/pymergetic/cruspy"))
        .compile("cruspy-cpp");

    println!("cargo:rerun-if-changed=Cargo.toml");
}

fn compile_bridges(manifest: &Manifest, crate_root: &Path) {
    for model in &manifest.models {
        let bridge_path = crate_root.join(&model.bridge_rs);
        let mut builder = cxx_build::bridge(&bridge_path);
        builder
            .std("c++20")
            .file(crate_root.join(&model.cpp))
            .include(crate_root.join("src/pymergetic/cruspy"));
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
