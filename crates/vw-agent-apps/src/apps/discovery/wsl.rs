//! WSL target rules; no host probing or process execution.
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub enum WslShell {
    Auto { flag: Option<String> },
    Explicit { shell: String, flag: String },
}

#[derive(Debug, PartialEq, Eq)]
pub struct WslTarget {
    pub distro: String,
    pub shell: WslShell,
}

/// Resolve on the actual execution host; never fall back to a local shell.
pub fn resolve_target(
    root: &str,
    shell: Option<&str>,
    flag: Option<&str>,
) -> io::Result<WslTarget> {
    if !cfg!(windows) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "WSL requires a Windows execution host",
        ));
    }
    parse_target(root, shell, flag)
}

/// Parse an authorized Windows host configuration snapshot on any platform.
pub fn parse_target(root: &str, shell: Option<&str>, flag: Option<&str>) -> io::Result<WslTarget> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid WSL configuration root or shell preference",
        )
    };
    let normalized = root.replace('/', "\\");
    let unc = normalized
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| normalized.strip_prefix(r"\\"))
        .ok_or_else(invalid)?;
    let mut parts = unc.split('\\');
    let server = parts.next().ok_or_else(invalid)?;
    if !server.eq_ignore_ascii_case("wsl$") && !server.eq_ignore_ascii_case("wsl.localhost") {
        return Err(invalid());
    }
    let distro = parts.next().ok_or_else(invalid)?;
    if distro.is_empty()
        || distro.len() > 64
        || !distro.as_bytes()[0].is_ascii_alphanumeric()
        || !distro.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        || root.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    if flag.is_some_and(|f| !matches!(f, "-c" | "-lc" | "-lic")) {
        return Err(invalid());
    }
    let shell = match shell {
        None => WslShell::Auto { flag: flag.map(str::to_owned) },
        Some(path) => {
            // Only a bare approved name or a clean absolute POSIX path is accepted.
            if path.contains('/')
                && (!path.starts_with('/')
                    || path.split('/').skip(1).any(|part| {
                        part.is_empty()
                            || part == "."
                            || part == ".."
                            || !part.bytes().all(|b| {
                                b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')
                            })
                    }))
            {
                return Err(invalid());
            }
            let name = path.rsplit('/').next().ok_or_else(invalid)?;
            let default = match name {
                "sh" | "dash" => "-c",
                "fish" => "-lc",
                "bash" | "zsh" => "-lic",
                _ => return Err(invalid()),
            };
            // Execute the approved basename inside the selected distro, never a host path.
            WslShell::Explicit { shell: name.to_owned(), flag: flag.unwrap_or(default).to_owned() }
        }
    };
    Ok(WslTarget { distro: distro.to_owned(), shell })
}
