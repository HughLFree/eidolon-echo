//! Build script for generating Tauri application metadata.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    ensure_sidecar_file();
    tauri_build::build()
}

fn ensure_sidecar_file() {
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(value) => PathBuf::from(value),
        Err(_) => return,
    };
    let target = match env::var("TARGET") {
        Ok(value) => value,
        Err(_) => return,
    };

    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let sidecar_name = format!("desktop-ai-backend-{target}{ext}");
    let sidecar_path = manifest_dir.join("binaries").join(sidecar_name);
    if sidecar_path.is_file() {
        return;
    }

    if let Some(parent) = sidecar_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    if let Some(source) = backend_binary_candidates(&manifest_dir, &target, ext)
        .into_iter()
        .find(|path| path.is_file())
    {
        let _ = fs::copy(source, &sidecar_path);
        make_executable_if_needed(&sidecar_path);
        return;
    }

    if target.contains("windows") {
        let _ = fs::write(&sidecar_path, []);
    } else {
        let placeholder =
            b"#!/bin/sh\necho \"desktop-ai-backend sidecar is not prepared\" >&2\nexit 1\n";
        let _ = fs::write(&sidecar_path, placeholder);
        make_executable_if_needed(&sidecar_path);
    }
}

fn backend_binary_candidates(manifest_dir: &Path, target: &str, ext: &str) -> Vec<PathBuf> {
    let workspace_root = manifest_dir.join("../../..");
    let binary_name = format!("desktop-ai-backend{ext}");
    vec![
        workspace_root
            .join("target")
            .join(target)
            .join("debug")
            .join(&binary_name),
        workspace_root
            .join("target")
            .join(target)
            .join("release")
            .join(&binary_name),
        workspace_root
            .join("target")
            .join("debug")
            .join(&binary_name),
        workspace_root
            .join("target")
            .join("release")
            .join(&binary_name),
    ]
}

fn make_executable_if_needed(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
}
