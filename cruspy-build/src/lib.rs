//! Build-time OpenAPI codegen driver for cruspy (EP-0021).

use std::path::Path;
use std::process::Command;

fn python_executable() -> String {
    for key in ["PYO3_PYTHON", "PYTHON_SYS_EXECUTABLE"] {
        if let Ok(path) = std::env::var(key) {
            if !path.is_empty() {
                return path;
            }
        }
    }
    "python3".to_string()
}

/// Run ``cruspy-gen`` for all OpenAPI models under ``cruspy_root``.
pub fn gen_models_glob(manifest_dir: &Path, cruspy_root: &Path, glob: &str) -> Result<(), String> {
    let script = manifest_dir.join("tools/cruspy-gen/cruspy_gen.py");
    if !script.is_file() {
        return Ok(());
    }
    let python = python_executable();
    let output = Command::new(&python)
        .arg(&script)
        .arg("--root")
        .arg(cruspy_root)
        .arg("--glob")
        .arg(glob)
        .output()
        .map_err(|err| format!("failed to run cruspy-gen with {python}: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "cruspy-gen failed (python={python}, exit={:?})\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        ));
    }
    Ok(())
}
