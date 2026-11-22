//! Fixed Claude ACP runtime identities and permission controls.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const FULL_MODE: &str = "bypassPermissions";
pub const AUTO_MODE: &str = "auto";
pub const RESTRICTED_MODE: &str = "default";

// This SDK mode includes filesystem shell commands and may skip canUseTool.
pub const ACCEPT_EDITS_MODE: &str = "acceptEdits";

pub fn desktop_restricted_mode(value: &str) -> bool {
    matches!(value, RESTRICTED_MODE | "plan")
}

pub fn current_mode(options: Option<&[SessionConfigOption]>) -> Option<&str> {
    options?.iter().find_map(|option| match &option.kind {
        SessionConfigKind::Select(select) if option.id.0.as_ref() == "mode" => {
            Some(select.current_value.0.as_ref())
        }
        _ => None,
    })
}

pub fn advertises_mode(options: Option<&[SessionConfigOption]>, value: &str) -> bool {
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
pub fn verified_entry(command: &str, args: &[String]) -> bool {
    args.is_empty() && verified_legacy_engine(command).is_some()
}

/// Verify the bundled JavaScript engine for a managed Linux launch.
#[cfg(target_os = "linux")]
pub fn verified_linux_legacy_engine(command: &str) -> Option<PathBuf> {
    if !Path::new(command).is_absolute() {
        return None;
    }
    verified_legacy_engine(command)
}

fn verified_legacy_engine(command: &str) -> Option<PathBuf> {
    let package = adapter_package(command)?;
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
        return None;
    }
    let sdk = node_dependency(&package.join("dist"), "@anthropic-ai/claude-agent-sdk")?;
    let files = [
        ("package.json", "c478cf463b38e9b289e5b16640d60a39b1d2a1528e94b1f7d2578c8754064d9c"),
        ("sdk.mjs", "d4b7aa175a37db7835d354fbdd8a6a0c03ad6a11ab32842d5362531b20ccd35b"),
        ("cli.js", "d4d06b148b05d3d21ad250a9900b7c8c47780088712299acd9892a8777f76ab4"),
    ];
    #[cfg(target_os = "linux")]
    let sdk = {
        let arch = linux_gnu_npm_arch()?;
        let sdk = sdk.canonicalize().ok()?;
        // SDK 0.2.96 tries the platform package before its bundled cli.js.
        let engine = format!("@anthropic-ai/claude-agent-sdk-linux-{arch}");
        if node_dependency(&sdk, &engine).is_some()
            || node_dependency(&sdk, &format!("{engine}-musl")).is_some()
        {
            return None;
        }
        sdk
    };
    matches_hashes(&sdk, &files).then(|| sdk.join("cli.js"))
}

/// The native SDK engine is reviewed for the managed model route only.
pub fn verified_effort_entry(command: &str) -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && verified_effort_engine(command).is_some()
}

/// Verify the engine that a managed Linux launch must pin before starting Node.
#[cfg(target_os = "linux")]
pub fn verified_linux_effort_engine(command: &str) -> Option<PathBuf> {
    if !Path::new(command).is_absolute() {
        return None;
    }
    verified_effort_engine(command)?.canonicalize().ok()
}

fn verified_effort_engine(command: &str) -> Option<PathBuf> {
    let (engine_package, manifest_hash, executable_hash) = effort_engine()?;
    let package = adapter_package(command)?;
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
        return None;
    }
    let sdk =
        node_dependency(&package.join("dist"), "@anthropic-ai/claude-agent-sdk")?.canonicalize().ok()?;
    let files = [
        ("package.json", "2db10595bb1769d0bc0250cfdef1d3200b0391e1b7b219d4df69d537a6ec9a55"),
        ("sdk.mjs", "cabbcc5ba1703b6e31c04051a78875fc778795e88dbddb84f2b98ff4f3cf4d16"),
        ("manifest.json", "cc7ca0fd459d886212e363d4ad439f6c61e26d30fb00bc9bfc4ada9228fc0676"),
    ];
    if !matches_hashes(&sdk, &files) {
        return None;
    }
    // The SDK tries the musl engine first on Linux even on a glibc host.
    if cfg!(target_os = "linux")
        && node_dependency(&sdk, &format!("{engine_package}-musl")).is_some()
    {
        return None;
    }
    let engine = node_dependency(&sdk, engine_package)?;
    matches_hashes(&engine, &[("package.json", manifest_hash), ("claude", executable_hash)])
        .then(|| engine.join("claude"))
}

#[cfg(target_os = "linux")]
fn linux_gnu_npm_arch() -> Option<&'static str> {
    if !cfg!(target_env = "gnu") {
        return None;
    }
    match std::env::consts::ARCH {
        "x86_64" => Some("x64"),
        "aarch64" => Some("arm64"),
        _ => None,
    }
}

fn effort_engine() -> Option<(&'static str, &'static str, &'static str)> {
    #[cfg(target_os = "linux")]
    return match linux_gnu_npm_arch()? {
        "x64" => Some((
            "@anthropic-ai/claude-agent-sdk-linux-x64",
            "0ba1df712e964d143187d6260a2c00ccb3233295cd7768d02629accc654ead30",
            "cca43053f062949495596b11b6fd1b59cf79102adb13bacbe66997e6fae41e4a",
        )),
        "arm64" => Some((
            "@anthropic-ai/claude-agent-sdk-linux-arm64",
            "ddde7d3fdaff312832f04e3dff32528c7a7acf38d3c0597d85cb4c6cb1b8d570",
            "382aa73ea4b07fd8d698e3159b5ef9e1b8739fae7505ba8ddd28b8a6a62819ce",
        )),
        _ => None,
    };
    #[cfg(not(target_os = "linux"))]
    cfg!(all(target_os = "macos", target_arch = "aarch64")).then_some((
        "@anthropic-ai/claude-agent-sdk-darwin-arm64",
        "fdc185e90bf9f869b3e5d8ab07f2082eac6811aacf2316df5244430c9041b59b",
        "31db3444309d5d0f8b85e8782e2dcd86f31f7e48c1a1e83d69b09268c7b4f9a2",
    ))
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
