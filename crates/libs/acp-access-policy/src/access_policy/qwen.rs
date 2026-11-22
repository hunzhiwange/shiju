//! Fixed Qwen Code 0.26.7 controls. Runtime identity and readback are separate gates.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub const FULL_MODE: &str = "yolo";
pub const RESTRICTED_MODE: &str = "default";

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

/// Hash the fixed npm package, including permission implementation chunks.
pub fn verified_entry(command: &str, args: &[String]) -> bool {
    let (base, managed) =
        if let Some(index) = args.iter().position(|arg| arg == "--shiju-model-source") {
            let suffix = &args[index..];
            if suffix.len() != 7
                || suffix[1] != "--auth-type"
                || suffix[2] != "openai"
                || suffix[3] != "--model"
                || suffix[5] != "--openai-base-url"
            {
                return false;
            }
            (&args[..index], true)
        } else {
            (args, false)
        };
    if base != ["--acp"] && base != ["--acp", "--experimental-skills"] {
        return false;
    }
    if managed
        && cfg!(all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))
    {
        return verified_linux_package(command);
    }
    package_fingerprint(command).is_some_and(|fingerprint| {
        fingerprint == "d34ed30c3b0647300e549f9f7e947e0ab4110307e55508ae77220c3b4e1577ca"
            || (!managed
                && fingerprint
                    == "bb52c5bcfc0c461f69dc66799856d0e4b246d47449cecb2fb55a8c0e692272e8")
    })
}

/// Check the exact patched Linux tree, independent of the launch arguments.
pub fn verified_linux_package(command: &str) -> bool {
    // The Linux overlay requires the private host tool guard before initialization.
    package_fingerprint(command).as_deref()
        == Some("2dd558a0d57266248f4f9ab4412066dfed2773965fd70b9cc461487c4a06313d")
}

fn package_fingerprint(command: &str) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};
    fn collect(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(std::io::Error::other("Unexpected package symlink"));
            }
            if kind.is_dir() {
                collect(&entry.path(), files)?;
            } else if matches!(
                entry.path().extension().and_then(|x| x.to_str()),
                Some("js" | "json")
            ) {
                files.push(entry.path());
            }
        }
        Ok(())
    }
    let entry = Path::new(command).canonicalize().ok()?;
    if entry.file_name()?.to_str()? != "cli-entry.js" {
        return None;
    }
    let package = entry.parent()?;
    let mut files = Vec::new();
    collect(package, &mut files).ok()?;
    files.sort();
    let mut digest = Sha256::new();
    for file in files {
        let name = file.strip_prefix(package).ok()?.to_str()?;
        let bytes = std::fs::read(&file).ok()?;
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(Sha256::digest(bytes));
    }
    Some(hex::encode(digest.finalize()))
}
