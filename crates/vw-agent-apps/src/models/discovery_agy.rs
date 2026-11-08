//! Read the authorized execution host's Antigravity runtime catalog.

use std::{collections::BTreeSet, fs, future::Future, io::Read, path::Path};

use vw_api_types::{
    agent_apps::AppId,
    app_providers::{agy::AgyProfile, paths::ConfigPathState},
};

use super::discovery_process;
use crate::{
    apps::{
        discovery::read::{LocalReadEnvironment, locate_cli},
        launch::{host, request::LaunchMaterial},
    },
    providers::{agy, live::patch_json, paths},
};

const SETTINGS_LIMIT: usize = 1024 * 1024;

/// The caller authorizes this host and resolves any selected tenant provider to its saved profile.
/// Without a saved key, the CLI uses the execution host's own login state.
/// Returned names are the values used in native Antigravity settings.
pub async fn discover(
    host: &LocalReadEnvironment,
    saved_profile: Option<&AgyProfile>,
    cancelled: impl Future<Output = ()>,
) -> Result<Vec<String>, &'static str> {
    // Keep the directory alive until the process has exited, including on cancellation.
    let mut scratch = None;
    let output = discovery_process::output(cancelled, || {
        if let Some(profile) = saved_profile {
            agy::validate(profile).map_err(|_| "invalid_config")?;
        }
        let program = locate_cli(AppId::Agy, host).map_err(|_| "unavailable")?;
        let mut home = host.home.canonicalize().map_err(|_| "unavailable")?;
        let mut args = Vec::new();
        let key_mode = saved_profile
            .and_then(|profile| Some((profile.api_key.as_deref()?, profile.base_url.as_deref())));
        if key_mode.is_some() {
            let dir = tempfile::tempdir().map_err(|_| "unavailable")?;
            home = dir.path().canonicalize().map_err(|_| "unavailable")?;
            scratch = Some(dir);
            let settings_dir = home.join("antigravity-cli");
            fs::create_dir(&settings_dir).map_err(|_| "unavailable")?;
            fs::write(settings_dir.join("settings.json"), br#"{"modelProvider":"gemini"}"#)
                .map_err(|_| "unavailable")?;
            args.push(format!("--gemini_dir={}", home.to_str().ok_or("unavailable")?));
        } else {
            validate_settings(&home)?;
        }
        args.push("models".into());
        let material = LaunchMaterial { program, cwd: home.clone(), args, skip_permissions: false };
        let mut command =
            host::command(material, &home, &host.search_paths).map_err(|_| "unavailable")?;
        if let Some((key, base_url)) = key_mode {
            command.env("GEMINI_API_KEY", key);
            if let Some(url) = base_url {
                command.env("GOOGLE_GEMINI_BASE_URL", url);
            }
        }
        Ok(command)
    })
    .await?;
    Ok(parse(&output))
}

fn validate_settings(home: &Path) -> Result<(), &'static str> {
    let config =
        paths::read(home, AppId::Agy, None, &Default::default()).map_err(|_| "invalid_config")?;
    let settings = config.files.first().ok_or("invalid_config")?;
    let file = match paths::open_beneath(home, Path::new(&settings.path), false) {
        Ok(file) => file,
        Err(ConfigPathState::Absent) => return Ok(()),
        Err(_) => return Err("invalid_config"),
    };
    let mut bytes = Vec::new();
    file.take((SETTINGS_LIMIT + 1) as u64).read_to_end(&mut bytes).map_err(|_| "invalid_config")?;
    if bytes.len() > SETTINGS_LIMIT {
        return Err("invalid_config");
    }
    let document = patch_json::object_value(&bytes).map_err(|_| "invalid_config")?;
    if document.get("model").is_some_and(|model| !model.is_string() && !model.is_null()) {
        return Err("invalid_config");
    }
    Ok(())
}

fn parse(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let (alias, name) = line.split_once('\t')?;
            if alias.is_empty()
                || !alias.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./:".contains(&b))
                || name.chars().any(char::is_control)
            {
                return None;
            }
            let name = name.trim();
            (!name.is_empty() && name.len() <= 256 && !name.starts_with('-')).then_some(name)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect()
}
