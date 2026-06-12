use std::path::PathBuf;

// Resolve a usable Node.js toolchain for GUI-launched processes, whose PATH
// typically misses version-manager shims (volta, nvm, fnm, homebrew).
pub fn node_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    if let Some(home) = home_dir() {
        dirs.push(home.join(".volta").join("bin"));
        dirs.push(home.join(".local").join("bin"));
        if let Some(nvm_bin) =
            latest_versioned_bin(&home.join(".nvm").join("versions").join("node"), "bin")
        {
            dirs.push(nvm_bin);
        }
        if let Some(fnm_bin) = latest_versioned_bin(
            &home
                .join("Library")
                .join("Application Support")
                .join("fnm")
                .join("node-versions"),
            "installation/bin",
        ) {
            dirs.push(fnm_bin);
        }
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.retain(|dir| dir.as_os_str().len() > 1);
    dirs.dedup();
    dirs
}

fn latest_versioned_bin(versions_root: &PathBuf, bin_suffix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(versions_root).ok()?;
    let mut versions: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    let latest = versions.pop()?;
    let bin = bin_suffix
        .split('/')
        .fold(latest, |acc, part| acc.join(part));
    bin.is_dir().then_some(bin)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn real_node_path(node: &PathBuf) -> Option<PathBuf> {
    let mut command = std::process::Command::new(node);
    command.args(["-p", "process.execPath"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    path.is_file().then_some(path)
}

pub struct NodeToolchain {
    pub node: PathBuf,
    pub npx: PathBuf,
    /// PATH value with the node directory prepended, for spawned children.
    pub path_env: std::ffi::OsString,
}

pub fn find_node() -> Result<NodeToolchain, String> {
    let dirs = node_search_dirs();
    let node = dirs
        .iter()
        .find_map(|dir| executable_in_dir(dir, "node"))
        .ok_or_else(|| {
            "Node.js was not found on this machine. Install it from https://nodejs.org and try again.".to_string()
        })?;
    // Version-manager shims (volta, fnm) wrap the real binary in a child
    // process, which would survive `Child::kill` on the dev server. Resolve
    // to the real executable so the spawned process tree is just one node.
    let node = real_node_path(&node).unwrap_or(node);
    let node_dir = node
        .parent()
        .expect("executable has a parent")
        .to_path_buf();
    let npx = executable_in_dir(&node_dir, "npx")
        .or_else(|| dirs.iter().find_map(|dir| executable_in_dir(dir, "npx")))
        .ok_or_else(|| {
            "npx was not found next to node. Reinstall Node.js to get npm/npx.".to_string()
        })?;

    let mut paths: Vec<PathBuf> = vec![node_dir];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let path_env = std::env::join_paths(paths).map_err(|err| err.to_string())?;
    Ok(NodeToolchain {
        node,
        npx,
        path_env,
    })
}

#[cfg(windows)]
pub fn executable_in_dir(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    ["exe", "cmd", "bat", "com"].iter().find_map(|ext| {
        let candidate = dir.join(format!("{name}.{ext}"));
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(not(windows))]
pub fn executable_in_dir(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let candidate = dir.join(name);
    match candidate.metadata() {
        Ok(meta) if meta.is_file() && meta.permissions().mode() & 0o111 != 0 => Some(candidate),
        _ => None,
    }
}
