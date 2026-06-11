use std::path::{Path, PathBuf};

use serde::Serialize;

struct KnownAgent {
    id: &'static str,
    name: &'static str,
    binary: &'static str,
}

const KNOWN_AGENTS: &[KnownAgent] = &[
    KnownAgent { id: "claude", name: "Claude Code", binary: "claude" },
    KnownAgent { id: "codex", name: "Codex CLI", binary: "codex" },
    KnownAgent { id: "gemini", name: "Gemini CLI", binary: "gemini" },
    KnownAgent { id: "copilot", name: "Copilot CLI", binary: "copilot" },
    KnownAgent { id: "cursor-agent", name: "Cursor Agent", binary: "cursor-agent" },
    KnownAgent { id: "opencode", name: "opencode", binary: "opencode" },
    KnownAgent { id: "aider", name: "Aider", binary: "aider" },
];

#[derive(Serialize, Clone)]
pub struct DetectedAgent {
    pub id: String,
    pub name: String,
    pub path: String,
}

pub fn detect() -> Vec<DetectedAgent> {
    let dirs = search_dirs();
    KNOWN_AGENTS
        .iter()
        .filter_map(|agent| {
            dirs.iter()
                .find_map(|dir| executable_in_dir(dir, agent.binary))
                .map(|path| DetectedAgent {
                    id: agent.id.to_string(),
                    name: agent.name.to_string(),
                    path: path.to_string_lossy().into_owned(),
                })
        })
        .collect()
}

pub fn find_agent(id: &str) -> Option<DetectedAgent> {
    detect().into_iter().find(|agent| agent.id == id)
}

// GUI apps (notably on macOS) inherit a minimal PATH, so well-known install
// locations are searched in addition to it.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    if let Some(home) = home_dir() {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".claude").join("local"));
        dirs.push(home.join("bin"));
        dirs.push(home.join(".opencode").join("bin"));
    }
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.retain(|dir| dir.as_os_str().len() > 1);
    dirs.dedup();
    dirs
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(windows)]
fn executable_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    ["exe", "cmd", "bat", "com"].iter().find_map(|ext| {
        let candidate = dir.join(format!("{name}.{ext}"));
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(not(windows))]
fn executable_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let candidate = dir.join(name);
    match candidate.metadata() {
        Ok(meta) if meta.is_file() && meta.permissions().mode() & 0o111 != 0 => Some(candidate),
        _ => None,
    }
}

pub fn project_folder() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("OPEN_SLIDE_PROJECT_DIR") {
        let dir = PathBuf::from(dir);
        return dir
            .canonicalize()
            .map_err(|err| format!("OPEN_SLIDE_PROJECT_DIR is not accessible: {err}"));
    }

    #[cfg(debug_assertions)]
    {
        let demo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("demo");
        if let Ok(demo) = demo.canonicalize() {
            return Ok(demo);
        }
    }

    std::env::current_dir().map_err(|err| format!("Could not resolve a project folder: {err}"))
}

#[cfg(target_os = "macos")]
pub fn launch_in_terminal(agent_path: &Path, dir: &Path) -> Result<(), String> {
    fn sh_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
    fn applescript_quote(s: &str) -> String {
        s.replace('\\', r"\\").replace('"', r#"\""#)
    }

    let shell_command = format!(
        "cd {} && {}",
        sh_quote(&dir.to_string_lossy()),
        sh_quote(&agent_path.to_string_lossy()),
    );
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
        applescript_quote(&shell_command),
    );
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to open Terminal: {err}"))
}

#[cfg(target_os = "windows")]
pub fn launch_in_terminal(agent_path: &Path, dir: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let agent = agent_path.to_string_lossy();
    let dir_s = dir.to_string_lossy();

    if let Some(wt) = search_dirs()
        .iter()
        .find_map(|d| executable_in_dir(d, "wt"))
    {
        let spawned = std::process::Command::new(wt)
            .arg("-d")
            .arg(dir)
            .arg(agent.as_ref())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
        if spawned.is_ok() {
            return Ok(());
        }
    }

    let mut command = std::process::Command::new("cmd");
    command
        .current_dir(dir)
        .creation_flags(CREATE_NO_WINDOW)
        .raw_arg(format!(
            "/C start \"open-slide\" /D \"{dir_s}\" cmd /K \"{agent}\""
        ));
    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to open a terminal: {err}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn launch_in_terminal(agent_path: &Path, dir: &Path) -> Result<(), String> {
    let agent = agent_path.to_string_lossy().into_owned();
    let dir_s = dir.to_string_lossy().into_owned();

    let candidates: Vec<(&str, Vec<String>)> = vec![
        (
            "gnome-terminal",
            vec![format!("--working-directory={dir_s}"), "--".into(), agent.clone()],
        ),
        ("konsole", vec!["--workdir".into(), dir_s.clone(), "-e".into(), agent.clone()]),
        (
            "xfce4-terminal",
            vec![format!("--working-directory={dir_s}"), "-x".into(), agent.clone()],
        ),
        ("kitty", vec!["--directory".into(), dir_s.clone(), agent.clone()]),
        (
            "alacritty",
            vec!["--working-directory".into(), dir_s.clone(), "-e".into(), agent.clone()],
        ),
        ("wezterm", vec!["start".into(), "--cwd".into(), dir_s.clone(), "--".into(), agent.clone()]),
        ("foot", vec![agent.clone()]),
        ("x-terminal-emulator", vec!["-e".into(), agent.clone()]),
        ("xterm", vec!["-e".into(), agent.clone()]),
    ];

    let dirs = search_dirs();
    for (binary, args) in candidates {
        if !dirs.iter().any(|d| executable_in_dir(d, binary).is_some()) {
            continue;
        }
        let spawned = std::process::Command::new(binary)
            .args(&args)
            .current_dir(dir)
            .spawn();
        if spawned.is_ok() {
            return Ok(());
        }
    }

    Err("No supported terminal emulator found. Install one (e.g. gnome-terminal, konsole, kitty, alacritty) or run the agent manually.".to_string())
}
