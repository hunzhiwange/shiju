//! Fixed Qwen Code 0.26.7 controls. Runtime identity and readback are separate gates.
use agent_client_protocol::{SessionConfigKind, SessionConfigOption};

pub(crate) const FULL_MODE: &str = "yolo";
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

/// Hash the fixed npm package, including permission implementation chunks.
pub(crate) fn verified_entry(command: &str, args: &[String]) -> bool {
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
    let Ok(entry) = Path::new(command).canonicalize() else {
        return false;
    };
    if entry.file_name().and_then(|x| x.to_str()) != Some("cli-entry.js") {
        return false;
    }
    let Some(package) = entry.parent() else {
        return false;
    };
    let mut files = Vec::new();
    if collect(package, &mut files).is_err() {
        return false;
    }
    files.sort();
    let mut digest = Sha256::new();
    for file in files {
        let Ok(name) = file.strip_prefix(package) else {
            return false;
        };
        let Some(name) = name.to_str() else {
            return false;
        };
        let Ok(bytes) = std::fs::read(&file) else {
            return false;
        };
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(Sha256::digest(bytes));
    }
    let fingerprint = hex::encode(digest.finalize());
    fingerprint == "d34ed30c3b0647300e549f9f7e947e0ab4110307e55508ae77220c3b4e1577ca"
        || (!managed
            && fingerprint == "bb52c5bcfc0c461f69dc66799856d0e4b246d47449cecb2fb55a8c0e692272e8")
}
