//! Reviewed patched CLIs installed beside, rather than over, native user installations.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Contract {
    #[serde(default)]
    layout: String,
    package: String,
    version: String,
    bin: String,
    files: std::collections::BTreeMap<String, String>,
    engine: Option<String>,
    engine_version: Option<String>,
    source_sha256: Option<String>,
    patch_sha256: Option<String>,
}
const CONTRACTS: &str = include_str!("managed_runtime/contracts.json");
fn contract(agent: &str) -> Option<Contract> {
    serde_json::from_str::<std::collections::HashMap<String, Contract>>(CONTRACTS)
        .ok()?
        .remove(agent)
}
fn root() -> Result<PathBuf, String> {
    if let Some(directory) = std::env::var_os("VIBEWINDOW_CONFIG_DIR") {
        return Ok(PathBuf::from(directory).join("acp-runtime"));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/shiju/acp-runtime"))
        .ok_or_else(|| "ACP runtime installation requires a home directory".into())
}
fn installed_directory(agent: &str, contract: &Contract) -> Result<PathBuf, String> {
    Ok(root()?.join(format!("{agent}-{}-model-source-v1", contract.version)))
}
fn verify(directory: &Path, contract: &Contract) -> Result<PathBuf, String> {
    if contract.layout == "standalone" {
        for (name, digest) in &contract.files {
            if hex::encode(Sha256::digest(
                std::fs::read(directory.join(name))
                    .map_err(|_| "Missing reviewed standalone artifact")?,
            )) != *digest
            {
                return Err("Standalone ACP artifact checksum mismatch".into());
            }
        }
        return Ok(directory.join(&contract.bin));
    }
    if contract.layout == "native" {
        let receipt: serde_json::Value = serde_json::from_slice(
            &std::fs::read(directory.join("build-receipt.json"))
                .map_err(|_| "Missing native build receipt")?,
        )
        .map_err(|_| "Invalid native build receipt")?;
        let executable = directory.join(&contract.bin);
        if receipt["version"] != contract.version
            || receipt["source_sha256"].as_str() != contract.source_sha256.as_deref()
            || receipt["patch_sha256"].as_str() != contract.patch_sha256.as_deref()
            || receipt["sha256"]
                != hex::encode(Sha256::digest(
                    std::fs::read(&executable).map_err(|_| "Missing native artifact")?,
                ))
        {
            return Err("Native ACP build receipt checksum mismatch".into());
        }
        return Ok(executable);
    }
    if contract.layout == "python" {
        let site = directory.join("lib/python3.12/site-packages");
        for (name, digest) in &contract.files {
            if hex::encode(Sha256::digest(
                std::fs::read(site.join(name)).map_err(|_| "Missing reviewed Python artifact")?,
            )) != *digest
            {
                return Err("Python ACP artifact checksum mismatch".into());
            }
        }
        let executable = directory.join("bin").join(&contract.bin);
        return executable
            .is_file()
            .then_some(executable)
            .ok_or_else(|| "Missing reviewed Python ACP entry".into());
    }
    let package = directory.join("node_modules").join(&contract.package);
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(package.join("package.json"))
            .map_err(|_| "ACP patched runtime is not installed")?,
    )
    .map_err(|_| "Invalid ACP package metadata")?;
    if metadata["name"] != contract.package || metadata["version"] != contract.version {
        return Err("ACP patched package version differs from reviewed artifact".into());
    }
    for (name, digest) in &contract.files {
        let bytes =
            std::fs::read(package.join(name)).map_err(|_| "Missing reviewed ACP artifact")?;
        if hex::encode(Sha256::digest(bytes)) != *digest {
            return Err("ACP patched artifact checksum mismatch".into());
        }
    }
    if let (Some(engine), Some(version)) = (&contract.engine, &contract.engine_version) {
        let engine_root = directory.join("node_modules").join(engine);
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(engine_root.join("package.json"))
                .map_err(|_| "Missing pinned ACP engine")?,
        )
        .map_err(|_| "Invalid ACP engine metadata")?;
        if metadata["name"] != *engine || metadata["version"] != *version {
            return Err("ACP engine version mismatch".into());
        }
        if engine == "@mariozechner/pi-coding-agent"
            && hex::encode(Sha256::digest(
                std::fs::read(engine_root.join("dist/cli.js")).map_err(|_| "Missing Pi engine")?,
            )) != "e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa"
        {
            return Err("Pi engine checksum mismatch".into());
        }
    }
    Ok(package.join(&contract.bin))
}

