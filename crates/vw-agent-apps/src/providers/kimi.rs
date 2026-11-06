//! Kimi native TOML nodes. Publication belongs to the shared transaction owner.
use super::live::write_guard::WriteError;
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use toml_edit::{DocumentMut, Item, Table};
use vw_api_types::{agent_apps::AppId, app_providers::kimi::*};
fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
fn key_valid(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 128 && !id.chars().any(char::is_control)
}
/// Native provider keys remain data, never paths or command arguments.
pub fn validate_identity(id: &str) -> Result<(), WriteError> {
    if !key_valid(id) || matches!(id, "." | "..") || id.contains(['/', '\\']) {
        return invalid();
    }
    Ok(())
}
pub fn validate_for(id: &str, p: &KimiProfile) -> Result<(), WriteError> {
    validate_identity(id)?;
    validate(p)?;
    validate_references(id, &p.declaration)
}
fn secret(s: &str) -> bool {
    !s.trim().is_empty()
        && !matches!(s, "[redacted]" | "<redacted>")
        && !s.chars().all(|c| matches!(c, '*' | '•'))
}
pub fn official(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("kimi")
        && node.pointer("/oauth/key").and_then(Value::as_str).is_some_and(|k| !k.trim().is_empty())
}
pub fn validate(p: &KimiProfile) -> Result<(), WriteError> {
    if !key_valid(&p.name)
        || serde_json::to_vec(&p.declaration).map_err(|_| WriteError::InvalidResource)?.len()
            > 1024 * 1024
    {
        return invalid();
    }
    let obj = p.declaration.as_object().ok_or(WriteError::InvalidResource)?;
    if obj.len() != 2 || !obj.contains_key("provider") || !obj.contains_key("models") {
        return invalid();
    }
    let node = &obj["provider"];
    if !node.is_object()
        || !matches!(
            node.get("type").and_then(Value::as_str),
            Some(
                "kimi" | "openai" | "openai_responses" | "anthropic" | "google-genai" | "vertexai"
            )
        )
    {
        return invalid();
    }
    if node.get("base_url").is_some_and(|v| {
        !v.as_str().is_some_and(|s| {
            reqwest::Url::parse(s)
                .is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
        })
    }) {
        return invalid();
    }
    for field in ["api_key", "api_key_env"] {
        if node.get(field).is_some_and(|v| {
            !v.as_str().is_some_and(|s| field == "api_key" && s.is_empty() || secret(s))
        }) {
            return invalid();
        }
    }
    if node
        .get("custom_headers")
        .is_some_and(|v| !v.as_object().is_some_and(|m| m.values().all(Value::is_string)))
    {
        return invalid();
    }
    if node
        .get("env")
        .is_some_and(|v| !v.as_object().is_some_and(|m| m.values().all(Value::is_string)))
    {
        return invalid();
    }
    if node.get("oauth").is_some_and(|v| {
        !v.is_object()
            || !v.get("key").and_then(Value::as_str).is_some_and(secret)
            || v.get("storage").is_some_and(|s| s.as_str() != Some("file"))
    }) {
        return invalid();
    }
    let api = node.get("api_key").and_then(Value::as_str).is_some_and(secret);
    let env = node.get("api_key_env").and_then(Value::as_str).is_some_and(secret);
    let oauth = node.get("oauth").is_some();
    let env_keys = [
        "KIMI_API_KEY",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GOOGLE_API_KEY",
        "VERTEXAI_API_KEY",
    ]
    .iter()
    .any(|k| node.get("env").and_then(|m| m.get(*k)).and_then(Value::as_str).is_some_and(secret));
    if !(api || env || oauth || env_keys) || env && (api || oauth) {
        return invalid();
    }
    let models = obj["models"].as_object().ok_or(WriteError::InvalidResource)?;
    if models.is_empty() {
        return invalid();
    }
    for (key, m) in models {
        if !key_valid(key)
            || !m.is_object()
            || !m.get("model").and_then(Value::as_str).is_some_and(|s| !s.trim().is_empty())
            || !m
                .get("max_context_size")
                .and_then(Value::as_u64)
                .is_some_and(|n| n > 0 && n <= i64::MAX as u64)
            || m.get("display_name").is_some_and(|v| !v.is_string())
            || m.get("overrides").is_some_and(|v| !v.is_object())
            || m.get("provider").is_some_and(|v| !v.as_str().is_some_and(key_valid))
        {
            return invalid();
        }
        for path in ["/capabilities", "/overrides/capabilities"] {
            if m.pointer(path)
                .is_some_and(|v| !v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
            {
                return invalid();
            }
        }
    }
    // Ensure all extensions can be represented in native TOML (e.g. no null values).
    toml::to_string(&p.declaration).map_err(|_| WriteError::InvalidResource)?;
    Ok(())
}
fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return invalid();
    }
    let doc: Value =
        toml::from_str(std::str::from_utf8(bytes).map_err(|_| WriteError::InvalidResource)?)
            .map_err(|_| WriteError::InvalidResource)?;
    for section in ["providers", "models", "services"] {
        if doc.get(section).is_some_and(|v| !v.is_object()) {
            return invalid();
        }
    }
    if let Some(rows) = doc.get("providers").and_then(Value::as_object) {
        for (id, node) in rows {
            if !key_valid(id) || !node.is_object() {
                return invalid();
            }
        }
    }
    if let Some(rows) = doc.get("models").and_then(Value::as_object) {
        for (key, m) in rows {
            if !key_valid(key)
                || !m.is_object()
                || !m
                    .get("provider")
                    .and_then(Value::as_str)
                    .is_some_and(|id| doc.get("providers").and_then(|p| p.get(id)).is_some())
            {
                return invalid();
            }
        }
    }
    for selected in [doc.get("default_model"), doc.pointer("/secondary_model/default_model")]
        .into_iter()
        .flatten()
    {
        if !selected.as_str().is_some_and(|key| {
            key.is_empty() || doc.get("models").and_then(|m| m.get(key)).is_some()
        }) {
            return invalid();
        }
    }
    if doc.get("secondary_model").is_some_and(|v| !v.is_object()) {
        return invalid();
    }
    if let Some(models) = doc.pointer("/secondary_model/models") {
        let map = models.as_object().ok_or(WriteError::InvalidResource)?;
        if map.keys().any(|k| doc.get("models").and_then(|m| m.get(k)).is_none()) {
            return invalid();
        }
    }
    Ok(doc)
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, KimiProfile>, WriteError> {
    let d = document(pre)?;
    let mut out = BTreeMap::new();
    if let Some(rows) = d.get("providers").and_then(Value::as_object) {
        for (id, node) in rows {
            let models: serde_json::Map<String, Value> = d
                .get("models")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter(|(_, m)| m.get("provider").and_then(Value::as_str) == Some(id))
                .map(|(k, m)| (k.clone(), m.clone()))
                .collect();
            out.insert(
                id.clone(),
                KimiProfile {
                    name: id.clone(),
                    declaration: json!({"provider":node,"models":models}),
                },
            );
        }
    }
    Ok(out)
}
pub fn state(pre: Option<&[u8]>) -> Result<KimiState, WriteError> {
    Ok(KimiState { members: declarations(pre)?.into_keys().collect() })
}
pub fn key_exists(pre: Option<&[u8]>, id: &str) -> Result<bool, WriteError> {
    Ok(declarations(pre)?.contains_key(id))
}
pub fn parse_editor(id: &str, text: &str) -> Result<Value, WriteError> {
    let doc: Value = toml::from_str(text).map_err(|_| WriteError::InvalidResource)?;
    validate_identity(id)?;
    if text.len() > 1024 * 1024 {
        return invalid();
    }
    if !doc.as_object().is_some_and(|m| m.len() == 2)
        || !doc
            .get("providers")
            .and_then(Value::as_object)
            .is_some_and(|m| m.len() == 1 && m.contains_key(id))
    {
        return invalid();
    }
    let node = json!({"provider":doc["providers"][id],"models":doc["models"]});
    validate(&KimiProfile { name: id.into(), declaration: node.clone() })?;
    validate_references(id, &node)?;
    Ok(node)
}
fn validate_references(id: &str, node: &Value) -> Result<(), WriteError> {
    if node["models"]
        .as_object()
        .ok_or(WriteError::InvalidResource)?
        .values()
        .any(|m| m.get("provider").is_some_and(|v| v.as_str() != Some(id)))
    {
        return invalid();
    }
    Ok(())
}
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    node: Option<&Value>,
    activation: KimiActivation,
) -> Result<Vec<u8>, WriteError> {
    validate_identity(id)?;
    let before = document(pre)?;
    let mut node = node.cloned();
    let previous = before.get("providers").and_then(|m| m.get(id));
    if let Some(n) = node.as_mut() {
        validate(&KimiProfile { name: id.into(), declaration: n.clone() })?;
        validate_references(id, n)?;
        if official(&n["provider"]) && !previous.is_some_and(official) {
            return invalid();
        }
        if previous.is_some_and(official) {
            n["provider"] = previous.unwrap().clone();
        }
        match activation {
            KimiActivation::CatalogOnly => return invalid(),
            KimiActivation::Add if previous.is_some() => return Err(WriteError::Conflict),
            KimiActivation::IfMember if previous.is_none() => {
                return Ok(pre.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }
    if node.is_none() && previous.is_some_and(official) {
        // Account/service ownership belongs to the authorization center; never rewrite it here.
        return invalid();
    }
    let removed = |key: &str| {
        before
            .get("models")
            .and_then(|m| m.get(key))
            .and_then(|m| m.get("provider"))
            .and_then(Value::as_str)
            == Some(id)
            && node.as_ref().and_then(|n| n["models"].get(key)).is_none()
    };
    if before.get("default_model").and_then(Value::as_str).is_some_and(removed)
        || before
            .pointer("/secondary_model/default_model")
            .and_then(Value::as_str)
            .is_some_and(removed)
        || before
            .pointer("/secondary_model/models")
            .and_then(Value::as_object)
            .is_some_and(|m| m.keys().any(|k| removed(k)))
    {
        return invalid();
    }
    let mut after = before.clone();
    if let Some(n) = &node {
        for key in n["models"].as_object().unwrap().keys() {
            if before
                .get("models")
                .and_then(|m| m.get(key))
                .is_some_and(|m| m.get("provider").and_then(Value::as_str) != Some(id))
            {
                return Err(WriteError::Conflict);
            }
        }
    }
    if node.is_none() && previous.is_none() {
        return Ok(pre.unwrap_or_default().to_vec());
    }
    for section in ["providers", "models"] {
        if after.get(section).is_none() {
            after[section] = json!({});
        }
    }
    after["models"]
        .as_object_mut()
        .unwrap()
        .retain(|_, m| m.get("provider").and_then(Value::as_str) != Some(id));
    if let Some(n) = node {
        after["providers"][id] = n["provider"].clone();
        for (key, m) in n["models"].as_object().unwrap() {
            let mut m = m.clone();
            m["provider"] = json!(id);
            after["models"][key] = m;
        }
        if before.get("default_model").and_then(Value::as_str).is_none_or(str::is_empty) {
            after["default_model"] = json!(n["models"].as_object().unwrap().keys().next().unwrap());
        }
    } else {
        after["providers"].as_object_mut().unwrap().remove(id);
    }
    let text =
        std::str::from_utf8(pre.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    let mut doc = text.parse::<DocumentMut>().map_err(|_| WriteError::InvalidResource)?;
    let next = toml::to_string(&after)
        .map_err(|_| WriteError::InvalidResource)?
        .parse::<DocumentMut>()
        .map_err(|_| WriteError::InvalidResource)?;
    for section in ["providers", "models"] {
        let old = before.get(section).and_then(Value::as_object);
        let new = after.get(section).and_then(Value::as_object);
        if old == new {
            continue;
        }
        if doc.get(section).is_none() {
            let mut t = Table::new();
            t.set_implicit(true);
            doc.insert(section, Item::Table(t));
        }
        let table = doc[section].as_table_like_mut().ok_or(WriteError::InvalidResource)?;
        for (key, _) in old.into_iter().flatten() {
            if new.is_none_or(|m| !m.contains_key(key)) {
                table.remove(key);
            }
        }
        for (key, value) in new.into_iter().flatten() {
            if old.and_then(|m| m.get(key)) != Some(value) {
                table.insert(key, next[section][key].clone());
            }
        }
    }
    if before.get("default_model") != after.get("default_model") {
        doc["default_model"] = next["default_model"].clone();
    }
    Ok(doc.to_string().into_bytes())
}
fn components(home: &Path) -> Result<Vec<String>, WriteError> {
    if !home.is_absolute() || std::env::var_os("KIMI_CODE_HOME").is_some_and(|v| !v.is_empty()) {
        return invalid();
    }
    Ok(vec![".kimi-code".into()])
}
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    components(home)?;
    super::catalog::read_file(home, &[".kimi-code", "config.toml"])
        .map_err(|_| WriteError::InvalidResource)
}
pub struct KimiFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl KimiFiles {
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
        for component in components(home)? {
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
        Ok(Self { guard: super::live::write_guard::lock_directory(AppId::Kimi, dir)? })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("config.toml", self.guard.read(OsStr::new("config.toml"))?))
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
        if name != "config.toml" {
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
                if unchanged() && tx.state(AppId::Kimi)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}

pub fn selection(
    pre: Option<&[u8]>,
) -> Result<vw_api_types::app_providers::catalog::ProviderSelection, WriteError> {
    use vw_api_types::app_providers::catalog::ProviderSelection;
    let d = document(pre)?;
    match d.get("default_model").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(alias) => Ok(ProviderSelection::Selected {
            id: d["models"][alias]["provider"].as_str().ok_or(WriteError::InvalidResource)?.into(),
        }),
        None => Ok(ProviderSelection::Unconfigured),
    }
}
