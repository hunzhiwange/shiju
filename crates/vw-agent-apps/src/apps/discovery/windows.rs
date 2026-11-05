//! Windows execution-host path rules. Inputs must come from the authorized host.
use std::path::PathBuf;

/// Merge process, user, machine and package-manager directories in priority order.
pub fn merge_paths(parts: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for part in parts {
        for entry in part.split(';') {
            let entry = entry.trim().trim_matches('"');
            let bytes = entry.as_bytes();
            let absolute = entry.starts_with(r"\\")
                || (bytes.len() >= 3
                    && bytes[0].is_ascii_alphabetic()
                    && bytes[1] == b':'
                    && matches!(bytes[2], b'\\' | b'/'));
            if absolute
                && seen.insert(entry.replace('/', "\\").trim_end_matches('\\').to_ascii_lowercase())
            {
                result.push(entry.to_owned());
            }
        }
    }
    result
}

/// Find a runnable entry without consulting the current working directory.
pub fn find_executable(
    tool: &str,
    directories: &[PathBuf],
    pathext: Option<&str>,
) -> Option<PathBuf> {
    if tool.is_empty()
        || !tool.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return None;
    }
    let extensions: Vec<_> = pathext
        .unwrap_or(".COM;.EXE;.BAT;.CMD")
        .split(';')
        .map(str::trim)
        .filter(|ext| {
            [".com", ".exe", ".bat", ".cmd"].iter().any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
        .collect();
    for directory in directories {
        let normalized = directory.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
        if !directory.is_absolute()
            || normalized.trim_end_matches('/').ends_with("/microsoft/windowsapps")
        {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        let mut files: Vec<_> = entries.filter_map(Result::ok).map(|entry| entry.path()).collect();
        files.sort();
        for extension in &extensions {
            let name = format!("{tool}{extension}");
            if let Some(path) = files.iter().find(|path| {
                path.is_file()
                    && path.file_name().is_some_and(|file| file.eq_ignore_ascii_case(&name))
            }) {
                if tool.eq_ignore_ascii_case("opencode") {
                    for sidecar in [
                        directory.join("opencode-cli.exe"),
                        directory.join("resources/opencode-cli.exe"),
                    ] {
                        if sidecar.is_file() {
                            return Some(sidecar);
                        }
                    }
                    if directory.join("resources/app.asar").is_file() {
                        continue;
                    }
                }
                return Some(path.clone());
            }
        }
    }
    None
}

/// Add conventional package-manager locations after effective PATH entries.
/// Environment values are a host snapshot, never client-supplied execution authority.
pub fn search_directories(parts: &[&str], environment: &[(String, String)]) -> Vec<String> {
    let mut paths = merge_paths(parts);
    let mut extra = Vec::new();
    for (key, value) in environment {
        let suffixes: &[&str] = match key.to_ascii_uppercase().as_str() {
            "APPDATA" => &["npm", "fnm\\aliases\\default"],
            "LOCALAPPDATA" => &["pnpm", "Volta\\bin", "fnm\\aliases\\default"],
            "USERPROFILE" => &["scoop\\shims", ".bun\\bin", ".cargo\\bin", ".local\\bin"],
            "VOLTA_HOME" => &["bin"],
            "FNM_DIR" => &["aliases\\default"],
            "PNPM_HOME" | "NVM_SYMLINK" => &[""],
            _ => continue,
        };
        for suffix in suffixes {
            extra.push(if suffix.is_empty() {
                value.clone()
            } else {
                format!("{}\\{suffix}", value.trim_end_matches(['\\', '/']))
            });
        }
    }
    paths.extend(extra);
    merge_paths(&[&paths.join(";")])
}