/// Absence is explicit: callers must not advertise patched support based on a CLI name.
pub fn reviewed_agent_command(agent: &str) -> Result<Option<(String, Vec<String>)>, String> {
    let Some(contract) = contract(agent) else { return Ok(None) };
    let path = verify(&installed_directory(agent, &contract)?, &contract)?;
    if !contract.layout.is_empty() {
        return Ok(Some((
            path.to_string_lossy().into_owned(),
            match agent {
                "vibe" => vec![],
                "trae" => vec!["acp".into(), "serve".into()],
                "droid" => vec!["exec".into(), "--output-format".into(), "acp".into()],
                _ => vec!["acp".into()],
            },
        )));
    }
    let args = match agent {
        "gemini" => vec!["--experimental-acp".into()],
        "iflow" => vec!["--experimental-acp".into()],
        "auggie" | "qoder" | "qwen" | "snow" => vec!["--acp".into()],
        "copilot" => vec!["--acp".into(), "--stdio".into()],
        "kilocode" => vec!["acp".into(), "--pure".into()],
        "opencode" | "openclaw" => vec!["acp".into()],
        _ => vec![],
    };
    // Drivers validate the supplier's executable basename, while npm owns the symlink.
    let bin_name = match agent {
        "codex" => "codex-acp",
        "pi" => "pi-acp",
        "deepagents" => "deepagents-acp",
        "qoder" => "qodercli",
        other => other,
    };
    let executable =
        installed_directory(agent, &contract)?.join("node_modules/.bin").join(bin_name);
    if executable.canonicalize().ok() != path.canonicalize().ok() || !executable.is_file() {
        return Err("ACP executable does not resolve to its reviewed artifact".into());
    }
    Ok(Some((executable.to_string_lossy().into_owned(), args)))
}

