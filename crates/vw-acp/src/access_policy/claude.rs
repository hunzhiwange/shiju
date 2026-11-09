//! Fixed Claude ACP 0.26.0 controls. Runtime identity and readback are separate gates.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) const FULL_MODE: &str = "bypassPermissions";
pub(crate) const AUTO_MODE: &str = "auto";
pub(crate) const RESTRICTED_MODE: &str = "default";

// This SDK mode includes filesystem shell commands and may skip canUseTool.
pub(crate) const ACCEPT_EDITS_MODE: &str = "acceptEdits";

pub(crate) fn desktop_restricted_mode(value: &str) -> bool {
    matches!(value, RESTRICTED_MODE | "plan")
}

pub(crate) fn current_mode(options: Option<&[SessionConfigOption]>) -> Option<&str> {
    options?.iter().find_map(|option| match &option.kind {
        SessionConfigKind::Select(select) if option.id.0.as_ref() == "mode" => {
            Some(select.current_value.0.as_ref())
        }
        _ => None,
    })
}

pub(crate) fn advertises_mode(options: Option<&[SessionConfigOption]>, value: &str) -> bool {
    options.is_some_and(|options| {
        options.iter().any(|option| {
            if option.id.0.as_ref() != "mode" {
                return false;
            }
            match &option.kind {
                SessionConfigKind::Select(select) => match &select.options {
                    agent_client_protocol::SessionConfigSelectOptions::Ungrouped(options) => {
                        options.iter().any(|option| option.value.0.as_ref() == value)
                    }
                    _ => false,
                },
                _ => false,
            }
        })
    })
}

/// Verify the actual npm entry and SDK, rather than trusting agentInfo or an npx label.
pub(crate) fn verified_entry(command: &str, args: &[String]) -> bool {
    if !args.is_empty() {
        return false;
    }
    let Some(package) = adapter_package(command) else {
        return false;
    };
    let files = [
        ("package.json", "822baae8c24189370708779ece72fef22a741de2981a861bd4bc7ee034ed783d"),
        ("dist/index.js", "2859ccaa5ba91ed27b80614eae47399370015acbcbc7d01d1408c71f7964565d"),
        ("dist/lib.js", "0d80ee1628ae07d4dc7e17a1a59635ad491c6bacdd0d9b7601c3c2a13b5ca45f"),
        ("dist/acp-agent.js", "00e66a056cc39ba087dffe63da3b62d27fcff6694e6eee5fe3b25961e6218ddd"),
        ("dist/settings.js", "9bf60a23f0b9dbb4f159f6b6bbb9d334d6f0cb0d9ca5c9531d923635b56a35bb"),
        ("dist/tools.js", "e3fef100a075f727824cc37dbb9629089bab9760c1efc0f761dfe698f75be630"),
        ("dist/utils.js", "2a7cc008c0c0614420ec182e0c680a67a06efdfe3a78960ebebcb032326f06c6"),
    ];
    if !matches_hashes(&package, &files) {
        return false;
    }
    let Some(sdk) = node_dependency(&package.join("dist"), "@anthropic-ai/claude-agent-sdk") else {
        return false;
    };
    let files = [
        ("package.json", "c478cf463b38e9b289e5b16640d60a39b1d2a1528e94b1f7d2578c8754064d9c"),
        ("sdk.mjs", "d4b7aa175a37db7835d354fbdd8a6a0c03ad6a11ab32842d5362531b20ccd35b"),
        ("cli.js", "d4d06b148b05d3d21ad250a9900b7c8c47780088712299acd9892a8777f76ab4"),
    ];
    matches_hashes(&sdk, &files)
}

/// The native SDK engine is reviewed for the managed model route only.
pub(crate) fn verified_effort_entry(command: &str) -> bool {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return false;
    }
    let Some(package) = adapter_package(command) else {
        return false;
    };
    let files = [
        ("package.json", "fa3ff61211b3af3de18b57cf11575c1c4da823af99d154aae454c68e248e1ed5"),
        ("dist/index.js", "0aa4c4255951db9614d2867df5e905e23167bb581d1391da50995c6491262239"),
        ("dist/lib.js", "82ed7ebb6256eda8d27db24a5fcfbf0f8855c93abb0384708f5d029c2c03b8e2"),
        ("dist/acp-agent.js", "1991036d67914038fee7526c2de89e4413cffaf7a1ffb705f2f03a1f8a65d1d8"),
        ("dist/settings.js", "f46dc3041f4d658b1d3363c19e398605c407a3e66f7edf49ac016e5965176d26"),
        ("dist/tools.js", "e4bb209c6cf52319b3c8af6c2e3bc1c2a165a728fa3406b9c1136949d9fa3bb3"),
        ("dist/utils.js", "2a7cc008c0c0614420ec182e0c680a67a06efdfe3a78960ebebcb032326f06c6"),
    ];
    if !matches_hashes(&package, &files) {
        return false;
    }
    let Some(sdk) = node_dependency(&package.join("dist"), "@anthropic-ai/claude-agent-sdk")
        .and_then(|path| path.canonicalize().ok())
    else {
        return false;
    };
    let files = [
        ("package.json", "2db10595bb1769d0bc0250cfdef1d3200b0391e1b7b219d4df69d537a6ec9a55"),
        ("sdk.mjs", "cabbcc5ba1703b6e31c04051a78875fc778795e88dbddb84f2b98ff4f3cf4d16"),
        ("manifest.json", "cc7ca0fd459d886212e363d4ad439f6c61e26d30fb00bc9bfc4ada9228fc0676"),
    ];
    if !matches_hashes(&sdk, &files) {
        return false;
    }
    let Some(engine) = node_dependency(&sdk, "@anthropic-ai/claude-agent-sdk-darwin-arm64") else {
        return false;
    };
    matches_hashes(
        &engine,
        &[
            ("package.json", "fdc185e90bf9f869b3e5d8ab07f2082eac6811aacf2316df5244430c9041b59b"),
            ("claude", "31db3444309d5d0f8b85e8782e2dcd86f31f7e48c1a1e83d69b09268c7b4f9a2"),
        ],
    )
}

fn adapter_package(command: &str) -> Option<PathBuf> {
    let entry = Path::new(command).canonicalize().ok()?;
    let package = entry.parent()?.parent()?;
    (entry == package.join("dist/index.js")).then(|| package.to_path_buf())
}

fn node_dependency(from: &Path, package: &str) -> Option<PathBuf> {
    // Never skip a nearer dependency with a different identity.
    from.ancestors().map(|dir| dir.join("node_modules").join(package)).find(|dir| dir.exists())
}

fn matches_hashes(directory: &Path, files: &[(&str, &str)]) -> bool {
    files.iter().all(|(name, hash)| {
        let path = directory.join(name);
        if !path.symlink_metadata().is_ok_and(|metadata| metadata.is_file()) {
            return false;
        }
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 16 * 1024];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => return hex::encode(digest.finalize()) == *hash,
                Ok(size) => digest.update(&buffer[..size]),
                Err(_) => return false,
            }
        }
    })
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "claude_fallback_acceptance.rs"]
mod fallback_acceptance;
