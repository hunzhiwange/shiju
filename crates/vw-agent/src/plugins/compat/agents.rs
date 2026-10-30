//! Skill-local agent metadata never creates delegated agents or grants tools.

use crate::skills::{
    SkillAgentDependency, SkillAgentInterface, SkillAgentMetadata, SkillAgentPolicy,
};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

#[derive(Default)]
pub(crate) struct LoadedAgentMetadata {
    pub metadata: Option<SkillAgentMetadata>,
    pub diagnostics: Vec<String>,
}

pub(crate) fn load_metadata(plugin_root: &Path, skill_dir: &Path) -> LoadedAgentMetadata {
    let path = skill_dir.join("agents/openai.yaml");
    let mut loaded = LoadedAgentMetadata::default();
    let relative =
        match path.strip_prefix(plugin_root).ok().and_then(|relative| {
            relative.iter().map(|part| part.to_str()).collect::<Option<Vec<_>>>()
        }) {
            Some(relative) => format!("./{}", relative.join("/")),
            None => {
                loaded.diagnostics.push("agent metadata path is outside the plugin root".into());
                return loaded;
            }
        };
    // Check every component before even probing the optional file, including broken symlinks.
    if !safe_components(plugin_root, path.strip_prefix(plugin_root).unwrap()) {
        loaded.diagnostics.push("ignoring unsafe agent metadata path".into());
        return loaded;
    }
    if !path.exists() {
        return loaded;
    }
    let path = match super::super::audit::resolve_component_path(
        plugin_root,
        "skill agent metadata",
        &relative,
    ) {
        Ok(path) if path.canonical_path.is_file() => path.canonical_path,
        _ => {
            loaded.diagnostics.push("ignoring invalid agent metadata file".into());
            return loaded;
        }
    };
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => {
            loaded.diagnostics.push("ignoring unreadable agent metadata file".into());
            return loaded;
        }
    };
    let raw: RawMetadata = match serde_yaml::from_str(&contents) {
        Ok(raw) => raw,
        Err(_) => {
            // YAML errors may quote file contents; report no untrusted payload.
            loaded.diagnostics.push("ignoring malformed optional agent metadata".into());
            return loaded;
        }
    };
    let interface = raw.interface.and_then(|raw| {
        let declared_fields = [
            raw.display_name.is_some(),
            raw.short_description.is_some(),
            raw.icon_small.is_some(),
            raw.icon_large.is_some(),
            raw.brand_color.is_some(),
            raw.default_prompt.is_some(),
        ];
        let interface = SkillAgentInterface {
            display_name: normalize(raw.display_name, 64),
            short_description: normalize(raw.short_description, 1024),
            default_prompt: normalize(raw.default_prompt, 1024),
            icon_small: resolve_asset(plugin_root, skill_dir, raw.icon_small),
            icon_large: resolve_asset(plugin_root, skill_dir, raw.icon_large),
            brand_color: raw.brand_color.and_then(|value| {
                let value = value.trim();
                (value.len() == 7
                    && value.starts_with('#')
                    && value[1..].chars().all(|character| character.is_ascii_hexdigit()))
                .then(|| value.to_string())
            }),
        };
        let resolved_fields = [
            interface.display_name.is_some(),
            interface.short_description.is_some(),
            interface.icon_small.is_some(),
            interface.icon_large.is_some(),
            interface.brand_color.is_some(),
            interface.default_prompt.is_some(),
        ];
        if declared_fields
            .into_iter()
            .zip(resolved_fields)
            .any(|(declared, valid)| declared && !valid)
        {
            loaded.diagnostics.push("ignoring invalid optional agent interface fields".into());
        }
        (interface != SkillAgentInterface::default()).then_some(interface)
    });
    let dependencies = raw
        .dependencies
        .into_iter()
        .flat_map(|dependencies| dependencies.tools)
        .filter_map(|tool| {
            let kind = normalize(tool.kind, 64);
            let value = normalize(tool.value, 1024);
            let (Some(kind), Some(value)) = (kind, value) else {
                loaded.diagnostics.push("ignoring invalid individual agent dependency".into());
                return None;
            };
            Some(SkillAgentDependency {
                r#type: kind,
                value,
                description: normalize(tool.description, 1024),
                transport: normalize(tool.transport, 64),
                command: normalize(tool.command, 1024),
                url: normalize(tool.url, 1024),
                oauth_callback_port: tool.oauth.and_then(|oauth| oauth.callback_port),
            })
        })
        .collect();
    let policy = raw.policy.map(|policy| SkillAgentPolicy {
        allow_implicit_invocation: policy.allow_implicit_invocation,
        products: policy.products.into_iter().map(|product| product.as_str().to_string()).collect(),
    });
    let metadata = SkillAgentMetadata { interface, dependencies, policy };
    if metadata != SkillAgentMetadata::default() {
        loaded.metadata = Some(metadata);
    }
    loaded
}

