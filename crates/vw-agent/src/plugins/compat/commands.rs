//! Legacy command templates become audited skills without writing into the package.

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use super::skill_roots::{SkillPackageFormat, package_format};
use crate::config::Config;
use crate::plugins::DiscoveredPlugin;
use crate::plugins::audit::resolve_component_path;
use crate::skills::{PluginSkillSource, Skill, SkillLoadMode, load_skill_directory};

#[derive(Default)]
pub(crate) struct CommandInventory {
    pub skills: Vec<Skill>,
    pub diagnostics: Vec<(PathBuf, String)>,
}

pub(crate) fn load_commands(
    plugin: &DiscoveredPlugin,
    config: &Config,
    mode: SkillLoadMode,
) -> CommandInventory {
    let mut inventory = CommandInventory::default();
    if package_format(&plugin.manifest) != SkillPackageFormat::Legacy {
        return inventory;
    }
    let roots = match command_roots(plugin) {
        Ok(roots) => roots,
        Err(error) => {
            inventory
                .diagnostics
                .push((plugin.manifest.manifest_path.clone(), format!("{error:#}")));
            return inventory;
        }
    };
    let mut sources = BTreeMap::<String, BTreeMap<PathBuf, String>>::new();
    for root in roots {
        let walker = walkdir::WalkDir::new(&root).follow_links(false).sort_by_file_name();
        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    inventory.diagnostics.push((
                        error.path().unwrap_or(&root).to_path_buf(),
                        "failed to inspect plugin command entry".into(),
                    ));
                    continue;
                }
            };
            let path = entry.path();
            if entry.file_type().is_symlink() {
                inventory
                    .diagnostics
                    .push((path.to_path_buf(), "plugin command path must not be a symlink".into()));
                continue;
            }
            if !entry.file_type().is_file()
                || path.extension().and_then(|ext| ext.to_str()) != Some("md")
            {
                continue;
            }
            match command_source(plugin, &root, path, config) {
                Ok((name, rendered)) => {
                    sources.entry(name).or_default().insert(path.to_path_buf(), rendered);
                }
                Err(error) => {
                    inventory.diagnostics.push((path.to_path_buf(), format!("{error:#}")))
                }
            }
        }
    }
    for (name, candidates) in sources {
        // Codex drops every distinct source in a slug collision, rather than choosing a winner.
        if candidates.len() != 1 {
            inventory.diagnostics.extend(
                candidates
                    .into_keys()
                    .map(|path| (path, format!("duplicate migrated command name {name}"))),
            );
            continue;
        }
        let (source, rendered) = candidates.into_iter().next().expect("nonempty command group");
        match load_cached_skill(plugin, &source, &name, &rendered, config, mode) {
            Ok(Some(skill)) => inventory.skills.push(skill),
            Ok(None) => {}
            Err(error) => inventory.diagnostics.push((source, format!("{error:#}"))),
        }
    }
    inventory
}

fn command_roots(plugin: &DiscoveredPlugin) -> Result<Vec<PathBuf>> {
    let manifest: Value = serde_json::from_slice(&fs::read(&plugin.manifest.manifest_path)?)
        .context("failed to read plugin command declaration")?;
    let paths = match manifest.get("commands") {
        None | Some(Value::Null) => {
            match fs::symlink_metadata(plugin.manifest.root.join("commands")) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(error) => return Err(error).context("failed to inspect default command root"),
                Ok(_) => vec!["./commands"],
            }
        }
        Some(Value::String(path)) => vec![path.as_str()],
        Some(Value::Array(paths)) if paths.iter().all(Value::is_string) => {
            paths.iter().map(|path| path.as_str().expect("string checked")).collect()
        }
        Some(_) => bail!("ignoring commands: expected a string or string array"),
    };
    paths
        .into_iter()
        .map(|raw| {
            Ok(resolve_component_path(&plugin.manifest.root, "commands", raw)?.canonical_path)
        })
        .collect()
}

fn command_source(
    plugin: &DiscoveredPlugin,
    root: &Path,
    source: &Path,
    config: &Config,
) -> Result<(String, String)> {
    let relative = source.strip_prefix(&plugin.manifest.root)?;
    let raw = relative
        .iter()
        .map(|part| part.to_str().context("command path must be UTF-8"))
        .collect::<Result<Vec<_>>>()?
        .join("/");
    resolve_component_path(&plugin.manifest.root, "command entry", &format!("./{raw}"))?;
    ensure!(
        source.file_stem().and_then(|stem| stem.to_str()) != Some("README"),
        "README is not a command"
    );
    let report = crate::skills::audit_open_skill_markdown_with_security(
        source,
        &plugin.manifest.root,
        &config.skills.security,
    )?;
    ensure!(report.is_clean(), "insecure command template: {}", report.summary());
    let content = fs::read_to_string(source).context("failed to read command template")?;
    let (description, body) =
        parse_command(&content).context("command requires a nonempty frontmatter description")?;
    ensure!(!unsupported_template(body), "unsupported command template syntax");
    let source_name = if root.is_file() {
        source
            .file_stem()
            .and_then(|name| name.to_str())
            .context("command name must be UTF-8")?
            .to_owned()
    } else {
        source
            .strip_prefix(root)?
            .with_extension("")
            .iter()
            .map(|part| part.to_str().context("command name must be UTF-8").map(ToOwned::to_owned))
            .collect::<Result<Vec<_>>>()?
            .join("-")
    };
    let name = slug(&format!("source-command-{source_name}"));
    ensure!(name.len() <= 64, "migrated command name exceeds 64 characters");
    let body = rewrite_document_name(body.trim());
    let body = if body.is_empty() { "No command template body was found." } else { &body };
    let rendered = format!(
        "---\nname: {}\ndescription: {}\n---\n\n# {name}\n\nUse this skill when the user asks to run the migrated source command `{source_name}`.\n\n## Command Template\n\n{body}\n",
        yaml_string(&name),
        yaml_string(&rewrite_document_name(&description))
    );
    Ok((name, rendered))
}

