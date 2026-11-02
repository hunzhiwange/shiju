//! Read-only marketplace metadata; reading never installs or authenticates a package.
use super::super::PluginIdentity;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};
const PATHS: [&str; 4] = [
    ".agents/plugins/marketplace.json",
    ".agents/plugins/api_marketplace.json",
    ".claude-plugin/marketplace.json",
    ".cursor-plugin/marketplace.json",
];
#[derive(Debug)]
pub struct PluginMarketplace {
    pub name: String,
    pub display_name: Option<String>,
    pub plugins: Vec<MarketplaceEntry>,
}
#[derive(Debug)]
pub struct MarketplaceEntry {
    pub identity: PluginIdentity,
    pub source: MarketplaceSource,
    pub policy: MarketplacePolicy,
    pub manifest_fallback: Value,
}
#[derive(Debug, PartialEq, Eq)]
pub enum MarketplaceSource {
    Local(PathBuf),
    Git { url: String, path: Option<String>, reference: Option<String>, sha: Option<String> },
    Npm { package: String, version: Option<String>, registry: Option<String> },
}
#[derive(Debug, Default, Deserialize)]
pub struct MarketplacePolicy {
    #[serde(default)]
    pub installation: InstallationPolicy,
    #[serde(default)]
    pub authentication: AuthenticationPolicy,
    pub products: Option<Vec<String>>,
}
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallationPolicy {
    NotAvailable,
    #[default]
    Available,
    InstalledByDefault,
}
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthenticationPolicy {
    #[default]
    OnInstall,
    OnUse,
}
#[derive(Deserialize)]
struct Raw {
    name: String,
    interface: Option<Value>,
    plugins: Vec<RawEntry>,
}
#[derive(Deserialize)]
struct RawEntry {
    name: String,
    source: Value,
    #[serde(default)]
    policy: MarketplacePolicy,
    #[serde(flatten)]
    metadata: serde_json::Map<String, Value>,
}
/// Load the first supported layout, or an explicitly selected supported manifest.
pub fn load_plugin_marketplace(input: &Path) -> Result<PluginMarketplace> {
    let (root, manifest) = if input.is_dir() {
        let root = fs::canonicalize(input)?;
        let manifest = PATHS
            .iter()
            .map(|p| root.join(p))
            .find(|p| p.is_file())
            .context("marketplace manifest not found")?;
        (root, manifest)
    } else {
        let (layout, root) = PATHS
            .iter()
            .find_map(|layout| {
                input.ends_with(layout).then(|| {
                    (
                        *layout,
                        input.ancestors().nth(Path::new(layout).components().count()).unwrap(),
                    )
                })
            })
            .context("unsupported marketplace layout")?;
        let root = fs::canonicalize(root)?;
        let manifest = root.join(layout);
        (root, manifest)
    };
    let canonical = fs::canonicalize(&manifest)?;
    if !canonical.starts_with(&root) {
        bail!("marketplace manifest escapes root");
    }
    let raw: Raw = serde_json::from_slice(&fs::read(canonical)?)?;
    PluginIdentity::marketplace("validation", &raw.name)?;
    let mut seen = HashSet::new();
    let mut plugins = Vec::new();
    for entry in raw.plugins {
        let identity = PluginIdentity::marketplace(&entry.name, &raw.name)?;
        if !seen.insert(identity.as_key()) {
            bail!("duplicate marketplace identity");
        }
        if entry.policy.products.as_ref().is_some_and(|ps| {
            ps.iter().any(|p| {
                !matches!(p.as_str(), "codex" | "CODEX" | "chatgpt" | "CHATGPT" | "atlas" | "ATLAS")
            })
        }) {
            bail!("invalid product policy");
        }
        let source = resolve(&root, &manifest, &entry.source)?;
        let mut fallback = entry.metadata;
        fallback.insert("name".into(), Value::String(entry.name));
        let mut interface =
            fallback.remove("interface").and_then(|v| v.as_object().cloned()).unwrap_or_default();
        for (from, to) in
            [("displayName", "displayName"), ("homepage", "websiteUrl"), ("category", "category")]
        {
            if let Some(value) = fallback.get(from).cloned() {
                interface.entry(to).or_insert(value);
            }
        }
        if let Some(author) =
            fallback.get("author").and_then(|v| v.get("name")).and_then(Value::as_str)
        {
            interface.entry("developerName").or_insert(Value::String(author.to_owned()));
        }
        if !interface.is_empty() {
            fallback.insert("interface".into(), Value::Object(interface));
        }
        plugins.push(MarketplaceEntry {
            identity,
            source,
            policy: entry.policy,
            manifest_fallback: Value::Object(fallback),
        });
    }
    Ok(PluginMarketplace {
        display_name: raw
            .interface
            .and_then(|v| v.get("displayName").and_then(Value::as_str).map(str::to_owned)),
        name: raw.name,
        plugins,
    })
}
fn field(value: &Value, key: &str) -> Result<String> {
    Ok(value.get(key).and_then(Value::as_str).context("missing source string")?.to_owned())
}
fn optional(value: &Value, key: &str) -> Result<Option<String>> {
    value
        .get(key)
        .map(|v| {
            let text = v.as_str().context("invalid source field")?.trim();
            if text.is_empty() {
                bail!("empty source field");
            }
            Ok(text.to_owned())
        })
        .transpose()
}
fn relative(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || Path::new(value).components().any(|c| !matches!(c, Component::Normal(_)))
    {
        bail!("source path escapes root");
    }
    Ok(())
}
fn resolve(root: &Path, manifest: &Path, value: &Value) -> Result<MarketplaceSource> {
    let kind = if value.is_string() { "local".to_owned() } else { field(value, "source")? };
    match kind.as_str() {
        "local" => {
            let path =
                if let Some(s) = value.as_str() { s.to_owned() } else { field(value, "path")? };
            let tail = if path == "." || path == "./" {
                ""
            } else {
                let tail = path
                    .strip_prefix("./")
                    .or_else(|| {
                        manifest
                            .ends_with(".cursor-plugin/marketplace.json")
                            .then_some(path.as_str())
                    })
                    .context("local source requires ./")?;
                relative(tail)?;
                tail
            };
            let target = root.join(tail);
            let mut ancestor = target.as_path();
            while !ancestor.exists() {
                ancestor = ancestor.parent().context("missing source ancestor")?;
            }
            if !fs::canonicalize(ancestor)?.starts_with(root) {
                bail!("local source escapes root");
            }
            Ok(MarketplaceSource::Local(target))
        }
        "url" | "git-subdir" => {
            let mut url = field(value, "url")?.trim().to_owned();
            if url.starts_with("./") {
                relative(&url[2..])?;
                url = root.join(&url[2..]).display().to_string();
            } else if !(url.starts_with("https://")
                || url.starts_with("http://")
                || url.starts_with("ssh://")
                || url.starts_with("file://")
                || url.starts_with('/')
                || url.starts_with("git@") && url.contains(':'))
            {
                let parts: Vec<_> = url.split('/').collect();
                if parts.len() != 2
                    || parts.iter().any(|s| {
                        s.is_empty()
                            || *s == "."
                            || *s == ".."
                            || !s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                    })
                {
                    bail!("invalid git source");
                }
                url = format!("https://github.com/{url}");
            }
            if url.starts_with("https://github.com/") && !url.ends_with(".git") {
                url.push_str(".git");
            }
            let path = optional(value, "path")?;
            if kind == "git-subdir" && path.is_none() {
                bail!("missing git subdirectory");
            }
            if let Some(p) = &path {
                relative(p.strip_prefix("./").unwrap_or(p))?;
            }
            Ok(MarketplaceSource::Git {
                url,
                path,
                reference: optional(value, "ref")?,
                sha: optional(value, "sha")?,
            })
        }
        "npm" => {
            let package = field(value, "package")?.trim().to_owned();
            let parts: Vec<_> = package.strip_prefix('@').unwrap_or(&package).split('/').collect();
            if parts.len() != if package.starts_with('@') { 2 } else { 1 }
                || parts.iter().any(|s| {
                    s.is_empty()
                        || *s == "."
                        || *s == ".."
                        || !s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                })
            {
                bail!("invalid npm package");
            }
            if !package.starts_with('@') && package.starts_with(['.', '_']) {
                bail!("invalid npm package");
            }
            let version = optional(value, "version")?;
            if version
                .as_deref()
                .is_some_and(|v| v == "." || v == ".." || v.contains(['/', '\\', ':']))
            {
                bail!("npm version must use registry");
            }
            let registry = optional(value, "registry")?;
            if let Some(r) = &registry {
                let parsed = url::Url::parse(r)?;
                if parsed.scheme() != "https"
                    || parsed.host_str().is_none()
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                    || parsed.query().is_some()
                    || parsed.fragment().is_some()
                {
                    bail!("invalid npm registry");
                }
            }
            Ok(MarketplaceSource::Npm { package, version, registry })
        }
        _ => bail!("unsupported marketplace source"),
    }
}