/// Validate explicit overrides too; a familiar executable name is never proof of a patch.
pub fn verify_reviewed_agent_executable(agent: &str, command: &str) -> Result<(), String> {
    if agent == "claude" {
        let command = command.trim();
        return if Path::new(command).is_absolute()
            && (crate::access_policy::claude::verified_entry(command, &[])
                || crate::access_policy::claude::verified_effort_entry(command))
        {
            Ok(())
        } else {
            Err("Claude managed launch requires an installed verified ACP adapter".into())
        };
    }
    let Some(contract) = contract(agent) else { return Ok(()) };
    let executable = Path::new(command)
        .canonicalize()
        .map_err(|_| "Reviewed ACP executable is unavailable; install it on the gateway host")?;
    if let Ok(Some((installed, _))) = reviewed_agent_command(agent)
        && Path::new(&installed).canonicalize().ok().as_ref() == Some(&executable)
    {
        return Ok(());
    }
    if contract.layout == "native" {
        if let Some(directory) = executable.parent()
            && verify(directory, &contract).is_ok_and(|path| path == executable)
        {
            return Ok(());
        }
        if hex::encode(Sha256::digest(
            std::fs::read(&executable).map_err(|_| "Cannot read Goose artifact")?,
        )) == "758bb14d83ac54928b52ed2740870b5708d141d46bc9e7cbe2c2a08c146deccc"
        {
            return Ok(());
        }
    } else if contract.layout == "python" {
        let directory =
            Path::new(command).parent().and_then(Path::parent).ok_or("Invalid Python ACP entry")?;
        verify(directory, &contract)?;
        return Ok(());
    } else {
        for directory in executable.ancestors() {
            if let Ok(path) = verify(directory, &contract)
                && path.canonicalize().ok().as_ref() == Some(&executable)
            {
                return Ok(());
            }
        }
    }
    Err("ACP executable is not the reviewed patched release; install the model-source runtime"
        .into())
}

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) async fn install(agent: &str) -> Result<bool, String> {
    let Some(contract) = contract(agent) else { return Ok(false) };
    if agent == "deepagents" && cfg!(windows) {
        return Err("DeepAgents reviewed runtime is not yet supported on Windows".into());
    }
    if agent != "deepagents" && !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        // Preserve the existing native installer for CLI-local use on unverified hosts.
        return Ok(false);
    }
    let destination = installed_directory(agent, &contract)?;
    if destination.exists() {
        verify(&destination, &contract)?;
        return Ok(true);
    }
    std::fs::create_dir_all(root()?).map_err(|_| "Cannot create ACP runtime root")?;
    let stage = Stage(root()?.join(format!(
            ".install-{agent}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "Invalid installation clock")?
                .as_nanos()
        )));
    std::fs::create_dir(&stage.0).map_err(|_| "Cannot create ACP installation stage")?;
    let assets = stage.0.join("assets");
    for (name, text) in [
        ("install.py", include_str!("managed_runtime/install.py")),
        ("deepagents-lock.json", include_str!("managed_runtime/deepagents-lock.json")),
        (
            "deepagents/apply.py",
            include_str!("../../../patches/acp-model-source/deepagents/apply.py"),
        ),
        (
            "deepagents/guard.mjs",
            include_str!("../../../patches/acp-model-source/deepagents/guard.mjs"),
        ),
        ("trae/apply.py", include_str!("../../../patches/acp-model-source/trae/apply.py")),
        ("trae/launcher.py", include_str!("../../../patches/acp-model-source/trae/launcher.py")),
        ("cursor/apply.py", include_str!("../../../patches/acp-model-source/cursor/apply.py")),
        ("cursor/patch.json", include_str!("../../../patches/acp-model-source/cursor/patch.json")),
        (
            "cursor/session-model.cjs",
            include_str!("../../../patches/acp-model-source/cursor/session-model.cjs"),
        ),
        ("qoder/apply.py", include_str!("../../../patches/acp-model-source/qoder/apply.py")),
        ("qoder/patch.json", include_str!("../../../patches/acp-model-source/qoder/patch.json")),
        (
            "qoder/session-model.mjs",
            include_str!("../../../patches/acp-model-source/qoder/session-model.mjs"),
        ),
        ("auggie/apply.py", include_str!("../../../patches/acp-model-source/auggie/apply.py")),
        ("auggie/patch.json", include_str!("../../../patches/acp-model-source/auggie/patch.json")),
        (
            "auggie/session-model.mjs",
            include_str!("../../../patches/acp-model-source/auggie/session-model.mjs"),
        ),
        ("openclaw/apply.py", include_str!("../../../patches/acp-model-source/openclaw/apply.py")),
        (
            "openclaw/patch.json",
            include_str!("../../../patches/acp-model-source/openclaw/patch.json"),
        ),
        (
            "opencode/apply-skills.py",
            include_str!("../../../patches/acp-model-source/opencode/apply-skills.py"),
        ),
        (
            "opencode/skill-patch.json",
            include_str!("../../../patches/acp-model-source/opencode/skill-patch.json"),
        ),
        ("kilocode/apply.py", include_str!("../../../patches/acp-model-source/kilocode/apply.py")),
        (
            "kilocode/patch.json",
            include_str!("../../../patches/acp-model-source/kilocode/patch.json"),
        ),
        ("snow/apply.py", include_str!("../../../patches/acp-model-source/snow/apply.py")),
        ("snow/patch.json", include_str!("../../../patches/acp-model-source/snow/patch.json")),
        ("copilot/apply.py", include_str!("../../../patches/acp-model-source/copilot/apply.py")),
        (
            "copilot/patch.json",
            include_str!("../../../patches/acp-model-source/copilot/patch.json"),
        ),
        ("kiro/apply.py", include_str!("../../../patches/acp-model-source/kiro/apply.py")),
        ("kiro/patch.json", include_str!("../../../patches/acp-model-source/kiro/patch.json")),
        ("kiro/launcher.py", include_str!("../../../patches/acp-model-source/kiro/launcher.py")),
        ("contracts.json", CONTRACTS),
        ("iflow-lock.json", include_str!("managed_runtime/iflow-lock.json")),
        (
            "vibe/apply_patch.py",
            include_str!("../../../docs/shiju/acp-model-source/evidence/vibe/apply_patch.py"),
        ),
        (
            "vibe/upstream-hashes.json",
            include_str!("../../../docs/shiju/acp-model-source/evidence/vibe/upstream-hashes.json"),
        ),
        (
            "vibe/mistral-vibe-3.0.4-session-binding.patch",
            include_str!(
                "../../../docs/shiju/acp-model-source/evidence/vibe/mistral-vibe-3.0.4-session-binding.patch"
            ),
        ),
        (
            "hermes/apply_patch.py",
            include_str!("../../../docs/shiju/acp-model-source/evidence/hermes/apply_patch.py"),
        ),
        (
            "hermes/patch.json",
            include_str!("../../../docs/shiju/acp-model-source/evidence/hermes/patch.json"),
        ),
        (
            "hermes/hermes-agent-0.19.0-session-state.patch",
            include_str!(
                "../../../docs/shiju/acp-model-source/evidence/hermes/hermes-agent-0.19.0-session-state.patch"
            ),
        ),
        (
            "goose.patch",
            include_str!(
                "../../../docs/shiju/acp-model-source/evidence/goose/v1.57.0-model-lock.patch"
            ),
        ),
        (
            "codex.patch",
            include_str!(
                "../../../patches/acp-model-source/codex/codex-acp-2.5.0-model-source.patch"
            ),
        ),
        (
            "gemini.patch",
            include_str!(
                "../../../patches/acp-model-source/gemini/gemini-cli-0.66.0-history.patch"
            ),
        ),
        (
            "pi.patch",
            include_str!("../../../patches/acp-model-source/pi/pi-acp-0.0.22-state-dir.patch"),
        ),
        ("iflow/apply.py", include_str!("../../../patches/acp-model-source/iflow/apply.py")),
        (
            "codebuddy/apply.py",
            include_str!("../../../patches/acp-model-source/codebuddy/apply.py"),
        ),
        (
            "codebuddy/patch.json",
            include_str!("../../../patches/acp-model-source/codebuddy/patch.json"),
        ),
        ("qwen/apply.py", include_str!("../../../patches/acp-model-source/qwen/apply.py")),
        ("qwen/patch.json", include_str!("../../../patches/acp-model-source/qwen/patch.json")),
    ] {
        let path = assets.join(name);
        std::fs::create_dir_all(path.parent().ok_or("Invalid ACP asset path")?)
            .map_err(|_| "Cannot create ACP patch directory")?;
        std::fs::write(path, text).map_err(|_| "Cannot write embedded ACP patch")?;
    }
    let runtime = stage.0.join("runtime");
    let uv = if matches!(agent, "vibe" | "hermes") {
        crate::install::ensure_uv(tokio::time::Instant::now() + std::time::Duration::from_secs(300))
            .await?
    } else {
        String::new()
    };
    let script = assets.join("install.py");
    crate::install::run_installer(
        "python3",
        &[
            script.to_str().ok_or("Invalid installer path")?,
            assets.to_str().ok_or("Invalid asset path")?,
            runtime.to_str().ok_or("Invalid runtime path")?,
            agent,
            &uv,
        ],
        tokio::time::Instant::now() + std::time::Duration::from_secs(900),
    )
    .await?;
    verify(&runtime, &contract)?;
    std::fs::rename(&runtime, &destination)
        .map_err(|_| "Cannot atomically publish reviewed ACP runtime")?;
    reviewed_agent_command(agent)?;
    Ok(true)
}

#[cfg(test)]
#[path = "managed_runtime_tests.rs"]
mod tests;
