//! Reviewed patched CLIs installed beside, rather than over, native user installations.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/linux_cli.rs"]
mod linux_cli;
#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/codex.rs"]
mod codex;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/omp.rs"]
mod omp;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/stakpak.rs"]
mod stakpak;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/vtcode.rs"]
mod vtcode;

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/droid.rs"]
mod droid;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/iflow.rs"]
mod iflow;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "managed_runtime/junie.rs"]
mod junie;

pub fn linux_native_only(agent: &str) -> bool {
    cfg!(target_os = "linux")
        && matches!(agent, "corust-agent" | "junie" | "mimo-code" | "omp" | "poolside" | "stakpak" | "trae" | "vtcode")
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) fn linux_cli_command(agent: &str) -> Result<Option<(String, Vec<String>)>, String> {
    if !linux_native_only(agent) {
        return Ok(None);
    }
    if agent == "junie" {
        return junie::discover(&root(agent)?);
    }
    linux_cli::discover(agent, &root(agent)?)
}

/// Recheck a fixed Linux CLI entry before spawning it from an existing configuration.
pub fn verify_fixed_linux_cli_launch(agent: &str, command: &str, args: &[String]) -> Result<(), String> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if linux_native_only(agent) && Path::new(command).is_absolute() {
        if agent == "junie" {
            return junie::verify_launch(&root(agent)?, Path::new(command), args);
        }
        return linux_cli::verify_launch(agent, &root(agent)?, Path::new(command), args);
    }
    let _ = (agent, command, args);
    Ok(())
}

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
const GLM_LINUX_CONTRACT: &str = include_str!(
    "../../../../patches/acp-model-source/glm-acp-agent/linux-host-mcp.json"
);

#[derive(Deserialize)]
struct GlmLinuxOverlay {
    files: std::collections::BTreeMap<String, String>,
}

const AMP_LINUX_CONTRACT: &str = include_str!(
    "../../../../patches/acp-model-source/amp-acp/linux-runtime.json"
);
const LINUX_GNU: bool = cfg!(all(
    target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")
));

#[derive(Deserialize)]
struct AmpLinuxContract {
    revision: String,
    files: std::collections::BTreeMap<String, String>,
}

fn contract(agent: &str) -> Option<Contract> {
    let mut contract = serde_json::from_str::<std::collections::HashMap<String, Contract>>(CONTRACTS)
        .ok()?
        .remove(agent)?;
    if agent == "glm-acp-agent" && LINUX_GNU {
        let overlay: GlmLinuxOverlay = serde_json::from_str(GLM_LINUX_CONTRACT).ok()?;
        contract.files.extend(overlay.files);
    }
    if agent == "amp-acp" && LINUX_GNU {
        let linux: AmpLinuxContract = serde_json::from_str(AMP_LINUX_CONTRACT).ok()?;
        contract.files = linux.files;
    }
    Some(contract)
}
fn root(_agent: &str) -> Result<PathBuf, String> {
    if let Some(directory) = std::env::var_os("VIBEWINDOW_CONFIG_DIR") {
        return Ok(PathBuf::from(directory).join("acp-runtime"));
    }
    #[cfg(windows)]
    if _agent == "glm-acp-agent" {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map(|path| path.join("shiju/acp-runtime"))
            .ok_or_else(|| "GLM runtime installation requires LOCALAPPDATA".into());
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/shiju/acp-runtime"))
        .ok_or_else(|| "ACP runtime installation requires a home directory".into())
}
fn installed_directory(agent: &str, contract: &Contract) -> Result<PathBuf, String> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if agent == "iflow" {
        return Ok(root(agent)?.join(iflow::Manifest::load()?.revision));
    }
    if agent == "amp-acp" && LINUX_GNU {
        let linux: AmpLinuxContract = serde_json::from_str(AMP_LINUX_CONTRACT)
            .map_err(|_| "Invalid Amp Linux runtime contract")?;
        return Ok(root(agent)?.join(linux.revision));
    }
    // Install revised contracts beside their prior reviewed runtimes.
    let revision = match agent {
        "glm-acp-agent" if LINUX_GNU => 5,
        "cursor" => 6,
        "cortex-code" | "amp-acp" | "harn" | "deepagents" => 5,
        "pi" | "vibe" | "corust-agent" | "sigit" | "snow" | "stakpak" => 4,
        "gemini" | "nova" | "glm-acp-agent" | "aionrs" | "dimcode" | "junie" | "hermes"
        | "poolside" | "openclaw" => 3,
        "vtcode"
            if contract
                .files
                .get(&contract.bin)
                .is_some_and(|digest| digest != VTCODE_V2_SHA256) =>
        {
            3
        }
        "auggie" | "copilot" | "goose" | "kilocode" | "opencode" | "droid" | "kiro" | "vtcode"
        | "codebuddy" | "codex" | "dirac" | "iflow" | "qoder" | "qwen" => 2,
        _ => 1,
    };
    Ok(root(agent)?.join(format!("{agent}-{}-model-source-v{revision}", contract.version)))
}

fn original_vtcode_directory(contract: &mut Contract) -> Result<PathBuf, String> {
    let digest = contract.source_sha256.as_ref().ok_or("Missing original VT Code checksum")?;
    contract.files.insert(contract.bin.clone(), digest.clone());
    Ok(root("vtcode")?.join(format!("vtcode-{}-model-source-v1", contract.version)))
}

