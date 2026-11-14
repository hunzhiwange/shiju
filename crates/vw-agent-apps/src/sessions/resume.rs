//! Host-side resume material. Never deserialize commands or trust display metadata.
use super::{
    providers::omp::{self, OmpSessionContext},
    source::SourceRegistry,
};
use crate::apps::launch::request::validate_launch_request;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
use vw_api_types::{
    agent_apps::{AppId, AppInstallation, LaunchRequest},
    external_session::{ExternalSessionIdentity, SessionProvider},
};

/// Resolved by the authorized execution host for the selected native instance.
/// `data_dir` is its native configuration directory, not a client override.
pub struct ResumeContext<'a> {
    pub execution_home: &'a Path,
    pub search_paths: &'a [PathBuf],
    pub data_dir: &'a Path,
    pub authorized_project_roots: &'a [PathBuf],
    pub omp: Option<OmpSessionContext<'a>>,
    pub pi_config_dir: Option<&'a Path>,
    pub xdg_state_home: Option<&'a Path>,
    pub xdg_cache_home: Option<&'a Path>,
}

/// Credential-free, non-wire material. Execute directly as argv; terminal adapters
/// must never concatenate these values into an unquoted shell command.
pub struct ResumePlan {
    identity: ExternalSessionIdentity,
    program: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    home: PathBuf,
    search_paths: Vec<PathBuf>,
    env: BTreeMap<&'static str, String>,
}
impl ResumePlan {
    /// Produce manual POSIX shell material without ever resolving account secrets.
    pub fn copy_command(&self) -> Result<String, String> {
        if self.identity.provider == SessionProvider::Agy {
            return Err("credential-dependent resume cannot be copied".into());
        }
        fn quote(value: &str) -> Result<String, String> {
            if value.chars().any(char::is_control) { return Err("invalid copy command value".into()); }
            Ok(format!("'{}'", value.replace('\'', "'\\''")))
        }
        let path = std::env::join_paths(self.search_paths.iter().cloned()
            .chain(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from)))
            .map_err(|_| "invalid execution-host paths")?;
        let path = path.to_str().ok_or("invalid execution-host path encoding")?;
        let mut env = BTreeMap::from([
            ("HOME", path_text(&self.home)?), ("PATH", path.into()),
            ("XDG_CONFIG_HOME", path_text(&self.home.join(".config"))?),
            ("XDG_CACHE_HOME", path_text(&self.home.join(".cache"))?),
            ("XDG_DATA_HOME", path_text(&self.home.join(".local/share"))?),
            ("XDG_STATE_HOME", path_text(&self.home.join(".local/state"))?),
        ]);
        for (key, value) in &self.env {
            if !matches!(*key, "HOME" | "CODEX_HOME" | "CLAUDE_CONFIG_DIR" | "GEMINI_CLI_HOME"
                | "KIMI_CODE_HOME" | "MINIMAX_DATA_DIR" | "OPENCODE_CONFIG_DIR" | "PI_CODING_AGENT_DIR" | "PI_CODING_AGENT_SESSION_DIR"
                | "OMP_PROFILE" | "PI_PROFILE" | "PI_CONFIG_DIR" | "XDG_CONFIG_HOME"
                | "XDG_DATA_HOME" | "XDG_STATE_HOME" | "XDG_CACHE_HOME") {
                return Err("unsupported copy command environment".into());
            }
            env.insert(*key, value.clone());
        }
        let mut command = format!("cd {} && /usr/bin/env -i", quote(&path_text(&self.cwd)?)?);
        for (key, value) in env { command.push_str(&format!(" {}", quote(&format!("{key}={value}"))?)); }
        command.push_str(&format!(" {}", quote(&path_text(&self.program)?)?));
        for arg in &self.args { command.push_str(&format!(" {}", quote(arg)?)); }
        Ok(command)
    }

    pub fn program(&self) -> &Path {
        &self.program
    }
    pub fn args(&self) -> &[String] {
        &self.args
    }
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }
    pub fn environment(&self) -> &BTreeMap<&'static str, String> {
        &self.env
    }

    /// AGY credentials must be resolved from the authorized identity here, on the
    /// execution host. A failed resolver aborts; no key reaches this plan or DTOs.
    /// Callers reauthorize/rebuild immediately before executing under their guard.
    pub fn command_with_credentials(
        &self,
        resolve_agy: impl FnOnce(&ExternalSessionIdentity, &mut Command) -> Result<(), String>,
    ) -> Result<Command, String> {
        let mut command = crate::apps::launch::host::command(
            crate::apps::launch::request::LaunchMaterial {
                program: self.program.clone(),
                cwd: self.cwd.clone(),
                args: self.args.clone(),
                skip_permissions: false,
            },
            &self.home,
            &self.search_paths,
        )?;
        command.envs(&self.env);
        if self.identity.provider == SessionProvider::Agy {
            resolve_agy(&self.identity, &mut command)?;
        }
        Ok(command)
    }
}

