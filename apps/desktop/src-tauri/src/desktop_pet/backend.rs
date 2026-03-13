//! Backend sidecar process management for desktop shell lifecycle.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, Runtime};
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

pub fn ensure_backend_running<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
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

pub fn stop_backend_process<R: Runtime>(app: &AppHandle<R>) {
    let state: tauri::State<BackendProcessState> = app.state();
    let mut child = {
        let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };

    if let Some(child) = child.as_mut() {
        terminate_child_process(child);
    }
}

pub fn clear_backend_local_data<R: Runtime>(app: &AppHandle<R>) -> Result<ClearLocalDataResult, String> {
    if is_backend_ready() && !has_managed_backend_child(app) {
        return Err(
            "检测到独立启动的后端进程。请先手动退出它，再清除本地数据。".to_string(),
        );
    }

    stop_backend_process(app);

    if is_backend_ready() {
        return Err("后端进程仍在运行，无法安全清除本地数据。".to_string());
    }

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

fn build_backend_command<R: Runtime>(app: &AppHandle<R>) -> Result<Command, String> {
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

fn has_managed_backend_child<R: Runtime>(app: &AppHandle<R>) -> bool {
    let state: tauri::State<BackendProcessState> = app.state();
    let guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
    guard.is_some()
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

fn configure_backend_environment<R: Runtime>(
    app: &AppHandle<R>,
    command: &mut Command,
) -> Result<(), String> {
    let backend_dir = resolve_backend_data_dir(app)?;
    std::fs::create_dir_all(&backend_dir)
        .map_err(|error| format!("failed to create backend data dir: {error}"))?;

    command.env("DATABASE_PATH", backend_dir.join("chat.db"));
    command.env("SERVER_PORT", backend_port().to_string());

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

fn resolve_backend_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Ok(path) = std::env::var("EIDOLON_ECHO_TEST_DATA_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

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

fn resolve_backend_binary_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
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

fn resolve_backend_resource_path<R: Runtime>(app: &AppHandle<R>, relative_path: &str) -> Option<PathBuf> {
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
    #[cfg(test)]
    if let Ok(path) = std::env::var("EIDOLON_ECHO_TEST_READY_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).exists();
        }
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), backend_port());
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

fn backend_port() -> u16 {
    #[cfg(test)]
    if let Ok(raw) = std::env::var("EIDOLON_ECHO_TEST_BACKEND_PORT") {
        if let Ok(port) = raw.parse::<u16>() {
            return port;
        }
    }

    BACKEND_PORT
}

#[cfg(test)]
mod tests {
    use super::{
        clear_backend_local_data, ensure_backend_running, reset_directory, stop_backend_process,
        terminate_child_process, BackendProcessState,
    };
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        sync::{Mutex, OnceLock},
        time::{Duration, Instant},
    };
    use tauri::Manager;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        static COUNTER: OnceLock<std::sync::atomic::AtomicUsize> = OnceLock::new();
        let counter = COUNTER.get_or_init(|| std::sync::atomic::AtomicUsize::new(0));
        let id = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()))
    }

    fn write_test_backend_script(dir: &Path) -> PathBuf {
        #[cfg(target_os = "windows")]
        let script_path = dir.join("test-backend.cmd");
        #[cfg(not(target_os = "windows"))]
        let script_path = dir.join("test-backend.py");

        #[cfg(target_os = "windows")]
        let script = r#"@echo off
python -c "import os, time; marker=os.environ.get('EIDOLON_ECHO_TEST_LAUNCH_MARKER'); ready=os.environ.get('EIDOLON_ECHO_TEST_READY_FILE'); \
f=None; \
if marker: f=open(marker,'a',encoding='utf-8'); f.write('started\n'); f.flush(); \
if ready: open(ready,'w',encoding='utf-8').write('ready'); \
try: \
    while True: time.sleep(1) \
except KeyboardInterrupt: pass"
"#;

        #[cfg(not(target_os = "windows"))]
        let script = r#"#!/usr/bin/env python3
import os
import signal
import sys
import time

marker = os.environ.get("EIDOLON_ECHO_TEST_LAUNCH_MARKER")
if marker:
    with open(marker, "a", encoding="utf-8") as fh:
        fh.write("started\n")

ready_file = os.environ.get("EIDOLON_ECHO_TEST_READY_FILE")
if ready_file:
    with open(ready_file, "w", encoding="utf-8") as fh:
        fh.write("ready\n")

def shutdown(_signum, _frame):
    sys.exit(0)

signal.signal(signal.SIGTERM, shutdown)
signal.signal(signal.SIGINT, shutdown)

while True:
    time.sleep(1)
"#;

        fs::create_dir_all(dir).expect("create helper dir");
        let mut file = fs::File::create(&script_path).expect("create helper script");
        file.write_all(script.as_bytes()).expect("write helper script");
        drop(file);

        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&script_path)
                .expect("stat helper script")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions).expect("chmod helper script");
        }

        script_path
    }

    fn mock_shell_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(BackendProcessState::default())
            .build(mock_context(noop_assets()))
            .expect("build mock tauri app")
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

    #[test]
    fn ensure_backend_running_spawns_and_tracks_ready_backend() {
        let _guard = test_guard();
        let port = 33101u16;
        let helper_dir = unique_temp_dir("eidolon-echo-backend-helper");
        let data_dir = unique_temp_dir("eidolon-echo-backend-data");
        let marker = helper_dir.join("launches.log");
        let ready = helper_dir.join("ready.flag");
        let script_path = write_test_backend_script(&helper_dir);

        std::env::set_var("DESKTOP_AI_BACKEND_BIN", &script_path);
        std::env::set_var("EIDOLON_ECHO_TEST_BACKEND_PORT", port.to_string());
        std::env::set_var("EIDOLON_ECHO_TEST_DATA_DIR", &data_dir);
        std::env::set_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER", &marker);
        std::env::set_var("EIDOLON_ECHO_TEST_READY_FILE", &ready);

        let app = mock_shell_app();
        let result = ensure_backend_running(app.handle());
        if result.is_ok() {
            let state: tauri::State<'_, BackendProcessState> = app.state();
            let guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
            assert!(guard.is_some(), "backend child should be tracked after startup");
            drop(guard);
            assert!(marker.exists(), "test backend helper should have started");
        }

        stop_backend_process(app.handle());
        std::env::remove_var("DESKTOP_AI_BACKEND_BIN");
        std::env::remove_var("EIDOLON_ECHO_TEST_BACKEND_PORT");
        std::env::remove_var("EIDOLON_ECHO_TEST_DATA_DIR");
        std::env::remove_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER");
        std::env::remove_var("EIDOLON_ECHO_TEST_READY_FILE");
        let _ = fs::remove_dir_all(&helper_dir);
        let _ = fs::remove_dir_all(&data_dir);

        result.expect("ensure_backend_running should start helper backend");
    }

    #[test]
    fn clear_backend_local_data_restarts_backend_after_reset() {
        let _guard = test_guard();
        let port = 33102u16;
        let helper_dir = unique_temp_dir("eidolon-echo-clear-helper");
        let data_dir = unique_temp_dir("eidolon-echo-clear-data");
        let marker = helper_dir.join("launches.log");
        let ready = helper_dir.join("ready.flag");
        let script_path = write_test_backend_script(&helper_dir);

        fs::create_dir_all(&data_dir).expect("create data dir");
        fs::write(data_dir.join("stale.txt"), "stale").expect("seed data dir");

        std::env::set_var("DESKTOP_AI_BACKEND_BIN", &script_path);
        std::env::set_var("EIDOLON_ECHO_TEST_BACKEND_PORT", port.to_string());
        std::env::set_var("EIDOLON_ECHO_TEST_DATA_DIR", &data_dir);
        std::env::set_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER", &marker);
        std::env::set_var("EIDOLON_ECHO_TEST_READY_FILE", &ready);

        let app = mock_shell_app();
        let result = clear_backend_local_data(app.handle());

        if let Ok(result) = &result {
            assert_eq!(PathBuf::from(&result.data_dir), data_dir);
            assert!(result.backend_restarted);
            assert!(!data_dir.join("stale.txt").exists(), "data reset should remove stale files");
            let launches = fs::read_to_string(&marker).expect("read launch marker");
            assert!(
                launches.lines().count() >= 1,
                "clearing local data should relaunch helper backend"
            );
        }

        stop_backend_process(app.handle());
        std::env::remove_var("DESKTOP_AI_BACKEND_BIN");
        std::env::remove_var("EIDOLON_ECHO_TEST_BACKEND_PORT");
        std::env::remove_var("EIDOLON_ECHO_TEST_DATA_DIR");
        std::env::remove_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER");
        std::env::remove_var("EIDOLON_ECHO_TEST_READY_FILE");
        let _ = fs::remove_dir_all(&helper_dir);
        let _ = fs::remove_dir_all(&data_dir);

        result.expect("clear_backend_local_data should reset dir and restart backend");
    }

    #[test]
    fn clear_backend_local_data_rejects_unmanaged_running_backend() {
        let _guard = test_guard();
        let helper_dir = unique_temp_dir("eidolon-echo-unmanaged-helper");
        let data_dir = unique_temp_dir("eidolon-echo-unmanaged-data");
        let marker = helper_dir.join("launches.log");
        let ready = helper_dir.join("ready.flag");
        let script_path = write_test_backend_script(&helper_dir);

        std::env::set_var("DESKTOP_AI_BACKEND_BIN", &script_path);
        std::env::set_var("EIDOLON_ECHO_TEST_DATA_DIR", &data_dir);
        std::env::set_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER", &marker);
        std::env::set_var("EIDOLON_ECHO_TEST_READY_FILE", &ready);

        let mut unmanaged = Command::new(&script_path)
            .env("EIDOLON_ECHO_TEST_LAUNCH_MARKER", &marker)
            .env("EIDOLON_ECHO_TEST_READY_FILE", &ready)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn unmanaged backend helper");

        let wait_deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() && Instant::now() < wait_deadline {
            std::thread::sleep(Duration::from_millis(20));
        }

        let app = mock_shell_app();
        let result = clear_backend_local_data(app.handle());

        terminate_child_process(&mut unmanaged);
        std::env::remove_var("DESKTOP_AI_BACKEND_BIN");
        std::env::remove_var("EIDOLON_ECHO_TEST_DATA_DIR");
        std::env::remove_var("EIDOLON_ECHO_TEST_LAUNCH_MARKER");
        std::env::remove_var("EIDOLON_ECHO_TEST_READY_FILE");
        let _ = fs::remove_dir_all(&helper_dir);
        let _ = fs::remove_dir_all(&data_dir);

        let error = result.expect_err("unmanaged backend should block local data reset");
        assert!(
            error.contains("独立启动的后端进程"),
            "unexpected error: {error}"
        );
    }
}