pub fn verify_vtcode_mcp_executable(command: &str) -> Result<(), String> {
    if linux_native_only("vtcode") {
        return Err("No reviewed VT Code managed executable for Linux".into());
    }
    let executable = Path::new(command).canonicalize().map_err(|_| "VT Code executable is unavailable")?;
    let contract = contract("vtcode").ok_or("Missing VT Code runtime contract")?;
    if verify_vtcode_patched_path(&executable, &contract) || verify_vtcode_legacy_path(&executable, &contract) {
        Ok(())
    } else {
        Err("VT Code MCP requires the reviewed patched runtime".into())
    }
}

pub fn verify_vtcode_skill_executable(command: &str) -> Result<(), String> {
    if linux_native_only("vtcode") {
        return Err("No reviewed VT Code managed executable for Linux".into());
    }
    let executable = Path::new(command).canonicalize().map_err(|_| "VT Code executable is unavailable")?;
    let contract = contract("vtcode").ok_or("Missing VT Code runtime contract")?;
    if cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && contract.files.get(&contract.bin).is_some_and(|digest| digest != VTCODE_V2_SHA256)
        && verify_vtcode_patched_path(&executable, &contract)
    {
        Ok(())
    } else {
        Err("VT Code skills require the reviewed read-only ACP runtime".into())
    }
}

fn verify_vtcode_patched_path(executable: &Path, contract: &Contract) -> bool {
    executable.parent().is_some_and(|directory| {
        verify(directory, contract).is_ok_and(|reviewed| reviewed.as_path() == executable)
    })
}

const VTCODE_V2_SHA256: &str = "8b8f6c5204e1125b20d63c7c9637b027821b22d206517ebd00db5f513890cdfb";

fn verify_vtcode_legacy_path(executable: &Path, contract: &Contract) -> bool {
    executable.file_name().is_some_and(|name| name == contract.bin.as_str())
        && std::fs::read(executable).is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == VTCODE_V2_SHA256)
}

