use std::io::BufRead;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::toolchain;

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_dir: Option<String>,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Could not resolve the app config directory: {err}"))?;
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let raw = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    std::fs::write(path, raw).map_err(|err| err.to_string())
}

pub fn resolve_project_folder(app: &AppHandle) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("OPEN_SLIDE_PROJECT_DIR") {
        let dir = PathBuf::from(dir);
        if let Ok(dir) = dir.canonicalize() {
            return Some(dir);
        }
    }

    if let Some(stored) = load_settings(app).project_dir {
        let dir = PathBuf::from(stored);
        if dir.is_dir() {
            return Some(dir);
        }
    }

    #[cfg(debug_assertions)]
    {
        let demo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("demo");
        if let Ok(demo) = demo.canonicalize() {
            return Some(demo);
        }
    }

    None
}

pub fn store_project_folder(app: &AppHandle, dir: &Path) -> Result<(), String> {
    let mut settings = load_settings(app);
    settings.project_dir = Some(dir.to_string_lossy().into_owned());
    save_settings(app, &settings)
}

pub fn is_project(dir: &Path) -> bool {
    dir.join("package.json").is_file()
}

fn emit_log(app: &AppHandle, event: &str, line: String) {
    let _ = app.emit(event, line);
}

fn command_for(executable: &Path) -> Command {
    #[cfg(windows)]
    {
        // .cmd/.bat shims (npm-installed CLIs) cannot be spawned directly.
        let is_shim = executable
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
        if is_shim {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(executable);
            return command;
        }
    }
    Command::new(executable)
}

fn run_streaming(
    app: &AppHandle,
    mut command: Command,
    event: &'static str,
    label: &str,
) -> Result<(), String> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start {label}: {err}"))?;

    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        readers.push(std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                emit_log(&app, event, line);
            }
        }));
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        readers.push(std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                emit_log(&app, event, line);
            }
        }));
    }

    let status = child
        .wait()
        .map_err(|err| format!("Failed waiting for {label}: {err}"))?;
    for reader in readers {
        let _ = reader.join();
    }
    if !status.success() {
        return Err(format!(
            "{label} exited with {status}. Check the log output."
        ));
    }
    Ok(())
}

pub fn init_project(app: &AppHandle, dir: &Path) -> Result<(), String> {
    let node = toolchain::find_node()?;
    let mut command = command_for(&node.npx);
    command
        .args(["-y", "@open-slide/cli@latest", "init", ".", "--use-npm"])
        .current_dir(dir)
        .env("PATH", &node.path_env)
        .env("CI", "1");
    run_streaming(app, command, "setup-log", "open-slide init")
}

fn ensure_installed(app: &AppHandle, dir: &Path) -> Result<(), String> {
    if core_bin(dir).is_file() {
        return Ok(());
    }
    let node = toolchain::find_node()?;
    let npm_dir = node.npx.parent().expect("npx has a parent directory");
    let npm = toolchain::executable_in_dir(npm_dir, "npm")
        .ok_or_else(|| "npm was not found next to node.".to_string())?;
    emit_log(
        app,
        "setup-log",
        "Installing project dependencies…".to_string(),
    );
    let mut command = command_for(&npm);
    command
        .arg("install")
        .current_dir(dir)
        .env("PATH", &node.path_env);
    run_streaming(app, command, "setup-log", "npm install")?;
    if core_bin(dir).is_file() {
        Ok(())
    } else {
        Err(
            "@open-slide/core is still missing after install. Is this an open-slide project?"
                .to_string(),
        )
    }
}

fn core_bin(dir: &Path) -> PathBuf {
    dir.join("node_modules")
        .join("@open-slide")
        .join("core")
        .join("bin.js")
}

fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|err| err.to_string())?;
    let port = listener.local_addr().map_err(|err| err.to_string())?.port();
    drop(listener);
    Ok(port)
}

pub struct DevServer(pub Mutex<Option<DevProcess>>);

pub struct DevProcess {
    child: Child,
}

pub fn stop_dev_server(state: &DevServer) {
    if let Ok(mut guard) = state.0.lock() {
        if let Some(mut proc) = guard.take() {
            let _ = proc.child.kill();
            let _ = proc.child.wait();
        }
    }
}

pub fn start_dev_server(app: &AppHandle, dir: &Path) -> Result<u16, String> {
    let state = app.state::<DevServer>();
    stop_dev_server(&state);

    ensure_installed(app, dir)?;
    let node = toolchain::find_node()?;
    let port = free_port()?;

    let mut command = Command::new(&node.node);
    command
        .arg(core_bin(dir))
        .args(["dev", "--port", &port.to_string()])
        .current_dir(dir)
        .env("PATH", &node.path_env)
        .env("OPEN_SLIDE_SKIP_SKILLS_CHECK", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start the dev server: {err}"))?;

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                emit_log(&app, "dev-log", line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                emit_log(&app, "dev-log", line);
            }
        });
    }

    let mut guard = state
        .0
        .lock()
        .map_err(|_| "dev server state poisoned".to_string())?;
    *guard = Some(DevProcess { child });
    Ok(port)
}
