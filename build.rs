use std::path::Path;

fn main() {
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

    cxx_build::bridge("src/models/document.rs")
        .std("c++20")
        .file("src/pymergetic/cruspy/models/document/mod.cpp")
        .include("src/pymergetic/cruspy")
        .compile("cruspy-bridge");

    println!("cargo:rerun-if-changed=src/pymergetic/cruspy");
    println!("cargo:rerun-if-changed=src/models/document.rs");

    for entry in glob::glob("src/pymergetic/cruspy/**/*.hpp").expect("glob pattern") {
        if let Ok(path) = entry {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let _ = Path::new("src/pymergetic/cruspy/models/document/mod.cpp");
}
