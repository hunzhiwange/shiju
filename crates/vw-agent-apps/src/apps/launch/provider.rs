//! Authorized provider processes; temporary profiles or a persistent Agy profile lease.
use super::request::LaunchMaterial;
use crate::providers::validate_profile;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use vw_api_types::{agent_apps::AppId, app_providers::AppProviderWrite};

/// Not serializable or Debug: argv/environment must never become API responses or logs.
pub struct ProviderLaunch {
    pub(crate) program: PathBuf,
    pub(crate) cwd: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(&'static str, String)>,
    pub(crate) private: Option<tempfile::TempDir>,
    #[cfg(target_os = "macos")]
    pub(crate) agy: Option<crate::providers::agy::Lease>,
    #[cfg(target_os = "macos")]
    pub(crate) agy_catalog: Option<std::fs::File>,
}
impl ProviderLaunch {
    /// Retain configuration until child exit; Agy sessions persist, temporary profiles are removed.
    /// Returns only a fixed error, never child output (which may contain credentials).
    pub fn run(self) -> Result<(), String> {
        self.run_with(|command| {
            command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
        })
    }
    /// Trusted execution-host transport retains this owner until CLI reap and cleanup.
    pub fn run_with(
        self,
        execute: impl FnOnce(&mut Command) -> std::io::Result<std::process::ExitStatus>,
    ) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        if let Some(lease) = &self.agy {
            lease.validate().map_err(|_| "Agy profile changed before launch")?;
        }
        let mut command = Command::new(&self.program);
        command.args(&self.args).current_dir(&self.cwd).env_clear();
        // Preserve OS essentials only. Never inherit host API keys, runtime hooks or provider selectors.
        for name in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SystemRoot",
            "WINDIR",
            "TEMP",
            "TMP",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TERM",
            "COLORTERM",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        let result = execute(&mut command);
        #[cfg(target_os = "macos")]
        drop(self.agy_catalog);
        // The private profile must outlive the process, even though it is not read here.
        if let Some(private) = self.private {
            private.close().map_err(|_| "private provider configuration cleanup failed")?;
        }
        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err("provider process exited unsuccessfully".into()),
            Err(_) => Err("provider process could not be started".into()),
        }
    }
}

fn write_private(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    write_bytes(path, value.to_string().as_bytes())
}
fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|_| "cannot create private provider configuration".into())
}