fn parse_command(content: &str) -> Option<(String, &str)> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let (end, body_start) =
        ["\r\n---\r\n", "\r\n---\n", "\n---\r\n", "\n---\n", "\r\n---", "\n---"]
            .into_iter()
            .filter_map(|delimiter| rest.find(delimiter).map(|end| (end, end + delimiter.len())))
            .min_by_key(|(end, _)| *end)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&rest[..end]).ok()?;
    let description = value.as_mapping()?.iter().find_map(|(key, value)| {
        if key.as_str()?.trim() != "description" {
            return None;
        }
        match value {
            serde_yaml::Value::String(value) => Some(value.trim().to_owned()),
            serde_yaml::Value::Bool(value) => Some(value.to_string()),
            serde_yaml::Value::Number(value) => Some(value.to_string()),
            _ => None,
        }
    })?;
    (!description.is_empty()).then_some((description, &rest[body_start..]))
}

fn unsupported_template(body: &str) -> bool {
    body.contains("$ARGUMENTS")
        || body.as_bytes().windows(2).any(|pair| pair[0] == b'$' && pair[1].is_ascii_digit())
        || (body.contains("{{") && body.contains("}}"))
        || body.contains("!`")
        || body.contains("! `")
        || body
            .split_whitespace()
            .any(|token| token.strip_prefix('@').is_some_and(|rest| !rest.is_empty()))
}

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}

// The plugin profile only normalizes the document filename; it has no brand replacements.
fn rewrite_document_name(value: &str) -> String {
    let mut result = String::new();
    let mut start = 0;
    for (index, _) in value.char_indices() {
        let end = index + "AGENTS.md".len();
        if !value.get(index..end).is_some_and(|part| part.eq_ignore_ascii_case("AGENTS.md")) {
            continue;
        }
        let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
        if (index == 0 || !word(value.as_bytes()[index - 1]))
            && (end == value.len() || !word(value.as_bytes()[end]))
        {
            result.push_str(&value[start..index]);
            result.push_str("AGENTS.md");
            start = end;
        }
    }
    result.push_str(&value[start..]);
    result
}

fn load_cached_skill(
    plugin: &DiscoveredPlugin,
    source: &Path,
    name: &str,
    rendered: &str,
    config: &Config,
    mode: SkillLoadMode,
) -> Result<Option<Skill>> {
    ensure!(rendered.len() <= 4_000, "migrated command exceeds 4000 bytes");
    let base = config
        .config_path
        .parent()
        .context("plugin command cache needs a host config directory")?;
    reject_package_cache(base, &plugin.manifest.root)?;
    fs::create_dir_all(base).context("failed to create host command cache parent")?;
    let mut directory = base.canonicalize()?;
    ensure!(
        !directory.starts_with(&plugin.manifest.root),
        "command cache must stay outside original package"
    );
    let key = hex::encode(Sha256::digest(serde_json::to_vec(&(
        plugin.identity.as_key(),
        &plugin.manifest.root,
        source,
        rendered,
    ))?));
    for child in ["cache", "plugin-commands", &key, name] {
        directory.push(child);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error).context("failed to create command cache directory"),
        }
        let metadata = fs::symlink_metadata(&directory)?;
        ensure!(
            !metadata.file_type().is_symlink() && metadata.is_dir(),
            "command cache directory must not be a symlink or file"
        );
    }
    let entry = directory.join("SKILL.md");
    match fs::symlink_metadata(&entry) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
            temporary.write_all(rendered.as_bytes())?;
            match temporary.persist_noclobber(&entry) {
                Ok(_) => {}
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(error.error).context("failed to publish command skill cache");
                }
            }
        }
        Err(error) => return Err(error).context("failed to inspect cached command skill"),
    }
    let metadata = fs::symlink_metadata(&entry)?;
    ensure!(
        !metadata.file_type().is_symlink() && metadata.is_file(),
        "cached command skill must be a regular file"
    );
    ensure!(fs::read(&entry)? == rendered.as_bytes(), "cached command skill contents changed");
    let Some(mut skill) = load_skill_directory(&directory, mode, &config.skills.security, true)?
    else {
        return Ok(None);
    };
    // The location is derived; provenance points to the original audited command, never the cache.
    skill.plugin_source = Some(PluginSkillSource {
        plugin_id: plugin.identity.as_key(),
        namespace: plugin.manifest.name.clone(),
        root: plugin.manifest.root.clone(),
        relative_skill_path: source.strip_prefix(&plugin.manifest.root)?.to_path_buf(),
    });
    Ok(Some(skill))
}

// Validate the nearest existing ancestor before creating anything, including a missing config dir.
fn reject_package_cache(base: &Path, root: &Path) -> Result<()> {
    ensure!(
        !base.components().any(|part| part == Component::ParentDir),
        "command cache must not use parent path components"
    );
    let absolute = std::path::absolute(base)?;
    let mut ancestor = absolute.as_path();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                ensure!(
                    !ancestor.canonicalize()?.starts_with(root),
                    "command cache must stay outside original package"
                );
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor =
                    ancestor.parent().context("command cache has no existing host ancestor")?;
            }
            Err(error) => return Err(error).context("failed to inspect host command cache parent"),
        }
    }
}
