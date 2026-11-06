//! Hermes custom-provider list edits; shared transactions own publication.
use super::live::write_guard::WriteError;
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::hermes::*};
fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
fn label(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}
/// Normalize form aliases and model arrays without reading environment credentials.
pub fn normalize_declaration(v: &Value) -> Result<Value, WriteError> {
    let mut obj = v.as_object().cloned().ok_or(WriteError::InvalidResource)?;
    for (old, new) in [
        ("baseUrl", "base_url"),
        ("apiKey", "api_key"),
        ("apiMode", "api_mode"),
        ("maxTokens", "max_tokens"),
        ("contextLength", "context_length"),
    ] {
        if let Some(v) = obj.remove(old) {
            obj.entry(new).or_insert(v);
        }
    }
    for key in ["api", "_cc_source", "provider_key", "name"] {
        obj.remove(key);
    }
    for key in ["base_url", "api_key", "api_mode", "key_env", "model"] {
        if obj.get(key).is_some_and(|v| !v.is_string()) {
            return invalid();
        }
    }
    if obj.get("key_env").and_then(Value::as_str).is_some_and(|s| {
        s.is_empty()
            || !s
                .bytes()
                .enumerate()
                .all(|(i, c)| c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit()))
    }) {
        return invalid();
    }
    if obj.get("api_key").and_then(Value::as_str).is_some_and(|s| {
        matches!(s, "[redacted]" | "<redacted>")
            || (!s.is_empty() && s.chars().all(|c| matches!(c, '*' | '•')))
    }) {
        return invalid();
    }
    if let Some(models) = obj.remove("models") {
        let mut map = serde_json::Map::new();
        let mut first = None;
        match models {
            Value::Array(models) => {
                for model in models {
                    let mut m = model.as_object().cloned().ok_or(WriteError::InvalidResource)?;
                    let id = m
                        .remove("id")
                        .and_then(|v| v.as_str().map(str::to_string))
                        .ok_or(WriteError::InvalidResource)?;
                    if !label(&id) || map.contains_key(&id) {
                        return invalid();
                    }
                    first.get_or_insert(id.clone());
                    map.insert(id, Value::Object(m));
                }
            }
            Value::Object(models) => {
                for (id, v) in models {
                    if !label(&id) || !(v.is_object() || v.is_null()) {
                        return invalid();
                    }
                    first.get_or_insert(id.clone());
                    map.insert(id, if v.is_null() { json!({}) } else { v });
                }
            }
            _ => return invalid(),
        }
        if let Some(id) = first {
            if !obj.contains_key("model") {
                obj.insert("model".into(), json!(id));
            }
        } else {
            obj.remove("model");
        }
        obj.insert("models".into(), Value::Object(map));
    }
    Ok(Value::Object(obj))
}
pub fn validate(p: &HermesProfile) -> Result<(), WriteError> {
    if !label(&p.name)
        || p.name.len() > 128
        || serde_json::to_vec(&p.declaration).map_err(|_| WriteError::InvalidResource)?.len()
            > 1024 * 1024
    {
        return invalid();
    }
    let obj = p.declaration.as_object().ok_or(WriteError::InvalidResource)?;
    if ["custom_providers", "providers", "agent", "mcp_servers", "env"]
        .iter()
        .any(|k| obj.contains_key(*k))
    {
        return invalid();
    }
    normalize_declaration(&p.declaration)?;
    Ok(())
}
fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let pre = pre.unwrap_or_default();
    if pre.len() > 1024 * 1024 {
        return invalid();
    }
    let v: serde_yaml::Value =
        serde_yaml::from_slice(pre).map_err(|_| WriteError::InvalidResource)?;
    let v = if v.is_null() {
        json!({})
    } else {
        serde_json::to_value(v).map_err(|_| WriteError::InvalidResource)?
    };
    if !v.is_object()
        || v.get("custom_providers").is_some_and(|v| !v.is_array() && !v.is_null())
        || v.get("providers").is_some_and(|v| !v.is_object() && !v.is_null())
    {
        return invalid();
    }
    Ok(v)
}
fn profile(id: &str, value: &Value, source: &str, key: &str) -> Result<HermesProfile, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let mut declaration = normalize_declaration(value)?;
    if let Some(models) = declaration.get_mut("models") {
        let map = models.as_object().ok_or(WriteError::InvalidResource)?;
        *models = Value::Array(
            map.iter()
                .map(|(id, v)| {
                    let mut v = v.clone();
                    v["id"] = json!(id);
                    v
                })
                .collect(),
        );
    }
    declaration["_cc_source"] = json!(source);
    if source == "providers_dict" {
        declaration["provider_key"] = json!(key);
    }
    let p = HermesProfile { name: id.into(), declaration };
    validate(&p)?;
    Ok(p)
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, HermesProfile>, WriteError> {
    let doc = document(pre)?;
    let mut rows = BTreeMap::new();
    if let Some(list) = doc.get("custom_providers").and_then(Value::as_array) {
        for entry in list {
            let id =
                entry.get("name").and_then(Value::as_str).ok_or(WriteError::InvalidResource)?;
            if rows.insert(id.into(), profile(id, entry, "custom_providers", id)?).is_some() {
                return invalid();
            }
        }
    }
    if let Some(map) = doc.get("providers").and_then(Value::as_object) {
        for (key, entry) in map {
            if !entry.is_object() {
                return invalid();
            }
            let id = entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(key.trim());
            if !rows.contains_key(id) {
                rows.insert(id.into(), profile(id, entry, "providers_dict", key)?);
            }
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<HermesState, WriteError> {
    let rows = declarations(pre)?;
    Ok(HermesState {
        read_only_members: rows
            .iter()
            .filter(|(_, p)| p.declaration["_cc_source"] == "providers_dict")
            .map(|(id, _)| id.clone())
            .collect(),
        members: rows.into_keys().collect(),
    })
}
/// Replace only the named list entry. Verify the resulting document against the intended value.
/// Ambiguous flow sections, anchors that cross the edited node and unsupported layouts fail closed.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    replacement: Option<&Value>,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let mut expected = document(pre)?;
    let rows = declarations(pre)?;
    // A dict key may differ from its display name; neither spelling may shadow the overlay.
    let list =
        expected.get("custom_providers").and_then(Value::as_array).cloned().unwrap_or_default();
    let index = list.iter().position(|p| p["name"].as_str() == Some(id));
    if index.is_none()
        && (rows.get(id).is_some_and(|p| p.declaration["_cc_source"] == "providers_dict")
            || expected
                .get("providers")
                .and_then(Value::as_object)
                .is_some_and(|m| m.contains_key(id)))
    {
        return invalid();
    }
    if replacement.is_none() && index.is_none() {
        return Ok(pre.unwrap_or_default().to_vec());
    }
    let mut intended = list.clone();
    let entry = if let Some(v) = replacement {
        validate(&HermesProfile { name: id.into(), declaration: v.clone() })?;
        let v = normalize_declaration(v)?;
        let mut merged =
            index.map(|i| normalize_declaration(&list[i])).transpose()?.unwrap_or(json!({}));
        let obj = merged.as_object_mut().ok_or(WriteError::InvalidResource)?;
        // Unknown native fields and omitted associated credentials survive partial forms.
        for (k, v) in v.as_object().ok_or(WriteError::InvalidResource)? {
            obj.insert(k.clone(), v.clone());
        }
        for key in [
            "baseUrl",
            "apiKey",
            "apiMode",
            "maxTokens",
            "contextLength",
            "api",
            "_cc_source",
            "provider_key",
        ] {
            obj.remove(key);
        }
        obj.insert("name".into(), json!(id));
        if let Some(i) = index {
            intended[i] = merged.clone();
        } else {
            intended.push(merged.clone());
        }
        Some(merged)
    } else {
        intended.remove(index.ok_or(WriteError::InvalidResource)?);
        None
    };
    expected["custom_providers"] = json!(intended);
    let text =
        std::str::from_utf8(pre.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut offsets = vec![0];
    for line in &lines {
        offsets.push(offsets.last().unwrap() + line.len());
    }
    let mut header = None;
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(char::is_whitespace)
            || line.trim().is_empty()
            || line.starts_with('#')
            || line.trim() == "---"
            || line.starts_with("- ")
            || line.trim() == "-"
        {
            continue;
        }
        if line.trim() == "..." {
            return invalid();
        }
        // Parse root key spelling; semantic verification catches any ambiguous boundary.
        let Some((key, _)) = line.split_once(':') else {
            return invalid();
        };
        let key: Value = serde_yaml::from_str(key).map_err(|_| WriteError::InvalidResource)?;
        if header.is_some() {
            end = i;
            break;
        }
        if key.as_str() == Some("custom_providers") {
            header = Some(i);
        }
    }
    let mut output = text.to_string();
    if let Some(h) = header {
        let tail = lines[h]
            .split_once(':')
            .ok_or(WriteError::InvalidResource)?
            .1
            .split('#')
            .next()
            .unwrap_or_default()
            .trim();
        if !matches!(tail, "" | "null" | "~" | "[]") {
            return invalid();
        }
        let mut items = vec![];
        let mut indent = None;
        for (i, line) in lines.iter().enumerate().take(end).skip(h + 1) {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let n = line.len() - line.trim_start_matches(' ').len();
            let n0 = *indent.get_or_insert(n);
            if n == n0 && (line.trim_start().starts_with("- ") || line.trim() == "-") {
                items.push(i);
            }
        }
        if items.len() != list.len() {
            return invalid();
        }
        let indent = indent.unwrap_or(2);
        let start = index.map(|i| items[i]).unwrap_or(end);
        let mut stop = index.and_then(|i| items.get(i + 1).copied()).unwrap_or(end);
        while stop > start
            && (lines[stop - 1].trim().is_empty() || lines[stop - 1].trim_start().starts_with('#'))
        {
            stop -= 1;
        }
        let emitted = emit(entry.as_ref(), indent)?;
        // Empty/null sections must have their header value normalized before appending.
        if list.is_empty() && !tail.is_empty() {
            let comment = lines[h]
                .find('#')
                .map(|i| format!(" {}", lines[h][i..].trim_end()))
                .unwrap_or_default();
            output.replace_range(
                offsets[h]..offsets[h + 1],
                &format!("custom_providers:{comment}\n{emitted}"),
            );
        } else {
            let emitted =
                if start == stop && offsets[start] > 0 && !text[..offsets[start]].ends_with('\n') {
                    format!("\n{emitted}")
                } else {
                    emitted
                };
            output.replace_range(offsets[start]..offsets[stop], &emitted);
            if intended.is_empty() {
                output.replace_range(offsets[h]..offsets[h + 1], "custom_providers: []\n");
            }
        }
    } else {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("custom_providers:\n");
        output.push_str(&emit(entry.as_ref(), 2)?);
    }
    if document(Some(output.as_bytes()))? != expected {
        return invalid();
    }
    Ok(output.into_bytes())
}
fn emit(entry: Option<&Value>, indent: usize) -> Result<String, WriteError> {
    let Some(entry) = entry else {
        return Ok(String::new());
    };
    let s = serde_yaml::to_string(&vec![entry]).map_err(|_| WriteError::InvalidResource)?;
    Ok(s.lines().map(|l| format!("{}{l}\n", " ".repeat(indent))).collect())
}
pub(super) fn directory_components(
    home: &Path,
    raw: Option<&OsStr>,
) -> Result<Vec<String>, WriteError> {
    if !home.is_absolute() {
        return invalid();
    }
    let raw = raw.map(|v| v.to_str().ok_or(WriteError::InvalidResource)).transpose()?;
    let path = raw
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".hermes"));
    let relative = path.strip_prefix(home).map_err(|_| WriteError::InvalidResource)?;
    let mut parts = vec![];
    for part in relative.components() {
        let std::path::Component::Normal(part) = part else {
            return invalid();
        };
        parts.push(part.to_str().ok_or(WriteError::InvalidResource)?.to_string());
    }
    if parts.is_empty() {
        return invalid();
    }
    Ok(parts)
}
/// Read only the configured Hermes directory beneath the authorized execution HOME.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    let parts = directory_components(home, std::env::var_os("HERMES_HOME").as_deref())?;
    let mut components: Vec<&str> = parts.iter().map(String::as_str).collect();
    components.push("config.yaml");
    super::catalog::read_file(home, &components).map_err(|_| WriteError::InvalidResource)
}
pub struct HermesFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl HermesFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        if !home.is_absolute() {
            return invalid();
        }
        let mut dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        for component in directory_components(home, std::env::var_os("HERMES_HOME").as_deref())? {
            let name =
                std::ffi::CString::new(component).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed single child of the retained authorized directory.
            if unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
            // SAFETY: fixed child; no symlinks; returned fd consumed once.
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return invalid();
            }
            dir = unsafe { std::fs::File::from_raw_fd(fd) };
        }
        Ok(Self { guard: super::live::write_guard::lock_directory(AppId::Hermes, dir)? })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("config.yaml", self.guard.read(OsStr::new("config.yaml"))?))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        expected: &super::live::commit::AppState,
        name: &str,
        pre: Option<&[u8]>,
        next: Vec<u8>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        if name != "config.yaml" {
            return invalid();
        }
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let after = if pre.is_none() && next.is_empty() { None } else { Some(next) };
        let patch = WholeFile::new(pre, after);
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[FileChange { name: OsStr::new(name), privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget {
                pointer: None,
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: expected.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
            },
            || {
                if unchanged() && tx.state(AppId::Hermes)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
