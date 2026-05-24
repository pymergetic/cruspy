use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cruspy_root = manifest_dir.join("src/pymergetic/cruspy");

    println!("cargo:rerun-if-changed=src/pymergetic/cruspy");
    println!("cargo:rerun-if-changed=tools/cruspy-gen");

    cruspy_build::gen_models_glob(&manifest_dir, &cruspy_root, "models/**/*.openapi.yaml")
        .expect("cruspy-gen failed");

    let reflect_cpp = ensure_reflect_cpp(&out_dir);
    println!("cargo:rerun-if-changed={}", reflect_cpp.join("include/rfl/Field.hpp").display());

    let cpp_files = collect_cpp_sources(&cruspy_root);
    for path in &cpp_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    if !cpp_files.is_empty() {
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .std("c++20")
            .flag_if_supported("-Wno-unused-parameter")
            .include(reflect_cpp.join("include"));
        for path in &cpp_files {
            build.file(path);
            if let Some(parent) = path.parent() {
                build.include(parent);
            }
        }
        build.include(&cruspy_root);
        for object in build.compile_intermediates() {
            println!("cargo:rustc-link-arg={}", object.display());
        }
        println!("cargo:rustc-link-arg=-lstdc++");
    }
}

fn ensure_reflect_cpp(out_dir: &Path) -> PathBuf {
    let dest = out_dir.join("reflect-cpp");
    let marker = dest.join("include/rfl/Field.hpp");
    if marker.is_file() {
        return dest;
    }
    if dest.exists() {
        fs::remove_dir_all(&dest).expect("clean partial reflect-cpp checkout");
    }
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            "v0.24.0",
            "https://github.com/getml/reflect-cpp.git",
            dest.to_str().expect("reflect-cpp path utf-8"),
        ])
        .status()
        .expect("run git clone for reflect-cpp");
    assert!(status.success(), "failed to fetch reflect-cpp v0.24.0");
    dest
}

fn collect_cpp_sources(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_cpp(root, &mut files);
    files.sort();
    files
}

fn walk_cpp(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_cpp(&path, out);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("_gen.cpp") || name.ends_with("__init__.cpp") {
                out.push(path);
            } else if name.ends_with(".cpp") && path.to_string_lossy().contains("/models/") {
                out.push(path);
            } else if name == "__init__.cpp" && path.to_string_lossy().contains("/testing/") {
                out.push(path);
            }
        }
    }
}
