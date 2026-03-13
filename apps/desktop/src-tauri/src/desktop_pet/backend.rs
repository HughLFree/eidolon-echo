//! Backend sidecar process management for desktop shell lifecycle.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use tracing::info;

const BACKEND_BINARY_BASENAME: &str = "eidolon-echo-backend";
const BACKEND_PACKAGE_NAME: &str = "eidolon-echo-backend";
const BACKEND_PORT: u16 = 3001;
const BACKEND_READY_TIMEOUT: Duration = Duration::from_secs(30);
const BACKEND_READY_POLL: Duration = Duration::from_millis(200);

#[derive(Default)]
pub struct BackendProcessState {
    child: Mutex<Option<Child>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClearLocalDataResult {
    pub data_dir: String,
    pub backend_restarted: bool,
}

pub fn ensure_backend_running(app: &AppHandle) -> Result<(), String> {
    if is_backend_ready() {
        return Ok(());
    }

    let state: tauri::State<BackendProcessState> = app.state();
    {
        let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    *guard = None;
                    return Err(format!("backend process exited early: {status}"));
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(format!("failed to inspect backend process: {error}"));
                }
            }
        }
    }

    if is_backend_ready() {
        return Ok(());
    }

    let mut command = build_backend_command(app)?;
    configure_backend_environment(app, &mut command)?;
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn backend process: {error}"))?;

    wait_backend_ready(&mut child)?;

    let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(child);

    Ok(())
}

pub fn stop_backend_process(app: &AppHandle) {
    let state: tauri::State<BackendProcessState> = app.state();
    let mut child = {
        let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };

    if let Some(child) = child.as_mut() {
        terminate_child_process(child);
    }
}

pub fn clear_backend_local_data(app: &AppHandle) -> Result<ClearLocalDataResult, String> {
    stop_backend_process(app);

    let data_dir = resolve_backend_data_dir(app)?;
    reset_directory(&data_dir)?;
    ensure_backend_running(app)?;

    info!("cleared local backend data at {}", data_dir.display());

    Ok(ClearLocalDataResult {
        data_dir: data_dir.display().to_string(),
        backend_restarted: true,
    })
}

fn wait_backend_ready(child: &mut Child) -> Result<(), String> {
    let start = Instant::now();

    loop {
        if is_backend_ready() {
            return Ok(());
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("backend process exited during startup: {status}"));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(format!("failed to inspect backend process: {error}"));
            }
        }

        if start.elapsed() >= BACKEND_READY_TIMEOUT {
            terminate_child_process(child);
            return Err(format!(
                "backend startup timed out after {}s",
                BACKEND_READY_TIMEOUT.as_secs()
            ));
        }

        std::thread::sleep(BACKEND_READY_POLL);
    }
}

fn build_backend_command(app: &AppHandle) -> Result<Command, String> {
    if let Ok(raw) = std::env::var("DESKTOP_AI_BACKEND_BIN") {
        let path = raw.trim();
        if !path.is_empty() {
            let explicit = PathBuf::from(path);
            if explicit.is_file() {
                return Ok(Command::new(explicit));
            }
            return Err(format!(
                "DESKTOP_AI_BACKEND_BIN points to a missing file: {}",
                explicit.display()
            ));
        }
    }

    if let Some(path) = resolve_backend_binary_path(app) {
        return Ok(Command::new(path));
    }

    if cfg!(debug_assertions) {
        if let Some(workspace_root) = workspace_root_dir() {
            build_debug_backend_binary(&workspace_root)?;

            if let Some(path) = resolve_backend_binary_path(app) {
                return Ok(Command::new(path));
            }
        }
    }

    Err("unable to locate backend sidecar binary".to_string())
}

fn terminate_child_process(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn reset_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove local data dir '{}': {error}", path.display()))?;
    }
    std::fs::create_dir_all(path)
        .map_err(|error| format!("failed to recreate local data dir '{}': {error}", path.display()))?;
    Ok(())
}

