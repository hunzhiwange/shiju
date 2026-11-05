//! POSIX path discovery; inputs belong to the authorized execution host.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, Read, Seek},
    os::unix::{ffi::OsStringExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Host environment snapshot. Never populate this from an untrusted API request.
pub struct PosixEnvironment {
    pub home: PathBuf,
    pub path: OsString,
    pub login_path: Option<OsString>,
    pub variables: BTreeMap<String, OsString>,
}

/// Existing absolute directories only; preserves priority without resolving symlinks.
pub fn build_tool_search_paths(tool: &str, env: &PosixEnvironment) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if path.is_absolute() && path.is_dir() && !paths.contains(&path) {
            paths.push(path);
        }
    };
    let home = &env.home;
    let variable = |key: &str| env.variables.get(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    match tool {
        "kimi" => push(
            variable("KIMI_INSTALL_DIR").unwrap_or_else(|| home.join(".kimi-code")).join("bin"),
        ),
        "grok" => {
            if let Some(path) = variable("GROK_BIN_DIR") {
                push(path);
            }
            push(home.join(".grok/bin"));
        }
        "mcode" => {
            if let Some(path) = variable("MCODE_INSTALL_DIR") {
                push(path.join("bin"));
            }
            push(home.join(".minimax-code/bin"));
        }
        "omp" => push(home.join(".bun/bin")),
        _ => {}
    }
    for relative in [
        ".local/bin",
        ".npm-global/bin",
        "n/bin",
        ".volta/bin",
        ".local/share/mise/shims",
        "Library/pnpm",
        ".local/share/pnpm",
    ] {
        push(home.join(relative));
    }
    for key in ["PNPM_HOME", "NPM_CONFIG_PREFIX"] {
        if let Some(path) = variable(key) {
            push(if key == "NPM_CONFIG_PREFIX" { path.join("bin") } else { path });
        }
    }
    for path in
        ["/opt/homebrew/bin", "/home/linuxbrew/.linuxbrew/bin", "/usr/local/bin", "/usr/bin"]
    {
        push(PathBuf::from(path));
    }
    for relative in
        [".local/share/mise/installs/node", ".local/state/fnm_multishells", ".nvm/versions/node"]
    {
        for entry in child_directories(&home.join(relative)) {
            push(entry.join("bin"));
        }
    }
    if tool == "hermes" {
        for entry in child_directories(&home.join("Library/Python")) {
            push(entry.join("bin"));
        }
    }
    if tool == "opencode" {
        for key in ["OPENCODE_INSTALL_DIR", "XDG_BIN_DIR"] {
            if let Some(path) = variable(key) {
                push(path);
            }
        }
        for relative in ["bin", ".opencode/bin", ".bun/bin", "go/bin"] {
            push(home.join(relative));
        }
        if let Some(value) = env.variables.get("GOPATH") {
            for path in std::env::split_paths(value).filter(|p| !p.as_os_str().is_empty()) {
                push(path.join("bin"));
            }
        }
    }
    for value in env.login_path.iter().chain(std::iter::once(&env.path)) {
        for path in std::env::split_paths(value) {
            push(path);
        }
    }
    paths
}

fn child_directories(parent: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    paths.sort();
    paths
}

/// Merge POSIX PATH values, discarding empty entries that would search cwd.
pub fn merge_path_segments(primary: &str, extra: &str) -> String {
    let mut merged = Vec::new();
    for segment in primary.split(':').chain(extra.split(':')) {
        if !segment.is_empty() && !merged.contains(&segment) {
            merged.push(segment);
        }
    }
    merged.join(":")
}

/// Query a trusted host shell, with bounded waiting and process-group cleanup.
/// Captures PATH alone, never the full environment or shell stderr.
pub fn login_shell_path(shell: &Path, timeout: Duration) -> io::Result<OsString> {
    if !shell.is_absolute() || timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute host shell and nonzero timeout required",
        ));
    }
    let flag = match shell.file_name().and_then(|s| s.to_str()) {
        Some("sh" | "dash") => "-c",
        Some("fish") => "-lc",
        _ => "-lic",
    };
    let mut output = tempfile::tempfile()?;
    let mut child = Command::new(shell)
        .args([flag, "/usr/bin/printenv PATH"])
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    let started = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(io::Error::other("login shell failed"))
                };
            }
            Err(error) => break Err(error),
            Ok(None) if started.elapsed() >= timeout => {
                break Err(io::Error::new(io::ErrorKind::TimedOut, "login shell timed out"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    // SAFETY: child owns a fresh process group, created above with process_group(0).
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
    result?;
    output.rewind()?;
    let mut bytes = Vec::new();
    output.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(io::Error::other("login PATH output too large"));
    }
    let path = bytes
        .split(|b| *b == b'\n')
        .rev()
        .find(|line| line.starts_with(b"/"))
        .ok_or_else(|| io::Error::other("login shell returned no absolute PATH"))?;
    Ok(OsString::from_vec(path.to_vec()))
}
