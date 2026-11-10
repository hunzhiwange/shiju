//! DeepSeek Harness MCP projection for the authorized host's profile patch files.

use crate::{
    mcp::validation::{parse_native, validate_connection},
    providers::{
        dsh::{DshFiles, Snapshot, entries},
        live::{
            commit::{PendingTarget, ProviderTransactions},
            patch_yaml::has_yaml_reference_tokens,
            write_guard::WriteError,
        },
    },
};
use serde_json::{Map, Value};
use serde_yaml::{Mapping, Value as Yaml};
use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    ops::Range,
};
use vw_api_types::{
    TenantScope,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const PLUGIN: &str = "@deepseek-ai/dsh-mcp-client";
const PROFILES: [&str; 4] = ["web", "desktop", "headless", "acp"];
const CONNECTION_FIELDS: [&str; 6] = ["command", "args", "env", "cwd", "url", "headers"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DshError {
    Native(WriteError),
    InvalidDefinition,
    UnsupportedTransport,
    UnresolvedCredential,
    Catalog(WriteError),
    RollbackFailed(WriteError),
}

impl From<WriteError> for DshError {
    fn from(error: WriteError) -> Self {
        Self::Native(error)
    }
}

pub struct NativeServer {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub type NativeServers = BTreeMap<String, Result<NativeServer, DshError>>;

pub enum ServerEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    /// Replace the managed connection and re-enable its native plugin.
    Update(&'a McpConnectionDefinition),
    Disable,
    Remove,
}

struct Profile<'a> {
    name: &'a str,
    bytes: Option<&'a [u8]>,
    patches: Vec<Yaml>,
}

fn profiles(snapshot: &Snapshot) -> Result<Vec<Profile<'_>>, WriteError> {
    PROFILES
        .into_iter()
        .filter_map(|name| snapshot.get(name).map(|bytes| (name, bytes.as_deref())))
        .map(|(name, bytes)| Ok(Profile { name, bytes, patches: entries(bytes)? }))
        .collect()
}

fn plugins(patches: &[Yaml]) -> impl Iterator<Item = (usize, Option<usize>, &Yaml)> {
    patches.iter().enumerate().flat_map(|(index, patch)| {
        std::iter::once((index, None, patch)).chain(
            patch["insert"]
                .as_sequence()
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(child, entry)| (index, Some(child), entry)),
        )
    })
}

fn server_name(plugin: &Yaml) -> Result<Option<&str>, WriteError> {
    if plugin["name"].as_str() != Some(PLUGIN) {
        return Ok(None);
    }
    plugin["config"]["serverName"].as_str().map(Some).ok_or(WriteError::InvalidResource)
}

/// Read host-private import candidates from every initialized profile.
pub fn read_servers(snapshot: &Snapshot) -> Result<NativeServers, DshError> {
    let profiles = profiles(snapshot)?;
    let mut servers = BTreeMap::new();
    for profile in &profiles {
        let mut seen = BTreeSet::new();
        for (_, _, plugin) in plugins(&profile.patches) {
            let Some(id) = server_name(plugin)? else { continue };
            if !seen.insert(id) {
                servers.insert(id.to_owned(), Err(DshError::Native(WriteError::Conflict)));
                continue;
            }
            match servers.entry(id.to_owned()) {
                Entry::Vacant(entry) => {
                    entry.insert(Ok(plugin));
                }
                Entry::Occupied(mut entry) => {
                    if entry.get().is_ok_and(|previous| previous != plugin) {
                        *entry.get_mut() = Err(DshError::Native(WriteError::Conflict));
                    }
                }
            }
        }
    }
    Ok(servers
        .into_iter()
        .map(|(id, plugin)| {
            let server = plugin.and_then(|plugin| {
                validate_id(&id)?;
                read_server(plugin)
            });
            (id, server)
        })
        .collect())
}

fn read_server(plugin: &Yaml) -> Result<NativeServer, DshError> {
    let enabled = match plugin.get("disabled") {
        None => true,
        Some(value) => !value.as_bool().ok_or(DshError::InvalidDefinition)?,
    };
    let config = &plugin["config"];
    let transport = match config["transport"].as_str() {
        Some("stdio") => "stdio",
        Some("streamable-http") => "http",
        Some(_) => return Err(DshError::UnsupportedTransport),
        None => return Err(DshError::InvalidDefinition),
    };
    let mut connection = Map::from_iter([("type".into(), Value::from(transport))]);
    for field in CONNECTION_FIELDS {
        if let Some(value) = config.get(field) {
            connection.insert(
                field.into(),
                serde_json::to_value(value).map_err(|_| DshError::InvalidDefinition)?,
            );
        }
    }
    let connection =
        parse_native(Value::Object(connection)).map_err(|_| DshError::InvalidDefinition)?;
    Ok(NativeServer { connection, enabled })
}

