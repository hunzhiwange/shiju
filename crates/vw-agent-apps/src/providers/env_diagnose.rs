//! macOS environment diagnostics. No shell execution, ambient HOME or value disclosure.
use std::path::Path;
use vw_api_types::{agent_apps::AppId, app_providers::env_diagnose::EnvDiagnosis};

pub fn diagnose(home: &Path, app: AppId) -> Result<EnvDiagnosis, &'static str> {
    #[cfg(target_os = "macos")]
    {
        // Values are discarded at this boundary and never enter the diagnostic model.
        let inherited = std::env::vars_os().filter_map(|(key, _)| key.into_string().ok());
        macos::diagnose_at(home, Path::new("/private/etc"), app, inherited)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (home, app);
        Err("unsupported environment platform")
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(super) use macos::diagnose_at;

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::{
        fs::{File, OpenOptions},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    use vw_api_types::app_providers::env_diagnose::*;

    const MAX_SOURCE: u64 = 1024 * 1024;
    const MAX_ENTRIES: usize = 4096;
    const UNAVAILABLE: &str = "environment source unavailable";

    fn relevant(app: AppId, name: &str) -> bool {
        let upper = name.to_ascii_uppercase();
        match app {
            AppId::Claude => upper.starts_with("ANTHROPIC"),
            AppId::Codex => upper.starts_with("OPENAI"),
            AppId::Gemini => upper.starts_with("GEMINI") || upper.starts_with("GOOGLE_GEMINI"),
            AppId::GrokBuild => matches!(upper.as_str(), "XAI_API_KEY" | "GROK_DEFAULT_MODEL"),
            _ => false,
        }
    }
    fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 128
            && (name.as_bytes()[0].is_ascii_alphabetic() || name.starts_with('_'))
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }
    fn assignment(line: &str) -> Option<(&str, &str)> {
        let line = line.trim();
        let line = line
            .strip_prefix("export ")
            .or_else(|| line.strip_prefix("export\t"))
            .unwrap_or(line)
            .trim_start();
        let (name, value) = line.split_once('=')?;
        // Spaces around '=' are not shell assignment syntax.
        valid_name(name).then_some((name, value))
    }
    /// Deliberately narrow: a single literal assignment, with an optional trailing comment.
    /// Complex files remain observable but cannot offer cleanup based on a line guess.
    fn literal(value: &str) -> bool {
        let mut quote = None;
        let mut whitespace = false;
        for ch in value.chars() {
            if ch.is_control() && ch != '\t' {
                return false;
            }
            if quote == Some('\'') {
                if ch == '\'' {
                    quote = None;
                }
                continue;
            }
            if matches!(ch, '$' | '`' | '\\') {
                return false;
            }
            if quote == Some('"') {
                if ch == '"' {
                    quote = None;
                }
                continue;
            }
            if whitespace {
                if ch == '#' {
                    return true;
                }
                if ch.is_whitespace() {
                    continue;
                }
                return false;
            }
            match ch {
                '\'' | '"' => quote = Some(ch),
                ' ' | '\t' => whitespace = true,
                ';' | '&' | '|' | '<' | '>' | '(' | ')' | '{' | '}' => return false,
                _ => {}
            }
        }
        quote.is_none()
    }
    fn open_root(root: &Path) -> Result<File, &'static str> {
        if !root.is_absolute() {
            return Err(UNAVAILABLE);
        }
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)
            .map_err(|_| UNAVAILABLE)
    }
    fn read_source(root: &File, name: &str) -> Result<Option<String>, &'static str> {
        let name = std::ffi::CString::new(name).map_err(|_| UNAVAILABLE)?;
        // SAFETY: the trusted root is live, and name is a fixed single component.
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(UNAVAILABLE)
            };
        }
        // SAFETY: openat returned a fresh descriptor, consumed exactly once.
        let file = unsafe { File::from_raw_fd(fd) };
        let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
        if !metadata.is_file() || metadata.len() > MAX_SOURCE {
            return Err(UNAVAILABLE);
        }
        let mut content = String::new();
        file.take(MAX_SOURCE + 1).read_to_string(&mut content).map_err(|_| UNAVAILABLE)?;
        if content.len() as u64 > MAX_SOURCE {
            return Err(UNAVAILABLE);
        }
        Ok(Some(content))
    }
    fn scan(
        content: &str,
        app: AppId,
        source: impl Fn(usize) -> EnvSource,
        user: bool,
        entries: &mut Vec<EnvDiagnostic>,
    ) -> Result<(), &'static str> {
        // Conditions, functions, sourcing, continuations and here-documents require shell
        // evaluation to establish context. Do not offer destructive actions for such files.
        let simple_file = content.lines().all(|line| {
            let trimmed = line.trim();
            trimmed.is_empty()
                || trimmed.starts_with('#')
                || assignment(line).is_some_and(|(_, value)| literal(value))
        });
        for (index, line) in content.lines().enumerate() {
            if let Some((name, _)) = assignment(line)
                && relevant(app, name)
            {
                if entries.len() >= MAX_ENTRIES {
                    return Err(UNAVAILABLE);
                }
                entries.push(EnvDiagnostic {
                    variable: name.into(),
                    source: source(index + 1),
                    cleanup_available: user && simple_file,
                });
            }
        }
        Ok(())
    }
    pub(crate) fn diagnose_at(
        home: &Path,
        system: &Path,
        app: AppId,
        inherited: impl IntoIterator<Item = String>,
    ) -> Result<EnvDiagnosis, &'static str> {
        if !matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild) {
            return Err("unsupported environment application");
        }
        let home = open_root(home)?;
        let system = open_root(system)?;
        let mut entries = Vec::new();
        for file in [
            UserShellFile::Bashrc,
            UserShellFile::BashProfile,
            UserShellFile::Zshrc,
            UserShellFile::Zprofile,
            UserShellFile::Profile,
        ] {
            if let Some(content) = read_source(&home, file.filename())? {
                scan(
                    &content,
                    app,
                    |line| EnvSource::UserShell { file, line },
                    true,
                    &mut entries,
                )?;
            }
        }
        for file in [SystemShellFile::Profile, SystemShellFile::Bashrc] {
            if let Some(content) = read_source(&system, file.filename())? {
                scan(
                    &content,
                    app,
                    |line| EnvSource::SystemShell { file, line },
                    false,
                    &mut entries,
                )?;
            }
        }
        let names: std::collections::BTreeSet<_> =
            inherited.into_iter().filter(|name| valid_name(name) && relevant(app, name)).collect();
        for name in names {
            if entries.len() >= MAX_ENTRIES {
                return Err(UNAVAILABLE);
            }
            entries.push(EnvDiagnostic {
                variable: name,
                source: EnvSource::ProcessInherited,
                cleanup_available: false,
            });
        }
        Ok(EnvDiagnosis { app_id: app, entries })
    }
}