fn normalize(value: Option<String>, max_chars: usize) -> Option<String> {
    let value = value?.split_whitespace().collect::<Vec<_>>().join(" ");
    (!value.is_empty() && value.chars().count() <= max_chars).then_some(value)
}

fn resolve_asset(plugin_root: &Path, skill_dir: &Path, raw: Option<PathBuf>) -> Option<PathBuf> {
    let raw = raw?;
    let text = raw.to_str()?;
    if text.is_empty() || text.contains('\\') || raw.is_absolute() {
        return None;
    }
    let has_parent = raw.components().any(|component| component == Component::ParentDir);
    let mut resolved = skill_dir.to_path_buf();
    for component in raw.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() || !resolved.starts_with(plugin_root) {
                    return None;
                }
            }
            Component::Normal(part) => {
                // A Windows drive is not accepted as a POSIX resource name either.
                if part.to_str()?.contains(':') {
                    return None;
                }
                resolved.push(part);
                if std::fs::symlink_metadata(&resolved)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return None;
                }
            }
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    let allowed_root =
        if has_parent { plugin_root.join("assets") } else { skill_dir.join("assets") };
    if !resolved.starts_with(allowed_root) {
        return None;
    }
    let relative = resolved.strip_prefix(plugin_root).ok()?;
    safe_components(plugin_root, relative).then_some(resolved)
}

fn safe_components(root: &Path, relative: &Path) -> bool {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                current.push(part);
                match std::fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() => return false,
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return false,
                }
            }
            _ => return false,
        }
    }
    true
}

#[derive(Default, Deserialize)]
struct RawMetadata {
    interface: Option<RawInterface>,
    dependencies: Option<RawDependencies>,
    policy: Option<RawPolicy>,
}

#[derive(Default, Deserialize)]
struct RawInterface {
    display_name: Option<String>,
    short_description: Option<String>,
    icon_small: Option<PathBuf>,
    icon_large: Option<PathBuf>,
    brand_color: Option<String>,
    default_prompt: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawDependencies {
    #[serde(default)]
    tools: Vec<RawDependency>,
}

#[derive(Default, Deserialize)]
struct RawDependency {
    #[serde(rename = "type")]
    kind: Option<String>,
    value: Option<String>,
    description: Option<String>,
    transport: Option<String>,
    command: Option<String>,
    url: Option<String>,
    oauth: Option<RawOAuth>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOAuth {
    #[serde(alias = "callback_port")]
    callback_port: Option<u16>,
}

#[derive(Deserialize)]
struct RawPolicy {
    allow_implicit_invocation: Option<bool>,
    #[serde(default)]
    products: Vec<Product>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Product {
    #[serde(alias = "CHATGPT")]
    Chatgpt,
    #[serde(alias = "CODEX")]
    Codex,
    #[serde(alias = "ATLAS")]
    Atlas,
}

impl Product {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Chatgpt => "chatgpt",
            Self::Codex => "codex",
            Self::Atlas => "atlas",
        }
    }
}