fn build_debug_backend_binary(workspace_root: &std::path::Path) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(["build", "-p", BACKEND_PACKAGE_NAME])
        .current_dir(workspace_root)
        .status()
        .map_err(|error| format!("failed to build backend binary in debug mode: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo build -p {BACKEND_PACKAGE_NAME} failed with status: {status}"
        ))
    }
}

fn configure_backend_environment(app: &AppHandle, command: &mut Command) -> Result<(), String> {
    let backend_dir = resolve_backend_data_dir(app)?;
    std::fs::create_dir_all(&backend_dir)
        .map_err(|error| format!("failed to create backend data dir: {error}"))?;

    command.env("DATABASE_PATH", backend_dir.join("chat.db"));

    if let Some(config_path) = resolve_backend_resource_path(app, "config/default.toml") {
        command.env("APP_CONFIG", config_path);
    }

    command.stdin(Stdio::null());
    if cfg!(debug_assertions) {
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }

    Ok(())
}

fn resolve_backend_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Development should use workspace DB for predictable migration/debug behavior.
    if cfg!(debug_assertions) {
        if let Some(workspace_root) = workspace_root_dir() {
            return Ok(workspace_root.join("apps/backend/data"));
        }
    }

    let local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("failed to resolve app local data dir: {error}"))?;
    Ok(local_data_dir.join("backend"))
}

fn resolve_backend_binary_path(app: &AppHandle) -> Option<PathBuf> {
    let file_name = backend_binary_filename();
    let mut candidates = Vec::new();

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join(&file_name));
            candidates.push(parent.join("sidecar").join(&file_name));
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(&file_name));
        candidates.push(resource_dir.join("binaries").join(&file_name));
        if let Some(contents_dir) = resource_dir.parent() {
            candidates.push(contents_dir.join("MacOS").join(&file_name));
        }
    }

    if let Some(workspace_root) = workspace_root_dir() {
        candidates.push(workspace_root.join("target").join("debug").join(&file_name));
        candidates.push(
            workspace_root
                .join("target")
                .join("release")
                .join(&file_name),
        );
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn resolve_backend_resource_path(app: &AppHandle, relative_path: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(relative_path));
    }
    if let Some(workspace_root) = workspace_root_dir() {
        candidates.push(workspace_root.join("apps/backend").join(relative_path));
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn workspace_root_dir() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn backend_binary_filename() -> String {
    if cfg!(target_os = "windows") {
        format!("{BACKEND_BINARY_BASENAME}.exe")
    } else {
        BACKEND_BINARY_BASENAME.to_string()
    }
}

fn is_backend_ready() -> bool {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), BACKEND_PORT);
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{reset_directory, terminate_child_process};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    fn spawn_long_running_child() -> Child {
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "ping", "127.0.0.1", "-n", "30", ">", "NUL"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn windows child")
        } else {
            Command::new("sh")
                .args(["-c", "sleep 30"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn unix child")
        }
    }

    #[test]
    fn terminate_child_process_stops_spawned_backend_like_process() {
        let mut child = spawn_long_running_child();
        terminate_child_process(&mut child);

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
                Ok(None) => panic!("child process still running after terminate_child_process"),
                Err(error) => panic!("failed to inspect child process: {error}"),
            }
        }
    }

    #[test]
    fn reset_directory_removes_existing_files_and_recreates_folder() {
        let base = std::env::temp_dir().join(format!(
            "eidolon-echo-reset-dir-{}",
            std::process::id()
        ));
        let nested = base.join("nested");
        std::fs::create_dir_all(&nested).expect("create temp nested dir");
        std::fs::write(nested.join("chat.db"), "test").expect("write temp db");

        reset_directory(&base).expect("reset temp dir");

        assert!(base.exists());
        assert!(base.is_dir());
        assert!(
            std::fs::read_dir(&base)
                .expect("read reset dir")
                .next()
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
