//! Server-owned locators. Construct the registry only after gateway authorization,
//! using roots resolved from the selected application instance on the target host.
//! Never deserialize this registry or populate it from client SessionMeta values.
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionMeta, SessionProvider, SessionSource};
use vw_api_types::ProjectScope;

/// A verified location is deliberately not a wire type. Re-resolve it immediately
/// before use; retaining it does not authorize a later filesystem mutation.
#[derive(Debug, Clone)]
pub struct ValidatedSource {
    identity: ExternalSessionIdentity,
    root: PathBuf,
    path: PathBuf,
    project_dir: Option<String>,
}
impl ValidatedSource {
    pub fn project_dir(&self) -> Option<&str> { self.project_dir.as_deref() }
    pub fn identity(&self) -> &ExternalSessionIdentity { &self.identity }
    pub fn root(&self) -> &Path { &self.root }
    pub fn path(&self) -> &Path { &self.path }
    pub fn record_id(&self) -> Option<&str> {
        match &self.identity.source {
            SessionSource::Sqlite { record_id, .. } => Some(record_id),
            SessionSource::File { .. } => None,
        }
    }
}

/// One authorized project/host/application binding. Roots and discovered paths
/// come from host-side adapters, never from a request's display sourcePath.
/// The gateway must repeat authorization on every operation before resolving.
pub struct SourceRegistry {
    scope: ProjectScope,
    target_host: String,
    instance_id: String,
    provider: SessionProvider,
    roots: Vec<PathBuf>,
    sources: HashMap<ExternalSessionIdentity, (PathBuf, Option<String>)>,
}
impl SourceRegistry {
    pub fn new(
        scope: ProjectScope,
        target_host: String,
        instance_id: String,
        provider: SessionProvider,
        roots: Vec<PathBuf>,
    ) -> io::Result<Self> {
        if target_host.trim().is_empty() || instance_id.trim().is_empty() || roots.is_empty() {
            return Err(invalid("missing authorized source binding"));
        }
        for root in &roots { check_absolute(root)?; }
        Ok(Self { scope, target_host, instance_id, provider, roots, sources: HashMap::new() })
    }

    /// Register only a freshly scanned server record with its adapter-resolved
    /// physical path. Display strings (including sqlite: URIs) are never parsed.
    pub fn register(&mut self, meta: &SessionMeta, path: PathBuf) -> io::Result<()> {
        self.check_identity(&meta.identity)?;
        self.validate_path(&meta.identity, &path)?;
        if let Some(existing) = self.sources.get(&meta.identity) {
            if existing != &(path.clone(), meta.project_dir.clone()) { return Err(invalid("ambiguous session locator")); }
        }
        self.sources.insert(meta.identity.clone(), (path, meta.project_dir.clone()));
        Ok(())
    }

    /// Exact equality binds native ID, source record, provider, tenant/project,
    /// instance and host. Forged paths and IDs fail before filesystem access.
    pub fn resolve(&self, identity: &ExternalSessionIdentity) -> io::Result<ValidatedSource> {
        self.check_identity(identity)?;
        let (path, project_dir) = self.sources.get(identity).ok_or_else(forbidden)?;
        let (root, path) = self.validate_path(identity, path)?;
        Ok(ValidatedSource { identity: identity.clone(), root, path, project_dir: project_dir.clone() })
    }

    fn check_identity(&self, identity: &ExternalSessionIdentity) -> io::Result<()> {
        if identity.scope != self.scope || identity.target_host != self.target_host
            || identity.instance_id != self.instance_id || identity.provider != self.provider
            || identity.session_id.0.is_empty() {
            return Err(forbidden());
        }
        super::identity::validate_identity(identity).map_err(|_| forbidden())?;
        if let SessionSource::Sqlite { database, record_id } = &identity.source {
            if record_id != &identity.session_id.0 || record_id.is_empty() {
                return Err(forbidden());
            }
            check_relative(Path::new(database))?;
        }
        Ok(())
    }

    fn validate_path(&self, identity: &ExternalSessionIdentity, path: &Path) -> io::Result<(PathBuf, PathBuf)> {
        check_absolute(path)?;
        let source = match &identity.source {
            SessionSource::File { relative_path } => relative_path,
            SessionSource::Sqlite { database, .. } => database,
        };
        let relative = Path::new(source);
        check_relative(relative)?;
        // Reject all symlinks, including links inside the allowed tree. This is
        // stricter than canonical containment and protects subsequent write paths.
        reject_symlinks(path)?;
        let metadata = fs::metadata(path)?;
        let legacy_opencode = identity.provider == SessionProvider::Opencode
            && matches!(&identity.source, SessionSource::File { relative_path }
                if relative_path == &format!("message/{}", identity.session_id.0));
        if !metadata.is_file() && !(legacy_opencode && metadata.is_dir()) {
            return Err(invalid("session source is not a supported file or message directory"));
        }
        let canonical = fs::canonicalize(path)?;
        for root in &self.roots {
            let expected = root.join(relative);
            if expected != path { continue; }
            reject_symlinks(root)?;
            let root = fs::canonicalize(root)?;
            if canonical.starts_with(&root) && canonical != root {
                return Ok((root, canonical));
            }
        }
        Err(forbidden())
    }
}
fn check_relative(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty() || path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(invalid("invalid relative source locator"));
    }
    Ok(())
}
fn check_absolute(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err(invalid("invalid host source path"));
    }
    Ok(())
}
fn reject_symlinks(path: &Path) -> io::Result<()> {
    let mut ancestor = PathBuf::new();
    for component in path.components() {
        ancestor.push(component.as_os_str());
        if fs::symlink_metadata(&ancestor)?.file_type().is_symlink() { return Err(forbidden()); }
    }
    Ok(())
}
fn forbidden() -> io::Error { io::Error::new(io::ErrorKind::PermissionDenied, "session source is not authorized") }
fn invalid(message: &'static str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