/// Registry, installation observation and context are server-owned and must be
/// selected after the gateway grants native-session resume on the target host.
/// No SessionMeta, command, argv, cwd or credential is accepted from the request.
pub fn build_resume_plan(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    installation: &AppInstallation,
    context: &ResumeContext<'_>,
) -> Result<ResumePlan, String> {
    let source = registry.resolve(identity).map_err(|_| "resume source is not authorized")?;
    super::identity::native_resume_identity(&source, identity)
        .map_err(|_| "resume identity is not authorized")?;
    let app = match identity.provider {
        SessionProvider::Claude => AppId::Claude,
        SessionProvider::Codex => AppId::Codex,
        SessionProvider::Gemini => AppId::Gemini,
        SessionProvider::Opencode => AppId::OpenCode,
        SessionProvider::Grokbuild => AppId::GrokBuild,
        SessionProvider::Pi => AppId::Pi,
        SessionProvider::Omp => AppId::Omp,
        SessionProvider::Kimi => AppId::Kimi,
        SessionProvider::Mcode => AppId::Mcode,
        SessionProvider::Agy => AppId::Agy,
        SessionProvider::Hermes | SessionProvider::Openclaw | SessionProvider::Dsh => {
            return Err("native session resume is unsupported for this application".into());
        }
    };
    // The authenticated gateway resolves the source instance's administrator-bound
    // installation ID; source instance IDs are independent of CLI installation IDs.
    let request = LaunchRequest {
        app_id: app,
        installation_id: installation.id.clone(),
        cwd: source.project_dir().ok_or("session has no verified working directory")?.into(),
        args: Vec::new(),
        skip_permissions: false,
    };
    let material = validate_launch_request(
        &request,
        &identity.target_host,
        std::slice::from_ref(installation),
        context.authorized_project_roots,
        false,
    )?;
    // These native resume commands expose no reviewed configuration-directory
    // argument. Refuse overrides rather than resume a different default instance.
    let default_dir = match identity.provider {
        SessionProvider::Grokbuild => Some(context.execution_home.join(".grok")),
        SessionProvider::Opencode => Some(context.execution_home.join(".config/opencode")),
        _ => None,
    };
    if default_dir.as_ref().is_some_and(|expected| expected != context.data_dir) {
        return Err("native resume does not support this instance directory override".into());
    }
    let id = &identity.session_id.0;
    // Native IDs are arguments but must not become options in CLI parsers.
    if id.starts_with('-') || id.chars().any(char::is_control) {
        return Err("invalid native resume identifier".into());
    }
    let mut env = BTreeMap::new();
    let data = path_text(context.data_dir)?;
    let args = match identity.provider {
        SessionProvider::Claude => {
            env.insert("CLAUDE_CONFIG_DIR", data);
            vec!["--resume".into(), id.clone()]
        }
        SessionProvider::Codex => {
            env.insert("CODEX_HOME", data);
            vec!["resume".into(), id.clone()]
        }
        SessionProvider::Gemini => {
            if context.data_dir.file_name().is_none_or(|name| name != ".gemini") {
                return Err(
                    "Gemini resume requires a native .gemini configuration directory".into()
                );
            }
            env.insert(
                "GEMINI_CLI_HOME",
                path_text(context.data_dir.parent().ok_or("invalid Gemini directory")?)?,
            );
            vec!["--resume".into(), id.clone()]
        }
        SessionProvider::Opencode => vec!["-s".into(), id.clone()],
        SessionProvider::Grokbuild => vec!["--resume".into(), id.clone()],
        SessionProvider::Kimi => {
            env.insert("KIMI_CODE_HOME", data);
            vec!["--session".into(), id.clone()]
        }
        SessionProvider::Mcode => {
            env.insert("MINIMAX_DATA_DIR", data);
            vec!["--session".into(), id.clone()]
        }
        SessionProvider::Pi => {
            env.insert("PI_CODING_AGENT_DIR", data);
            vec!["--session".into(), path_text(source.path())?]
        }
        SessionProvider::Omp => {
            let omp = context.omp.as_ref().ok_or("OMP resume requires its discovery context")?;
            if !omp::session_files(omp)?.iter().any(|path| path == source.path()) {
                return Err("OMP source does not match its execution context".into());
            }
            let profile = if omp.agent_dir == omp.native_agent_dir {
                omp.profile.unwrap_or("default")
            } else {
                "default"
            };
            if profile.is_empty()
                || profile.len() > 64
                || profile == "."
                || profile == ".."
                || !profile
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
            {
                return Err("invalid OMP profile".into());
            }
            env.insert("OMP_PROFILE", profile.into());
            env.insert("PI_PROFILE", profile.into());
            env.insert("PI_CODING_AGENT_DIR", path_text(omp.agent_dir)?);
            env.insert("PI_CODING_AGENT_SESSION_DIR", omp.env_session_dir.unwrap_or("").into());
            for (key, value) in [
                ("PI_CONFIG_DIR", context.pi_config_dir),
                ("XDG_DATA_HOME", omp.xdg_data_home),
                ("XDG_STATE_HOME", context.xdg_state_home),
                ("XDG_CACHE_HOME", context.xdg_cache_home),
            ] {
                env.insert(key, value.map(path_text).transpose()?.unwrap_or_default());
            }
            vec!["--resume".into(), path_text(source.path())?]
        }
        SessionProvider::Agy => {
            vec![format!("--gemini_dir={data}"), format!("--conversation={id}")]
        }
        _ => return Err("unsupported native resume".into()),
    };
    if env.values().any(|value| value.chars().any(char::is_control)) {
        return Err("invalid resume environment".into());
    }
    Ok(ResumePlan {
        identity: identity.clone(),
        program: material.program,
        cwd: material.cwd,
        args,
        env,
        home: context.execution_home.to_path_buf(),
        search_paths: context.search_paths.to_vec(),
    })
}

fn path_text(path: &Path) -> Result<String, String> {
    if !path.is_absolute() {
        return Err("resume path must be absolute".into());
    }
    path.to_str()
        .filter(|s| !s.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| "invalid resume path".into())
}
