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
    bridge_rs: String,
    cpp: String,
}

fn main() {
    let crate_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let manifest = run_codegen(&crate_root);
    compile_bridges(&manifest, &crate_root);
    if needs_cpp_build(&crate_root) {
        compile_core_static(&crate_root);
        write_input_stamp(&crate_root);
    }
}

fn inputs_changed(crate_root: &Path) -> bool {
    let stamp = crate_root.join("target/.cruspy_input_hash");
    fs::read_to_string(&stamp).ok().as_deref() != Some(&hash_inputs(crate_root))
}

fn needs_cpp_build(crate_root: &Path) -> bool {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let lib_path = PathBuf::from(out_dir).join("libcruspy-cpp.a");
    inputs_changed(crate_root) || !lib_path.exists()
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
    let easybind_src = crate_root.join("../../packages/easybind/src");

    let mut command = if let Some(cli) = resolve_codegen_cli(crate_root) {
        Command::new(cli)
    } else {
        let mut cmd = Command::new(&python);
        if easybind_src.is_dir() {
            cmd.env("PYTHONPATH", &easybind_src);
        }
        cmd.arg("-m").arg("pymergetic.easybind.rust_codegen");
        cmd
    };
    let status = command
        .args([
            "--source",
            "src/pymergetic/cruspy",
            "--namespace",
            "pymergetic::cruspy",
            "--output",
            "src/pymergetic/cruspy",
        ])
        .current_dir(crate_root)
        .status()
        .expect("run easybind-rust-module");
    if !status.success() {
        panic!("easybind-rust-module failed");
    }

    println!("cargo:rerun-if-changed=src/pymergetic/cruspy");
    println!("cargo:rerun-if-changed=src/pymergetic/cruspy/manifest.json");

    let manifest_path = crate_root.join("src/pymergetic/cruspy/manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest.json");
    serde_json::from_str(&manifest_text).expect("parse manifest.json")
}

fn compile_core_static(crate_root: &Path) {
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let version_define = format!("\"{version}\"");
    let reflect_include = ensure_reflect_cpp(crate_root);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let cxx_include = out_dir.join("cxxbridge/include");

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .define("CRUSPY_PKG_VERSION", version_define.as_str())
        .include(reflect_include)
        .include(&cxx_include)
        .file(crate_root.join("src/pymergetic/cruspy/core/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/core/registry.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/runtime/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/schema/codec.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/schema/schema_base.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/schema/field_base.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/schema/model_base.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/schema/substrate_api.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/domain_backend.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/process_arena_backend.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/file_map_backend.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/domain_registry.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/allocator/substrate_api.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/shm/mod.cpp"))
        .file(crate_root.join("src/pymergetic/cruspy/functions/mod.cpp"))
        .include(crate_root.join("src/pymergetic/cruspy"))
        .compile("cruspy-cpp");

    println!("cargo:rerun-if-changed=Cargo.toml");
}

fn compile_bridges(manifest: &Manifest, crate_root: &Path) {
    let reflect_include = ensure_reflect_cpp(crate_root);
    let mut bridge_modules: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for model in &manifest.models {
        bridge_modules
            .entry(model.bridge_rs.clone())
            .or_default()
            .push(model.cpp.clone());
    }
    for (bridge_rs, cpp_files) in bridge_modules {
        let mut builder = cxx_build::bridge(&bridge_rs);
        builder
            .std("c++20")
            .include(&reflect_include)
            .include(crate_root.join("src/pymergetic/cruspy"));
        for cpp in cpp_files {
            builder.file(crate_root.join(&cpp));
            println!("cargo:rerun-if-changed={}", cpp);
        }
        let lib_name = format!(
            "cruspy-bridge-{}",
            bridge_rs
                .split('/')
                .nth_back(1)
                .unwrap_or("module")
        );
        builder.compile(&lib_name);
        println!("cargo:rerun-if-changed={}", crate_root.join(&bridge_rs).display());
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

fn ensure_reflect_cpp(crate_root: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("REFLECT_CPP_INCLUDE") {
        return PathBuf::from(path);
    }
    let dest = crate_root.join("target/deps/reflect-cpp");
    let include = dest.join("include");
    if !include.join("rfl/Field.hpp").is_file() {
        let _ = fs::remove_dir_all(&dest);
        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                "v0.24.0",
                "https://github.com/getml/reflect-cpp.git",
            ])
            .arg(&dest)
            .status()
            .expect("clone reflect-cpp");
        if !status.success() {
            panic!("failed to clone reflect-cpp v0.24.0");
        }
    }
    println!("cargo:rerun-if-changed={}", dest.join(".git").display());
    include
}