fn verify(directory: &Path, contract: &Contract) -> Result<PathBuf, String> {
    if contract.layout == "standalone" {
        if contract.package == "harn-cli" {
            for name in contract.files.keys().map(String::as_str).chain(["harn-native", "build-receipt.json"]) {
                if !std::fs::symlink_metadata(directory.join(name))
                    .is_ok_and(|metadata| metadata.file_type().is_file())
                {
                    return Err("Harn artifact is missing or redirected".into());
                }
            }
        }
        for (name, digest) in &contract.files {
            if hex::encode(Sha256::digest(
                std::fs::read(directory.join(name))
                    .map_err(|_| "Missing reviewed standalone artifact")?,
            )) != *digest
            {
                return Err("Standalone ACP artifact checksum mismatch".into());
            }
        }
        if contract.package == "harn-cli" {
            let receipt: serde_json::Value = serde_json::from_slice(
                &std::fs::read(directory.join("build-receipt.json"))
                    .map_err(|_| "Missing Harn build receipt")?,
            ).map_err(|_| "Invalid Harn build receipt")?;
            use std::io::Read;
            let mut engine = std::fs::File::open(directory.join("harn-native"))
                .map_err(|_| "Missing Harn engine")?;
            let mut digest = Sha256::new();
            let mut block = [0; 64 * 1024];
            loop {
                let size = engine.read(&mut block).map_err(|_| "Cannot read Harn engine")?;
                if size == 0 {
                    break;
                }
                digest.update(&block[..size]);
            }
            let digest = hex::encode(digest.finalize());
            if receipt["version"] != contract.version
                || receipt["source_sha256"].as_str() != contract.source_sha256.as_deref()
                || receipt["patch_sha256"].as_str() != contract.patch_sha256.as_deref()
                || receipt["engine_sha256"].as_str() != Some(digest.as_str())
            {
                return Err("Harn engine build receipt checksum mismatch".into());
            }
        }
        if contract.package == "stakpak" {
            let receipt: serde_json::Value = serde_json::from_slice(
                &std::fs::read(directory.join("build-receipt.json"))
                    .map_err(|_| "Missing Stakpak build receipt")?,
            ).map_err(|_| "Invalid Stakpak build receipt")?;
            let digest = hex::encode(Sha256::digest(
                std::fs::read(directory.join("stakpak-native"))
                    .map_err(|_| "Missing Stakpak native program")?,
            ));
            if receipt["version"] != contract.version
                || receipt["source_sha256"].as_str() != contract.source_sha256.as_deref()
                || receipt["patch_sha256"].as_str() != contract.patch_sha256.as_deref()
                || receipt["sha256"].as_str() != Some(digest.as_str())
            {
                return Err("Stakpak native build receipt checksum mismatch".into());
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
        if contract.package == "aion-cli"
            && receipt["engine_sha256"]
                != hex::encode(Sha256::digest(
                    std::fs::read(directory.join("aionrs")).map_err(|_| "Missing aionrs engine")?,
                ))
        {
            return Err("Native aionrs engine checksum mismatch".into());
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
    if contract.package == "glm-acp-agent" {
        // Resolve the host's runtime root (e.g. macOS /var), then reject redirects
        // within the versioned installation, including package junctions on Windows.
        let parent = directory.parent().ok_or("Missing GLM runtime root")?;
        let base = parent.canonicalize().map_err(|_| "Missing GLM runtime root")?;
        let version = directory.file_name().ok_or("Missing GLM runtime directory")?;
        let expected = base.join(version).join("node_modules").join(&contract.package);
        if !contract.files.contains_key(&contract.bin) {
            return Err("GLM entry is missing from the reviewed file contract".into());
        }
        for name in contract.files.keys().map(String::as_str).chain(["package.json"]) {
            let file = package.join(name);
            if file.canonicalize().ok().as_ref() != Some(&expected.join(name)) {
                return Err("GLM artifact is missing or redirected".into());
            }
        }
    }
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(package.join("package.json"))
            .map_err(|_| "ACP patched runtime is not installed")?,
    )
    .map_err(|_| "Invalid ACP package metadata")?;
    if metadata["name"] != contract.package || metadata["version"] != contract.version {
        return Err("ACP patched package version differs from reviewed artifact".into());
    }
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if contract.package == "@iflow-ai/iflow-cli" {
        iflow::verify(directory, contract)?;
        return Ok(package.join(&contract.bin));
    }
    if contract.package == "amp-acp" && LINUX_GNU {
        let base = directory.parent().ok_or("Missing Amp runtime root")?
            .canonicalize().map_err(|_| "Missing Amp runtime root")?;
        let modules = base.join(directory.file_name().ok_or("Missing Amp runtime directory")?)
            .join("node_modules");
        for file in contract.files.keys().map(String::as_str).chain(["package.json", "../@sourcegraph/amp/package.json"]) {
            let expected = match file.strip_prefix("../") {
                Some(worker) => modules.join(worker),
                None => modules.join("amp-acp").join(file),
            };
            if expected.canonicalize().ok().as_ref() != Some(&expected) {
                return Err("Amp Linux artifact is missing or redirected".into());
            }
        }
    }
    for (name, digest) in &contract.files {
        let bytes = std::fs::read(package.join(name)).map_err(|_| {
            if contract.package == "amp-acp" && name.starts_with("../@sourcegraph/amp/") {
                "Amp main CLI is missing; reinstall the reviewed runtime"
            } else {
                "Missing reviewed ACP artifact"
            }
        })?;
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

fn use_auggie_v1_contract(contract: &mut Contract) {
    contract.files.insert("augment.mjs".into(),
        "1e39bfa85bb3ebe44afb6d627bdd3202550508d5224c837dc4370576f555cb18".into());
    contract.files.insert("session-model.mjs".into(),
        "d50926db8682babc44174201b0591e7b309e00e5f7f5989c924f610e7b1a7081".into());
}

/// Absence is explicit: callers must not advertise patched support based on a CLI name.
pub fn reviewed_agent_command(agent: &str) -> Result<Option<(String, Vec<String>)>, String> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if agent == "droid" {
        return droid::reviewed_command();
    }
    if linux_native_only(agent) {
        return Ok(None);
    }
    let Some(mut contract) = contract(agent) else { return Ok(None) };
    let mut directory = installed_directory(agent, &contract)?;
    if agent == "auggie"
        && !directory.try_exists().map_err(|_| "Cannot inspect Auggie managed runtime")?
    {
        use_auggie_v1_contract(&mut contract);
        directory = root(agent)?.join(format!("auggie-{}-model-source-v1", contract.version));
    }
    if agent == "openclaw" && !directory.try_exists().map_err(|_| "Cannot inspect OpenClaw install")? {
        contract.files.insert(OPENCLAW_SCANNER.into(), OPENCLAW_LEGACY_SCANNER.into());
        directory = root(agent)?.join(format!("openclaw-{}-model-source-v2", contract.version));
        if !directory.try_exists().map_err(|_| "Cannot inspect OpenClaw v2 install")? {
            contract.files.remove(OPENCLAW_MCP_TRANSPORT);
            directory = root(agent)?.join(format!("openclaw-{}-model-source-v1", contract.version));
        }
    }
    if agent == "vtcode" && !directory.try_exists().map_err(|_| "Cannot inspect VT Code managed runtime")? {
        let legacy = root(agent)?.join(format!("vtcode-{}-model-source-v2", contract.version));
        if legacy.try_exists().map_err(|_| "Cannot inspect VT Code v2 runtime")? {
            contract.files.insert(contract.bin.clone(), VTCODE_V2_SHA256.into());
            directory = legacy;
        } else {
            directory = original_vtcode_directory(&mut contract)?;
        }
    }
    // Keep the reviewed original available until the separate patched install succeeds.
    if agent == "droid"
        && !directory.try_exists().map_err(|_| "Cannot inspect Droid managed runtime")?
        && let Some(source_sha256) = &contract.source_sha256
    {
        contract.files.insert(contract.bin.clone(), source_sha256.clone());
        directory = root(agent)?.join(format!("droid-{}-model-source-v1", contract.version));
    }
    let path = verify(&directory, &contract)?;
    if agent == "glm-acp-agent" {
        // The client passes the reviewed entry directly to Node on every platform.
        return Ok(Some((path.to_string_lossy().into_owned(), vec![])));
    }
    if !contract.layout.is_empty() {
        return Ok(Some((
            path.to_string_lossy().into_owned(),
            match agent {
                "vibe" | "aionrs" => vec![],
                "harn" | "corust-agent" => vec![],
                "junie" => vec!["--acp=true".into()],
                "sigit" => vec!["--acp".into()],
                "cortex-code" | "trae" => vec!["acp".into(), "serve".into()],
                "droid" => vec!["exec".into(), "--output-format".into(), "acp".into()],
                _ => vec!["acp".into()],
            },
        )));
    }
    let args = match agent {
        "gemini" => vec!["--experimental-acp".into()],
        "iflow" => vec!["--experimental-acp".into()],
        "autohand" => vec!["--acp".into()],
        "auggie" | "codebuddy" | "dirac" | "qoder" | "qwen" | "snow" => vec!["--acp".into()],
        "copilot" => vec!["--acp".into(), "--stdio".into()],
        "kilocode" => vec!["acp".into(), "--pure".into()],
        "nova" | "opencode" | "openclaw" => vec!["acp".into()],
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
        directory.join("node_modules/.bin").join(bin_name);
    if executable.canonicalize().ok() != path.canonicalize().ok() || !executable.is_file() {
        return Err("ACP executable does not resolve to its reviewed artifact".into());
    }
    Ok(Some((executable.to_string_lossy().into_owned(), args)))
}

const OPENCLAW_SCANNER: &str = "dist/workspace-skill-sources-DqAcMM_z.mjs";
const OPENCLAW_LEGACY_SCANNER: &str = "db073f36c9ac2d94a9a51b5076bbe6e3ae780c67ec820e81ef000bef1bb83459";

const OPENCLAW_MCP_TRANSPORT: &str = "dist/mcp-stdio-transport-Dlawzoy1.mjs";

/// Restricted launches require the exact scanner and transport from the current contract.
pub fn verify_openclaw_restricted_executable(command: &str) -> Result<(), String> {
    let contract = contract("openclaw").ok_or("OpenClaw contract is unavailable")?;
    let executable = Path::new(command).canonicalize()
        .map_err(|_| "Missing restricted OpenClaw runtime")?;
    if executable.ancestors().any(|directory| {
        verify(directory, &contract)
            .is_ok_and(|path| path.canonicalize().ok().as_ref() == Some(&executable))
    }) {
        Ok(())
    } else {
        Err("Restricted OpenClaw requires the v3 managed runtime".into())
    }
}

/// Native stdio mounts require the transport with verified host-exit cleanup.
pub fn verify_openclaw_mcp_executable(command: &str) -> Result<(), String> {
    let contract = contract("openclaw").ok_or("OpenClaw contract is unavailable")?;
    let executable = Path::new(command)
        .canonicalize()
        .map_err(|_| "OpenClaw MCP requires the reviewed runtime")?;
    let package = executable.parent().ok_or("Invalid OpenClaw entry")?;
    let bytes = std::fs::read(package.join(OPENCLAW_MCP_TRANSPORT))
        .map_err(|_| "OpenClaw MCP requires the reviewed stdio lifecycle patch")?;
    if contract.files.get(OPENCLAW_MCP_TRANSPORT)
        != Some(&hex::encode(Sha256::digest(bytes)))
    {
        return Err("OpenClaw MCP requires the reviewed stdio lifecycle patch".into());
    }
    Ok(())
}

/// Restricted Auggie requires the current contract, with no v1 fallback.
pub fn verify_auggie_restricted_executable(command: &str) -> Result<(), String> {
    let contract = contract("auggie").ok_or("Missing Auggie runtime contract")?;
    let executable = Path::new(command).canonicalize().map_err(|_| "Missing Auggie executable")?;
    if executable.ancestors().any(|root| {
        verify(root, &contract).is_ok_and(|path| path.canonicalize().ok().as_ref() == Some(&executable))
    }) {
        Ok(())
    } else {
        Err("Restricted Auggie requires the reviewed skill runtime".into())
    }
}

pub fn verify_droid_restricted_executable(command: &str) -> Result<(), String> {
    let contract = contract("droid").ok_or("Missing Droid artifact contract")?;
    let executable = Path::new(command).canonicalize().map_err(|_| "Missing Droid executable")?;
    let digest = hex::encode(Sha256::digest(std::fs::read(&executable).map_err(|_| "Missing Droid executable")?));
    if cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && executable.file_name().is_some_and(|name| name == contract.bin.as_str())
        && contract.files.get(&contract.bin) == Some(&digest)
    {
        Ok(())
    } else {
        Err("Restricted Droid requires the reviewed patched executable".into())
    }
}

/// Validate explicit overrides too; a familiar executable name is never proof of a patch.
pub fn verify_reviewed_agent_executable(agent: &str, command: &str) -> Result<(), String> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if agent == "droid" {
        return droid::verify_command(command);
    }
    if linux_native_only(agent) {
        return Err("No reviewed managed ACP executable for this Linux agent".into());
    }
    if agent == "claude" {
        let command = command.trim();
        return if Path::new(command).is_absolute()
            && (acp_access_policy::access_policy::claude::verified_entry(command, &[])
                || acp_access_policy::access_policy::claude::verified_effort_entry(command))
        {
            Ok(())
        } else {
            Err("Claude managed launch requires an installed verified ACP adapter".into())
        };
    }
    let Some(mut contract) = contract(agent) else { return Ok(()) };
    let executable = Path::new(command)
        .canonicalize()
        .map_err(|_| "Reviewed ACP executable is unavailable; install it on the gateway host")?;
    if let Ok(Some((installed, _))) = reviewed_agent_command(agent)
        && Path::new(&installed).canonicalize().ok().as_ref() == Some(&executable)
    {
        return Ok(());
    }
    if agent == "amp-acp" && cfg!(target_os = "linux") {
        return Err("Amp Linux requires the installed reviewed v6 executable".into());
    }
    if agent == "iflow" && cfg!(target_os = "linux") {
        return Err("iFlow Linux requires the installed reviewed v4 executable".into());
    }
    if agent == "glm-acp-agent" {
        return Err("GLM executable must resolve to the installed reviewed entry".into());
    }
    if agent == "stakpak" {
        return Err("Stakpak requires the installed reviewed source build".into());
    }
    if agent == "harn" {
        return Err("Harn executable must resolve to the installed reviewed entry".into());
    }
    if agent == "vtcode" && verify_vtcode_legacy_path(&executable, &contract) {
        return Ok(());
    }
    if agent == "vtcode" && executable.file_name().is_some_and(|name| name == contract.bin.as_str())
        && let Some(source_sha256) = &contract.source_sha256
        && hex::encode(Sha256::digest(std::fs::read(&executable).map_err(|_| "Missing VT Code executable")?))
            == *source_sha256
    {
        return Ok(());
    }
    if contract.layout == "native" {
        if let Some(directory) = executable.parent()
            && verify(directory, &contract).is_ok_and(|path| path == executable)
        {
            return Ok(());
        }
        if agent == "goose"
            && hex::encode(Sha256::digest(
                std::fs::read(&executable).map_err(|_| "Cannot read native ACP artifact")?,
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
        if contract.package == "amp-acp" {
            for directory in executable.ancestors() {
                let entry =
                    directory.join("node_modules").join(&contract.package).join(&contract.bin);
                if entry.canonicalize().ok().as_ref() == Some(&executable) {
                    verify(directory, &contract)?;
                    return Ok(());
                }
            }
        }
        for directory in executable.ancestors() {
            if let Ok(path) = verify(directory, &contract)
                && path.canonicalize().ok().as_ref() == Some(&executable)
            {
                return Ok(());
            }
        }
    }
    if agent == "auggie" {
        use_auggie_v1_contract(&mut contract);
        if executable.ancestors().any(|root| {
            verify(root, &contract).is_ok_and(|path| path.canonicalize().ok().as_ref() == Some(&executable))
        }) {
            return Ok(());
        }
    }
    if agent == "openclaw" {
        contract.files.insert(OPENCLAW_SCANNER.into(), OPENCLAW_LEGACY_SCANNER.into());
        for legacy_without_mcp in [false, true] {
            if legacy_without_mcp {
                contract.files.remove(OPENCLAW_MCP_TRANSPORT);
            }
            if executable.ancestors().any(|directory| {
                verify(directory, &contract)
                    .is_ok_and(|path| path.canonicalize().ok().as_ref() == Some(&executable))
            }) {
                return Ok(());
            }
        }
    }
    Err("ACP executable is not the reviewed patched release; install the model-source runtime"
        .into())
}

#[cfg(not(target_arch = "wasm32"))]
struct Stage(PathBuf);
#[cfg(not(target_arch = "wasm32"))]
impl Stage {
    fn new(agent: &str, root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|_| "Cannot create ACP runtime root")?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Invalid installation clock")?
            .as_nanos();
        let path = root.join(format!(".install-{agent}-{}-{timestamp}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|_| "Cannot create ACP installation stage")?;
        Ok(Self(path))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Stage {
    fn drop(&mut self) {
        // Unpacking can leave a large tree. Do not hold the install request
        // and the gateway's install slot while removing it after cancellation.
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let path = std::mem::take(&mut self.0);
            runtime.spawn_blocking(move || {
                let _ = std::fs::remove_dir_all(path);
            });
        } else {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn install(agent: &str, log: &mut crate::install::InstallLog) -> Result<bool, String> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if agent == "droid" {
        droid::install(log).await?;
        return Ok(true);
    }
    if linux_native_only(agent) {
        #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            if agent == "junie" {
                junie::install(log).await?;
            } else {
                linux_cli::install(agent, log).await?;
            }
            return Ok(true);
        }
        #[cfg(not(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64"))))]
        return Err("No fixed ACP executable for this Linux target".into());
    }
    let Some(mut contract) = contract(agent) else { return Ok(false) };
    let amp_linux = agent == "amp-acp" && LINUX_GNU;
    if agent == "amp-acp" && !amp_linux && !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err("Amp reviewed local runtime is only verified on macOS arm64".into());
    }
    if agent == "deepagents" && cfg!(windows) {
        return Err("DeepAgents reviewed runtime is not yet supported on Windows".into());
    }
    let glm_platform = agent == "glm-acp-agent"
        && cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows"));
    let iflow_linux = agent == "iflow" && LINUX_GNU;
    if agent != "deepagents" && !glm_platform && !iflow_linux && !amp_linux && !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        // Preserve the existing native installer for CLI-local use on unverified hosts.
        return Ok(false);
    }
    log.step("检查托管运行时");
    let mut destination = installed_directory(agent, &contract)?;
    if destination.exists() {
        if agent == "glm-acp-agent" {
            reviewed_agent_command(agent)?;
        } else {
            verify(&destination, &contract)?;
        }
        return Ok(true);
    }
    let mut vtcode_binary = None;
    if agent == "vtcode" {
        let binary = std::env::current_exe()
            .map_err(|_| "Cannot locate packaged ShiJu binaries")?
            .with_file_name("vtcode-acp");
        match binary.symlink_metadata() {
            Ok(metadata) if metadata.is_file() => vtcode_binary = Some(binary),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                destination = original_vtcode_directory(&mut contract)?;
                if destination.exists() {
                    verify(&destination, &contract)?;
                    return Ok(true);
                }
            }
            _ => return Err("Packaged vtcode-acp is not a regular file".into()),
        }
    }
    log.step("准备托管安装目录和补丁");
    let stage = Stage::new(agent, &root(agent)?)?;
    let assets = stage.0.join("assets");
    for (name, text) in [
        ("autohand/apply.py", include_str!("../../../../patches/acp-model-source/autohand/apply.py")),
        ("autohand/session-boundary.mjs", include_str!("../../../../patches/acp-model-source/autohand/session-boundary.mjs")),
        ("autohand-lock.json", include_str!("../../../../patches/acp-model-source/autohand/package-lock.json")),
        ("dimcode/apply.py", include_str!("../../../../patches/acp-model-source/dimcode/apply.py")),
        ("nova/apply.py", include_str!("../../../../patches/acp-model-source/nova/apply.py")),
        ("nova/session-model.mjs", include_str!("../../../../patches/acp-model-source/nova/session-model.mjs")),
        ("nova/session-mcp.mjs", include_str!("../../../../patches/acp-model-source/nova/session-mcp.mjs")),
        ("install.py", include_str!("managed_runtime/install.py")),
        (
            "glm-acp-agent.patch",
            include_str!("../../../../patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-skills.patch"),
        ),
        ("glm-linux-contract.json", GLM_LINUX_CONTRACT),
        (
            "glm-linux-session-model.patch",
            include_str!("../../../../patches/acp-model-source/glm-acp-agent/glm-acp-agent-1.17.1-linux-host-mcp.patch"),
        ),
        ("droid/apply.py", include_str!("../../../../patches/acp-model-source/droid/apply.py")),
        ("cortex-code/apply.py", include_str!("../../../../patches/acp-model-source/cortex-code/apply.py")),
        ("cortex-code/launcher.py", include_str!("../../../../patches/acp-model-source/cortex-code/launcher.py")),
        ("cortex-code/mcp_bridge.py", include_str!("../../../../patches/acp-model-source/cortex-code/mcp_bridge.py")),
        ("mimo-code/stdio-only.patch", include_str!("../../../../patches/acp-model-source/mimo-code/stdio-only.patch")),
        ("junie/launcher.py", include_str!("../../../../patches/acp-model-source/junie/launcher.py")),
        ("junie/mcp_bridge.py", include_str!("../../../../patches/acp-model-source/junie/mcp_bridge.py")),
        ("harn/skills.patch", include_str!("../../../../patches/acp-model-source/harn/skills.patch")),
        ("harn/agent.harn", include_str!("../../../../patches/acp-model-source/harn/agent.harn")),
        ("poolside/launcher.py", include_str!("../../../../patches/acp-model-source/poolside/launcher.py")),
        ("poolside/mcp_bridge.py", include_str!("../../../../patches/acp-model-source/poolside/mcp_bridge.py")),
        ("harn/launcher.py", include_str!("../../../../patches/acp-model-source/harn/launcher.py")),
        ("corust-agent/launcher.py", include_str!("../../../../patches/acp-model-source/corust-agent/launcher.py")),
        ("corust-agent/mcp_bridge.py", include_str!("../../../../patches/acp-model-source/corust-agent/mcp_bridge.py")),
        ("corust-agent/mcp_graph.py", include_str!("../../../../patches/acp-model-source/corust-agent/mcp_graph.py")),
        ("stakpak/launcher.py", include_str!("../../../../patches/acp-model-source/stakpak/launcher.py")),
        ("stakpak/mcp.py", include_str!("../../../../patches/acp-model-source/stakpak/mcp.py")),
        ("stakpak/skills.patch", include_str!("../../../../patches/acp-model-source/stakpak/skills.patch")),
        (
            "aionrs.patch",
            include_str!("../../../../patches/acp-model-source/aionrs/read-only-skills.patch"),
        ),
        (
            "sigit.patch",
            include_str!("../../../../patches/acp-model-source/sigit/host-boundary.patch"),
        ),
        ("amp-acp/apply.py", include_str!("../../../../patches/acp-model-source/amp-acp/apply.py")),
        ("amp-acp/local.mjs", include_str!("../../../../patches/acp-model-source/amp-acp/local.mjs")),
        ("deepagents-lock.json", include_str!("managed_runtime/deepagents-lock.json")),
        ("deepagents/mcp.mjs", include_str!("../../../../patches/acp-model-source/deepagents/mcp.mjs")),
        ("deepagents/mcp.py", include_str!("../../../../patches/acp-model-source/deepagents/mcp.py")),
        (
            "deepagents/apply.py",
            include_str!("../../../../patches/acp-model-source/deepagents/apply.py"),
        ),
        (
            "deepagents/guard.mjs",
            include_str!("../../../../patches/acp-model-source/deepagents/guard.mjs"),
        ),
        (
            "deepagents/skills.mjs",
            include_str!("../../../../patches/acp-model-source/deepagents/skills.mjs"),
        ),
        ("trae/apply.py", include_str!("../../../../patches/acp-model-source/trae/apply.py")),
        ("trae/launcher.py", include_str!("../../../../patches/acp-model-source/trae/launcher.py")),
        ("cursor/apply.py", include_str!("../../../../patches/acp-model-source/cursor/apply.py")),
        ("cursor/patch.json", include_str!("../../../../patches/acp-model-source/cursor/patch.json")),
        (
            "cursor/session-model.cjs",
            include_str!("../../../../patches/acp-model-source/cursor/session-model.cjs"),
        ),
        ("qoder/apply.py", include_str!("../../../../patches/acp-model-source/qoder/apply.py")),
        ("qoder/patch.json", include_str!("../../../../patches/acp-model-source/qoder/patch.json")),
        (
            "qoder/session-model.mjs",
            include_str!("../../../../patches/acp-model-source/qoder/session-model.mjs"),
        ),
        ("auggie/apply.py", include_str!("../../../../patches/acp-model-source/auggie/apply.py")),
        ("auggie/patch.json", include_str!("../../../../patches/acp-model-source/auggie/patch.json")),
        (
            "auggie/session-model.mjs",
            include_str!("../../../../patches/acp-model-source/auggie/session-model.mjs"),
        ),
        ("openclaw/coexistence-candidate.json", include_str!("../../../../patches/acp-model-source/openclaw/coexistence-candidate.json")),
        ("openclaw/apply.py", include_str!("../../../../patches/acp-model-source/openclaw/apply.py")),
        (
            "openclaw/patch.json",
            include_str!("../../../../patches/acp-model-source/openclaw/patch.json"),
        ),
        (
            "opencode/apply-skills.py",
            include_str!("../../../../patches/acp-model-source/opencode/apply-skills.py"),
        ),
        (
            "opencode/skill-patch.json",
            include_str!("../../../../patches/acp-model-source/opencode/skill-patch.json"),
        ),
        ("kilocode/apply.py", include_str!("../../../../patches/acp-model-source/kilocode/apply.py")),
        (
            "kilocode/patch.json",
            include_str!("../../../../patches/acp-model-source/kilocode/patch.json"),
        ),
        ("dirac/apply.py", include_str!("../../../../patches/acp-model-source/dirac/apply.py")),
        (
            "dirac/session-mcp.mjs",
            include_str!("../../../../patches/acp-model-source/dirac/session-mcp.mjs"),
        ),
        ("snow/apply.py", include_str!("../../../../patches/acp-model-source/snow/apply.py")),
        ("snow/patch.json", include_str!("../../../../patches/acp-model-source/snow/patch.json")),
        ("copilot/apply.py", include_str!("../../../../patches/acp-model-source/copilot/apply.py")),
        (
            "copilot/patch.json",
            include_str!("../../../../patches/acp-model-source/copilot/patch.json"),
        ),
        ("kiro/apply.py", include_str!("../../../../patches/acp-model-source/kiro/apply.py")),
        ("kiro/patch.json", include_str!("../../../../patches/acp-model-source/kiro/patch.json")),
        ("kiro/launcher.py", include_str!("../../../../patches/acp-model-source/kiro/launcher.py")),
        ("hermes/coexistence/apply_patch.py", include_str!("../../../../docs/shiju/acp-model-source/evidence/hermes/apply_patch.py")),
        ("hermes/coexistence/patch.json", include_str!("../../../../patches/acp-model-source/hermes/patch.json")),
        ("hermes/coexistence/coexistence.patch", include_str!("../../../../patches/acp-model-source/hermes/coexistence.patch")),
        ("contracts.json", CONTRACTS),
        ("iflow-lock.json", include_str!("managed_runtime/iflow-lock.json")),
        (
            "vibe/apply_patch.py",
            include_str!("../../../../docs/shiju/acp-model-source/evidence/vibe/apply_patch.py"),
        ),
        (
            "vibe/upstream-hashes.json",
            include_str!("../../../../docs/shiju/acp-model-source/evidence/vibe/upstream-hashes.json"),
        ),
        (
            "vibe/mistral-vibe-3.0.4-session-binding.patch",
            include_str!(
                "../../../../docs/shiju/acp-model-source/evidence/vibe/mistral-vibe-3.0.4-session-binding.patch"
            ),
        ),
        (
            "hermes/apply_patch.py",
            include_str!("../../../../docs/shiju/acp-model-source/evidence/hermes/apply_patch.py"),
        ),
        (
            "hermes/patch.json",
            include_str!("../../../../docs/shiju/acp-model-source/evidence/hermes/patch.json"),
        ),
        (
            "hermes/hermes-agent-0.19.0-session-state.patch",
            include_str!(
                "../../../../docs/shiju/acp-model-source/evidence/hermes/hermes-agent-0.19.0-session-state.patch"
            ),
        ),
        (
            "goose.patch",
            include_str!(
                "../../../../docs/shiju/acp-model-source/evidence/goose/v1.57.0-model-lock-skill-coexistence.patch"
            ),
        ),
        (
            "codex.patch",
            include_str!(
                "../../../../patches/acp-model-source/codex/codex-acp-2.5.0-model-source.patch"
            ),
        ),
        (
            "gemini.patch",
            include_str!(
                "../../../../patches/acp-model-source/gemini/gemini-cli-0.66.0-history.patch"
            ),
        ),
        (
            "pi.patch",
            include_str!("../../../../patches/acp-model-source/pi/pi-acp-0.0.22-state-dir.patch"),
        ),
        ("pi/mcp-extension.mjs", include_str!("../../../../patches/acp-model-source/pi/mcp-extension.mjs")),
        ("iflow/apply.py", include_str!("../../../../patches/acp-model-source/iflow/apply.py")),
        (
            "codebuddy/apply.py",
            include_str!("../../../../patches/acp-model-source/codebuddy/apply.py"),
        ),
        (
            "codebuddy/patch.json",
            include_str!("../../../../patches/acp-model-source/codebuddy/patch.json"),
        ),
        ("qwen/apply.py", include_str!("../../../../patches/acp-model-source/qwen/apply.py")),
        ("qwen/patch.json", include_str!("../../../../patches/acp-model-source/qwen/patch.json")),
    ] {
        let path = assets.join(name);
        std::fs::create_dir_all(path.parent().ok_or("Invalid ACP asset path")?)
            .map_err(|_| "Cannot create ACP patch directory")?;
        std::fs::write(path, text).map_err(|_| "Cannot write embedded ACP patch")?;
    }
    if agent == "cortex-code" {
        let archive = root(agent)?
            .join("downloads")
            .join(format!("coco-{}-darwin-arm64.tar.gz", contract.version));
        if archive.symlink_metadata().is_ok_and(|metadata| metadata.is_file()) {
            std::fs::copy(archive, assets.join("cortex-code.tar.gz"))
                .map_err(|_| "Cannot stage cached Cortex Code archive")?;
        }
    }
    if let Some(binary) = vtcode_binary {
        std::fs::copy(binary, assets.join("vtcode-acp"))
            .map_err(|_| "Cannot stage packaged VT Code ACP runtime")?;
    }
    if agent == "aionrs" {
        let bridge = std::env::current_exe()
            .map_err(|_| "Cannot locate packaged ShiJu binaries")?
            .with_file_name("aionrs-acp");
        if !bridge.symlink_metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Err("Packaged aionrs-acp is missing beside the ShiJu executable".into());
        }
        std::fs::copy(bridge, assets.join("aionrs-acp"))
            .map_err(|_| "Cannot stage packaged aionrs bridge")?;
    }
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if iflow_linux {
        for (name, text) in [
            ("iflow/linux-artifacts.json", iflow::MANIFEST),
            ("iflow/apply-linux.py", include_str!("../../../../patches/acp-model-source/iflow/apply-linux-mcp.py")),
        ] {
            std::fs::write(assets.join(name), text).map_err(|_| "Cannot stage iFlow Linux assets")?;
        }
    }
    if amp_linux {
        for (name, text) in [
            ("amp-acp/linux-runtime.json", AMP_LINUX_CONTRACT),
            ("amp-acp/install_linux.py", include_str!("../../../../patches/acp-model-source/amp-acp/install_linux.py")),
        ] {
            std::fs::write(assets.join(name), text).map_err(|_| "Cannot stage Amp Linux assets")?;
        }
    }
    let runtime = stage.0.join("runtime");
    let uv = if matches!(agent, "vibe" | "hermes") {
        crate::install::ensure_uv(log).await?
    } else {
        String::new()
    };
    let script = assets.join("install.py");
    let python = if cfg!(windows) && agent == "glm-acp-agent" {
        "python.exe"
    } else {
        "python3"
    };
    crate::install::run_installer(
        log,
        "运行托管安装脚本",
        python,
        &[
            script.to_str().ok_or("Invalid installer path")?,
            assets.to_str().ok_or("Invalid asset path")?,
            runtime.to_str().ok_or("Invalid runtime path")?,
            agent,
            &uv,
            if agent == "glm-acp-agent" && LINUX_GNU { "--glm-linux-v4" } else { "" },
        ],
    )
    .await?;
    log.step("校验托管运行时");
    verify(&runtime, &contract)?;
    log.step("发布托管运行时");
    if std::fs::rename(&runtime, &destination).is_err()
        && (agent != "glm-acp-agent" || !destination.is_dir())
    {
        return Err("Cannot atomically publish reviewed ACP runtime".into());
    }
    reviewed_agent_command(agent)?;
    Ok(true)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "managed_runtime_tests.rs"]
mod tests;