pub fn prepare_provider_launch(
    app: AppId,
    profile: &AppProviderWrite,
    material: LaunchMaterial,
    private_temp_root: &Path,
) -> Result<ProviderLaunch, String> {
    if app == AppId::Agy {
        return Err("Agy requires persistent execution-host profile selection".into());
    }
    validate_profile(app, profile)?;
    if profile.request_options.as_ref().is_some_and(|v| {
        v.api_key_field.is_some()
            || v.request_headers.as_ref().is_some_and(|h| !h.is_empty())
            || v.local_proxy_request_overrides.as_ref().is_some_and(|o| {
                o.headers.as_ref().is_some_and(|h| !h.is_empty())
                    || o.body.as_ref().is_some_and(|b| !b.is_empty())
            })
    }) {
        return Err("provider request options require a native application adapter".into());
    }
    if !material.program.is_absolute()
        || !material.cwd.is_absolute()
        || !material.program.is_file()
        || !material.cwd.is_dir()
        || !private_temp_root.is_absolute()
    {
        return Err("provider launch requires validated execution-host paths".into());
    }
    if material
        .program
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
    {
        return Err("provider batch entrypoint requires a reviewed Windows host adapter".into());
    }
    if material.args.len() > 32
        || material.args.iter().any(|arg| {
            arg.starts_with('-') || arg.len() > 8192 || arg.chars().any(char::is_control)
        })
    {
        return Err("invalid provider launch arguments".into());
    }
    // A project override could select another provider despite a private user profile.
    for ancestor in material.cwd.ancestors() {
        let conflict = match app {
            AppId::Codex => ancestor.join(".codex/config.toml").exists(),
            AppId::Gemini => {
                ancestor.join(".gemini/settings.json").exists()
                    || ancestor.join(".gemini/.env").exists()
                    || ancestor.join(".env").exists()
            }
            AppId::Omp => {
                let directory = ancestor.join(".omp");
                (directory.exists() && !directory.is_dir())
                    || directory.join("config.yml").exists()
                    || directory.join("config.yaml").exists()
                    || ancestor.join(".env").exists()
            }
            _ => false,
        };
        if conflict {
            return Err("project configuration conflicts with isolated provider launch".into());
        }
        if ancestor.join(".git").exists() {
            break;
        }
    }
    let mut builder = tempfile::Builder::new();
    builder.prefix("shiju-provider-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    let private = builder
        .tempdir_in(private_temp_root)
        .map_err(|_| "cannot create private provider directory")?;
    let path = private.path().to_str().ok_or("private provider directory is not UTF-8")?.to_owned();
    let mut args = Vec::new();
    let mut env = Vec::new();
    use AppId::*;
    match app {
        Claude => {
            let settings = private.path().join("settings.json");
            write_private(&settings, &serde_json::json!({"model": profile.model}))?;
            args.extend([
                "--setting-sources".into(),
                String::new(),
                "--settings".into(),
                settings.to_str().ok_or("invalid private settings path")?.into(),
                "--model".into(),
                profile.model.clone(),
            ]);
            env.extend([
                ("CLAUDE_CONFIG_DIR", path),
                ("ANTHROPIC_API_KEY", profile.api_key.clone()),
                ("ANTHROPIC_BASE_URL", profile.base_url.clone()),
            ]);
        }
        Codex => {
            // JSON quoted strings are valid TOML basic strings after control characters are rejected.
            let quote = |s: &str| serde_json::to_string(s).expect("string serialization");
            let config = format!(
                "model = {}\nmodel_provider = \"shiju_session\"\n[model_providers.shiju_session]\nname = \"Single process provider\"\nbase_url = {}\nenv_key = \"SHIJU_PROVIDER_API_KEY\"\nwire_api = \"responses\"\n",
                quote(&profile.model),
                quote(&profile.base_url)
            );
            write_bytes(&private.path().join("config.toml"), config.as_bytes())?;
            env.extend([("CODEX_HOME", path), ("SHIJU_PROVIDER_API_KEY", profile.api_key.clone())]);
        }
        Gemini => {
            let config_dir = private.path().join(".gemini");
            fs::create_dir(&config_dir)
                .map_err(|_| "cannot create private Gemini configuration")?;
            write_private(
                &config_dir.join("settings.json"),
                &serde_json::json!({"security":{"auth":{"selectedType":"gemini-api-key"}}, "advanced":{"ignoreLocalEnv":true}, "model":{"name": profile.model}}),
            )?;
            args.extend(["--model".into(), profile.model.clone()]);
            env.extend([
                ("GEMINI_CLI_HOME", path),
                ("GEMINI_API_KEY", profile.api_key.clone()),
                ("GOOGLE_GEMINI_BASE_URL", profile.base_url.clone()),
            ]);
        }
        Omp => {
            // JSON is valid YAML; keep credentials out of configuration and argv.
            write_private(
                &private.path().join("models.yml"),
                &serde_json::json!({"providers":{"shiju_session":{
                    "baseUrl":profile.base_url,"api":"openai-responses",
                    "apiKey":"SHIJU_PROVIDER_API_KEY","authHeader":true,
                    "models":[{"id":profile.model,"name":profile.model}]
                }}}),
            )?;
            args.extend([
                "--model".into(),
                format!("shiju_session/{}", profile.model),
                "--no-extensions".into(),
                "--no-skills".into(),
            ]);
            env.extend([
                ("PI_CODING_AGENT_DIR", path.clone()),
                ("HOME", path.clone()),
                ("XDG_CONFIG_HOME", private.path().join(".config").to_string_lossy().into_owned()),
                ("XDG_CACHE_HOME", private.path().join(".cache").to_string_lossy().into_owned()),
                (
                    "XDG_DATA_HOME",
                    private.path().join(".local/share").to_string_lossy().into_owned(),
                ),
                (
                    "XDG_STATE_HOME",
                    private.path().join(".local/state").to_string_lossy().into_owned(),
                ),
                ("SHIJU_PROVIDER_API_KEY", profile.api_key.clone()),
            ]);
        }
        // validate_profile rejects every unimplemented capability individually before creating files.
        _ => return Err("application provider session capability is not implemented".into()),
    }
    if material.skip_permissions {
        let flag = match app {
            Claude | Agy => "--dangerously-skip-permissions",
            Codex => "--dangerously-bypass-approvals-and-sandbox",
            _ => return Err("permission skipping is not supported by this provider adapter".into()),
        };
        args.push(flag.into());
    }
    args.extend(material.args);
    Ok(ProviderLaunch {
        program: material.program,
        cwd: material.cwd,
        args,
        env,
        private: Some(private),
        #[cfg(target_os = "macos")]
        agy: None,
        #[cfg(target_os = "macos")]
        agy_catalog: None,
    })
}