/// The host resolves credentials and authorizes the DSH files before opening them.
/// Preflight checks saved ownership and revision, and commit settles the catalog
/// after file publication. A failed catalog commit must leave the catalog unchanged.
pub fn sync_and_commit<T>(
    files: &mut DshFiles,
    transactions: &ProviderTransactions,
    tenant: &TenantScope,
    id: &str,
    edit: ServerEdit<'_>,
    preflight: impl Fn() -> Result<(), WriteError>,
    commit: impl FnOnce() -> Result<T, WriteError>,
) -> Result<T, DshError> {
    let before = files.snapshot()?;
    let after = plan(&before, id, edit)?;
    files.apply_checked(
        transactions,
        tenant,
        PendingTarget::default(),
        &before,
        &after,
        preflight,
    )?;
    match commit() {
        Ok(result) => Ok(result),
        Err(error) => {
            if before != after {
                files
                    .apply_checked(
                        transactions,
                        tenant,
                        PendingTarget::default(),
                        &after,
                        &before,
                        || Ok(()),
                    )
                    .map_err(DshError::RollbackFailed)?;
            }
            Err(DshError::Catalog(error))
        }
    }
}

fn validate_id(id: &str) -> Result<(), DshError> {
    if id.is_empty()
        || id.len() > 32
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(DshError::InvalidDefinition);
    }
    Ok(())
}

fn project(connection: &McpConnectionDefinition) -> Result<Mapping, DshError> {
    validate_connection(connection).map_err(|_| DshError::InvalidDefinition)?;
    let mut config = Mapping::new();
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd } => {
            config.insert("transport".into(), "stdio".into());
            config.insert("command".into(), command.as_str().into());
            if !args.is_empty() {
                config.insert(
                    "args".into(),
                    Yaml::Sequence(args.iter().map(|arg| arg.as_str().into()).collect()),
                );
            }
            if !env.is_empty() {
                config.insert("env".into(), literals(env)?);
            }
            if let Some(cwd) = cwd {
                config.insert("cwd".into(), cwd.as_str().into());
            }
        }
        McpConnectionDefinition::StreamableHttp { url, headers } => {
            config.insert("transport".into(), "streamable-http".into());
            config.insert("url".into(), url.as_str().into());
            if !headers.is_empty() {
                config.insert("headers".into(), literals(headers)?);
            }
        }
        McpConnectionDefinition::Sse { .. } => return Err(DshError::UnsupportedTransport),
    }
    Ok(config)
}

fn literals(values: &BTreeMap<String, McpConfigValue>) -> Result<Yaml, DshError> {
    values
        .iter()
        .map(|(key, value)| match value {
            McpConfigValue::Literal { value } => Ok((key.as_str().into(), value.as_str().into())),
            McpConfigValue::Credential { .. } => Err(DshError::UnresolvedCredential),
        })
        .collect::<Result<Mapping, _>>()
        .map(Yaml::Mapping)
}

type PluginLocation<'a> = (usize, Option<usize>, &'a Yaml);

fn locate<'a>(patches: &'a [Yaml], id: &str) -> Result<Option<PluginLocation<'a>>, WriteError> {
    let mut found = None;
    for (index, child, plugin) in plugins(patches) {
        if server_name(plugin)? == Some(id) && found.replace((index, child, plugin)).is_some() {
            return Err(WriteError::Conflict);
        }
    }
    Ok(found)
}