/// A named local catalog, kept outside all original packages.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketplaceRegistration {
    pub name: String,
    pub root: PathBuf,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Registrations {
    schema_version: u32,
    marketplaces: Vec<MarketplaceRegistration>,
}
/// Read registered local catalogs. This does not fetch remote data or install a package.
pub fn list_plugin_marketplaces(config_dir: &Path) -> Result<Vec<MarketplaceRegistration>> {
    let path = config_dir.join("plugin-marketplaces.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 64 * 1024 {
        bail!("invalid marketplace registry file");
    }
    let state: Registrations = serde_json::from_slice(&fs::read(path)?)?;
    if state.schema_version != 1 {
        bail!("unsupported marketplace registry schema");
    }
    let mut names = HashSet::new();
    for item in &state.marketplaces {
        PluginIdentity::marketplace("validation", &item.name)?;
        if !item.root.is_absolute() || !names.insert(&item.name) {
            bail!("invalid marketplace registration");
        }
    }
    Ok(state.marketplaces)
}
/// Register an existing local catalog; Git/npm entries within it use their audited factories.
pub fn add_plugin_marketplace(config_dir: &Path, source: &Path) -> Result<MarketplaceRegistration> {
    use std::io::Write;
    let root = source.canonicalize().context("marketplace requires an existing local catalog")?;
    let marketplace = load_plugin_marketplace(&root)?;
    fs::create_dir_all(config_dir)?;
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let lock_path = config_dir.join(".plugin-marketplaces.lock");
    let lock = options.open(&lock_path)?;
    if fs::symlink_metadata(&lock_path)?.file_type().is_symlink() {
        bail!("unsafe marketplace registry lock");
    }
    lock.lock()?;
    let mut entries = list_plugin_marketplaces(config_dir)?;
    if let Some(old) = entries.iter().find(|e| e.name == marketplace.name) {
        if old.root != root {
            bail!("marketplace name is already registered at a different root");
        }
        return Ok(MarketplaceRegistration { name: marketplace.name, root });
    }
    entries.push(MarketplaceRegistration { name: marketplace.name.clone(), root: root.clone() });
    let bytes = serde_json::to_vec(&Registrations { schema_version: 1, marketplaces: entries })?;
    if bytes.len() > 64 * 1024 {
        bail!("marketplace registry exceeds size limit");
    }
    let mut staged = tempfile::NamedTempFile::new_in(config_dir)?;
    staged.write_all(&bytes)?;
    staged.as_file().sync_all()?;
    staged.persist(config_dir.join("plugin-marketplaces.json"))?;
    super::installed_state::sync_dir(config_dir)?;
    Ok(MarketplaceRegistration { name: marketplace.name, root })
}
/// Read the current catalog at a previously registered root, preserving its exact identity.
pub fn read_registered_marketplace(config_dir: &Path, name: &str) -> Result<PluginMarketplace> {
    let entry = list_plugin_marketplaces(config_dir)?
        .into_iter()
        .find(|e| e.name == name)
        .context("marketplace is not registered")?;
    let marketplace = load_plugin_marketplace(&entry.root)?;
    if marketplace.name != name {
        bail!("registered marketplace identity changed");
    }
    Ok(marketplace)
}
pub fn registered_plugin_entry(config_dir: &Path, key: &str) -> Result<MarketplaceEntry> {
    let identity = PluginIdentity::parse_marketplace(key)?;
    read_registered_marketplace(
        config_dir,
        identity.marketplace_name().context("missing marketplace")?,
    )?
    .plugins
    .into_iter()
    .find(|p| p.identity == identity)
    .context("plugin is not listed in registered marketplace")
}
