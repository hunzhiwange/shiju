//! Fixed Claude ACP 0.26.0 controls. Runtime identity and readback are separate gates.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub(crate) const FULL_MODE: &str = "bypassPermissions";
pub(crate) const RESTRICTED_MODE: &str = "default";

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
    use sha2::{Digest, Sha256};
    if !args.is_empty() {
        return false;
    }
    let Ok(entry) = std::path::Path::new(command).canonicalize() else {
        return false;
    };
    let Some(package) = entry.parent().and_then(std::path::Path::parent) else {
        return false;
    };
    if entry != package.join("dist/index.js") {
        return false;
    }
    let files = [
        ("package.json", "822baae8c24189370708779ece72fef22a741de2981a861bd4bc7ee034ed783d"),
        ("dist/index.js", "2859ccaa5ba91ed27b80614eae47399370015acbcbc7d01d1408c71f7964565d"),
        ("dist/lib.js", "0d80ee1628ae07d4dc7e17a1a59635ad491c6bacdd0d9b7601c3c2a13b5ca45f"),
        ("dist/acp-agent.js", "00e66a056cc39ba087dffe63da3b62d27fcff6694e6eee5fe3b25961e6218ddd"),
        ("dist/settings.js", "9bf60a23f0b9dbb4f159f6b6bbb9d334d6f0cb0d9ca5c9531d923635b56a35bb"),
        ("dist/tools.js", "e3fef100a075f727824cc37dbb9629089bab9760c1efc0f761dfe698f75be630"),
        ("dist/utils.js", "2a7cc008c0c0614420ec182e0c680a67a06efdfe3a78960ebebcb032326f06c6"),
    ];
    if !files.iter().all(|(name, hash)| {
        std::fs::read(package.join(name))
            .is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == *hash)
    }) {
        return false;
    }
    // Node resolves the nearest installed SDK. Never skip a mismatched nearer package.
    let Some(sdk) = entry
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("node_modules/@anthropic-ai/claude-agent-sdk"))
        .find(|dir| dir.exists())
    else {
        return false;
    };
    let files = [
        ("package.json", "c478cf463b38e9b289e5b16640d60a39b1d2a1528e94b1f7d2578c8754064d9c"),
        ("sdk.mjs", "d4b7aa175a37db7835d354fbdd8a6a0c03ad6a11ab32842d5362531b20ccd35b"),
        ("cli.js", "d4d06b148b05d3d21ad250a9900b7c8c47780088712299acd9892a8777f76ab4"),
    ];
    files.iter().all(|(name, hash)| {
        std::fs::read(sdk.join(name)).is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == *hash)
    })
}