fn plan(before: &Snapshot, id: &str, edit: ServerEdit<'_>) -> Result<Snapshot, DshError> {
    validate_id(id)?;
    let fields = match edit {
        ServerEdit::Insert(connection) | ServerEdit::Update(connection) => {
            Some(project(connection)?)
        }
        ServerEdit::Disable | ServerEdit::Remove => None,
    };
    let profiles = profiles(before)?;
    let locations = profiles
        .iter()
        .map(|profile| locate(&profile.patches, id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut existing = None;
    for (_, _, plugin) in locations.iter().flatten() {
        if matches!(edit, ServerEdit::Insert(_))
            || existing.is_some_and(|previous| previous != *plugin)
        {
            return Err(WriteError::Conflict.into());
        }
        existing = Some(*plugin);
    }

    let home = entries(before.get("home").and_then(|bytes| bytes.as_deref()))?;
    if locate(&home, id)?.is_some() {
        return Err(WriteError::Conflict.into());
    }
    if existing.is_none() {
        match edit {
            ServerEdit::Insert(_) => {}
            ServerEdit::Update(_) => return Err(WriteError::Conflict.into()),
            ServerEdit::Disable | ServerEdit::Remove => return Ok(before.clone()),
        }
    }
    let plugin_id = match existing {
        Some(plugin) => plugin["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or(WriteError::InvalidResource)?
            .to_owned(),
        None => format!("shiju-mcp-{id}"),
    };
    if plugins(&home).any(|(_, _, plugin)| plugin["id"].as_str() == Some(&plugin_id)) {
        return Err(WriteError::Conflict.into());
    }
    if let Some(plugin) = existing.filter(|_| !matches!(edit, ServerEdit::Remove)) {
        read_server(plugin)?;
    }
    let replacement = match edit {
        ServerEdit::Remove => None,
        ServerEdit::Disable => {
            let mut plugin = existing.ok_or(WriteError::Conflict)?.clone();
            plugin["disabled"] = true.into();
            Some(plugin)
        }
        ServerEdit::Insert(_) | ServerEdit::Update(_) => {
            let mut plugin = existing.cloned().unwrap_or_else(|| {
                Yaml::Mapping(Mapping::from_iter([
                    ("id".into(), plugin_id.as_str().into()),
                    ("name".into(), PLUGIN.into()),
                    ("config".into(), Yaml::Mapping(Mapping::new())),
                ]))
            });
            let entry = plugin.as_mapping_mut().ok_or(WriteError::InvalidResource)?;
            entry.remove("disabled");
            let config = entry
                .get_mut("config")
                .and_then(Yaml::as_mapping_mut)
                .ok_or(WriteError::InvalidResource)?;
            for field in CONNECTION_FIELDS {
                config.remove(field);
            }
            config.extend(fields.ok_or(WriteError::InvalidResource)?);
            config.insert("serverName".into(), id.into());
            Some(plugin)
        }
    };
    let seeding = matches!(edit, ServerEdit::Update(_)) && locations.iter().any(Option::is_none);
    let mut after = before.clone();
    for (profile, location) in profiles.iter().zip(locations) {
        if plugins(&profile.patches)
            .filter(|(_, _, plugin)| plugin["id"].as_str() == Some(&plugin_id))
            .count()
            > usize::from(location.is_some())
        {
            return Err(WriteError::Conflict.into());
        }
        if location.is_none() && matches!(edit, ServerEdit::Disable | ServerEdit::Remove) {
            continue;
        }
        if let Some((index, _, plugin)) = location
            && Some(plugin) == replacement.as_ref()
        {
            if seeding {
                // A copied plugin must not depend on aliases, tags or anchors in its source.
                patch_span(profile_text(profile)?, profile.patches.len(), index)?;
            }
            continue;
        }
        let bytes = patch_profile(profile, location, replacement.clone())?;
        after.insert(profile.name.into(), Some(bytes));
    }
    Ok(after)
}

fn patch_profile(
    profile: &Profile<'_>,
    location: Option<PluginLocation<'_>>,
    replacement: Option<Yaml>,
) -> Result<Vec<u8>, WriteError> {
    let text = profile_text(profile)?;
    let mut expected = profile.patches.clone();
    let (index, next) = match location {
        Some((index, Some(child), _)) => {
            let mut group = expected[index].clone();
            let inserted = group["insert"].as_sequence_mut().ok_or(WriteError::InvalidResource)?;
            if let Some(replacement) = replacement {
                inserted[child] = replacement;
                (Some(index), Some(group))
            } else {
                inserted.remove(child);
                let keep_group = !inserted.is_empty()
                    || group.as_mapping().is_some_and(|fields| fields.len() > 1);
                (Some(index), keep_group.then_some(group))
            }
        }
        Some((index, None, _)) => (Some(index), replacement),
        None => (
            None,
            replacement.map(|plugin| {
                Yaml::Mapping(Mapping::from_iter([("insert".into(), Yaml::Sequence(vec![plugin]))]))
            }),
        ),
    };
    match (index, &next) {
        (Some(index), Some(patch)) => expected[index] = patch.clone(),
        (Some(index), None) => {
            expected.remove(index);
        }
        (None, Some(patch)) => expected.push(patch.clone()),
        (None, None) => return Err(WriteError::InvalidResource),
    }
    let encoded = match next {
        Some(patch) => serde_yaml::to_string(&[patch]).map_err(|_| WriteError::InvalidResource)?,
        None if expected.is_empty() => "[]\n".into(),
        None => String::new(),
    };
    let result = if let Some(index) = index {
        let span = patch_span(text, profile.patches.len(), index)?;
        format!("{}{}{}", &text[..span.start], encoded, &text[span.end..])
    } else {
        let mut prefix: String = if profile.patches.is_empty() {
            text.split_inclusive('\n')
                .filter(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
                .collect()
        } else {
            text.to_owned()
        };
        if !prefix.is_empty() && !prefix.ends_with('\n') {
            prefix.push('\n');
        }
        prefix.push_str(&encoded);
        prefix
    };
    if entries(Some(result.as_bytes()))? != expected {
        return Err(WriteError::InvalidResource);
    }
    Ok(result.into_bytes())
}

fn profile_text<'a>(profile: &Profile<'a>) -> Result<&'a str, WriteError> {
    std::str::from_utf8(profile.bytes.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)
}

fn patch_span(text: &str, patch_count: usize, index: usize) -> Result<Range<usize>, WriteError> {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("- ") || line.trim_end() == "-" {
            starts.push(offset);
        }
        offset += line.len();
    }
    if starts.len() != patch_count {
        return Err(WriteError::InvalidResource);
    }
    let span = starts[index]..starts.get(index + 1).copied().unwrap_or(text.len());
    if text[span.clone()].lines().any(has_yaml_reference_tokens) {
        return Err(WriteError::InvalidResource);
    }
    Ok(span)
}

#[cfg(test)]
#[path = "dsh_tests.rs"]
mod tests;
