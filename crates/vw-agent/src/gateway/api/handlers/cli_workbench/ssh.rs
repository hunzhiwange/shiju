//! Saved connections and OpenSSH processes owned by the gateway.
use std::{
    collections::{BTreeSet, HashSet},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path as FilePath, PathBuf},
    process::Command,
};

use vw_api_types::cli_workbench::{
    SshConnection, SshConnectionChange, SshConnectionFields, SshConnections,
};

use super::*;

const MAX_CONNECTIONS: usize = 256;
static STORE: Mutex<()> = Mutex::new(());

pub(super) fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/cli-workbench/ssh", get(list).post(change))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024))
}

async fn list(
    authority: terminal::Authority,
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<SshConnections>, WorkbenchError> {
    authority.require_local_pty()?;
    tokio::task::spawn_blocking(move || catalog(&tenant, None))
        .await
        .map_err(|_| unavailable())?
        .map(Json)
}

async fn change(
    authority: terminal::Authority,
    Extension(tenant): Extension<TenantScope>,
    Json(change): Json<SshConnectionChange>,
) -> Result<Json<SshConnections>, WorkbenchError> {
    authority.require_local_pty()?;
    tokio::task::spawn_blocking(move || catalog(&tenant, Some(change)))
        .await
        .map_err(|_| unavailable())?
        .map(Json)
}

fn catalog(
    tenant: &TenantScope,
    change: Option<SshConnectionChange>,
) -> Result<SshConnections, WorkbenchError> {
    executable()?;
    let aliases = discover_aliases(&agent_paths::paths().home);
    let _guard = STORE.lock();
    let path = store_path(tenant);
    let mut saved = load(&path)?;
    if let Some(change) = change {
        apply_change(&mut saved, &aliases, change)?;
        saved.sort_by(|a, b| a.fields.name.cmp(&b.fields.name).then_with(|| a.id.cmp(&b.id)));
        persist(&path, &saved).map_err(|_| unavailable())?;
    }
    let aliases = aliases.into_iter().filter(|alias| !imported(&saved, alias)).collect();
    Ok(SshConnections { saved, aliases })
}

fn store_path(tenant: &TenantScope) -> PathBuf {
    let mut scope = Sha256::new();
    for part in
        [tenant.platform_id().as_ref(), tenant.company_id().as_ref(), tenant.user_id().as_ref()]
    {
        scope.update(part.len().to_be_bytes());
        scope.update(part.as_bytes());
    }
    let digest = hex::encode(scope.finalize());
    agent_paths::paths().config.join(format!("ssh-connections-{digest}.json"))
}

fn load(path: &FilePath) -> Result<Vec<SshConnection>, WorkbenchError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| unavailable()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(_) => Err(unavailable()),
    }
}

fn persist(path: &FilePath, saved: &[SshConnection]) -> io::Result<()> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut file, saved)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

fn apply_change(
    saved: &mut Vec<SshConnection>,
    discovered: &BTreeSet<String>,
    change: SshConnectionChange,
) -> Result<(), WorkbenchError> {
    match change {
        SshConnectionChange::Save { id, mut fields } => {
            if let Some(id) = id {
                let connection =
                    saved.iter_mut().find(|entry| entry.id == id).ok_or_else(not_found)?;
                if connection.from_config {
                    fields.host.clone_from(&connection.fields.host);
                    fields.port = None;
                    fields.identity_file = None;
                }
                validate(&mut fields)?;
                connection.fields = fields;
            } else {
                validate(&mut fields)?;
                saved.push(new_connection(fields, false));
            }
        }
        SshConnectionChange::Import { aliases } => {
            if aliases.is_empty() || aliases.len() > MAX_CONNECTIONS {
                return Err(invalid());
            }
            for alias in aliases {
                if imported(saved, &alias) {
                    continue;
                }
                if !discovered.contains(&alias) {
                    return Err(not_found());
                }
                saved.push(new_connection(
                    SshConnectionFields {
                        name: alias.clone(),
                        host: alias,
                        port: None,
                        identity_file: None,
                    },
                    true,
                ));
            }
        }
        SshConnectionChange::Delete { id } => saved.retain(|entry| entry.id != id),
    }
    if saved.len() > MAX_CONNECTIONS {
        return Err(invalid());
    }
    Ok(())
}

fn imported(saved: &[SshConnection], alias: &str) -> bool {
    saved.iter().any(|entry| entry.from_config && entry.fields.host.eq_ignore_ascii_case(alias))
}

fn new_connection(fields: SshConnectionFields, from_config: bool) -> SshConnection {
    SshConnection { id: uuid::Uuid::new_v4().to_string(), fields, from_config }
}

fn invalid() -> WorkbenchError {
    error(StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest, "invalid SSH connection")
}

fn validate(fields: &mut SshConnectionFields) -> Result<(), WorkbenchError> {
    fields.name = fields.name.trim().to_owned();
    fields.host = fields.host.trim().to_owned();
    if fields.name.is_empty()
        || fields.name.len() > 128
        || fields.name.chars().any(char::is_control)
        || !valid_destination(&fields.host)
        || fields.port == Some(0)
    {
        return Err(invalid());
    }
    if let Some(path) = &fields.identity_file
        && (path.len() > 4096
            || !(path.starts_with('/') || path.strip_prefix("~/").is_some_and(|p| !p.is_empty()))
            || path.chars().any(|ch| ch.is_control() || matches!(ch, '$' | '%')))
    {
        return Err(invalid());
    }
    Ok(())
}

fn valid_destination(destination: &str) -> bool {
    let host = if let Some((user, host)) = destination.split_once('@') {
        if !valid_host_token(user) {
            return false;
        }
        host
    } else {
        destination
    };
    let ipv6 = host.strip_prefix('[').and_then(|host| host.strip_suffix(']')).unwrap_or(host);
    valid_host_token(host) || ipv6.parse::<std::net::Ipv6Addr>().is_ok()
}

fn valid_host_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 253
        && !token.starts_with('-')
        && token.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn executable() -> Result<&'static FilePath, WorkbenchError> {
    [FilePath::new("/usr/bin/ssh"), FilePath::new("/bin/ssh")]
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(unavailable)
}

pub(super) fn command(
    tenant: &TenantScope,
    id: &str,
    cwd: &str,
) -> Result<Command, WorkbenchError> {
    let mut connection = {
        let _guard = STORE.lock();
        load(&store_path(tenant))?.into_iter().find(|entry| entry.id == id).ok_or_else(not_found)?
    };
    validate(&mut connection.fields)?;
    if connection.from_config
        && !discover_aliases(&agent_paths::paths().home)
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(&connection.fields.host))
    {
        return Err(not_found());
    }
    let SshConnectionFields { host, port, identity_file, .. } = connection.fields;
    let mut command = Command::new(executable()?);
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin:/opt/homebrew/bin")
        .env("HOME", &agent_paths::paths().home)
        .current_dir(cwd)
        .args([
            "-tt",
            "-e",
            "none",
            "-o",
            "StrictHostKeyChecking=ask",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ForwardX11=no",
            "-o",
            "PermitLocalCommand=no",
            "-o",
            "ControlPath=none",
            "-o",
            "ConnectionAttempts=1",
            "-o",
            "ConnectTimeout=15",
            "-o",
            "RemoteCommand=none",
            "-o",
            "SessionType=default",
        ]);
    for key in ["SSH_AUTH_SOCK", "USER", "LOGNAME", "LANG", "LC_ALL", "LC_CTYPE"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if let Some(port) = port {
        command.arg("-p").arg(port.to_string());
    }
    if let Some(path) = identity_file {
        command.arg("-i").arg(path).args(["-o", "IdentitiesOnly=yes"]);
    }
    // IPv6 brackets accepted by the manager are for display, not OpenSSH lookup.
    let destination = host.replace(['[', ']'], "");
    command.arg("--").arg(destination);
    Ok(command)
}

// As in Codex's SSH config dependency reader, resolve relative Includes from
// ~/.ssh and guard recursion by canonical path. OpenSSH applies the config at launch.
fn discover_aliases(home: &FilePath) -> BTreeSet<String> {
    let base = home.join(".ssh");
    let mut aliases = BTreeSet::new();
    read_aliases(&base.join("config"), &base, home, 0, &mut HashSet::new(), &mut aliases);
    let mut seen = HashSet::new();
    aliases.retain(|alias| seen.insert(alias.to_ascii_lowercase()));
    aliases
}

fn read_aliases(
    path: &FilePath,
    base: &FilePath,
    home: &FilePath,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    aliases: &mut BTreeSet<String>,
) {
    if depth >= 16 || visited.len() >= MAX_CONNECTIONS || aliases.len() >= MAX_CONNECTIONS {
        return;
    }
    let Ok(path) = path.canonicalize() else { return };
    if !path.is_file() || !visited.insert(path.clone()) {
        return;
    }
    let Ok(file) = File::open(path) else { return };
    let mut contents = String::new();
    if file.take(1024 * 1024).read_to_string(&mut contents).is_err() {
        return;
    }
    for line in contents.lines() {
        let line = line.trim_start();
        let end = line.find(|ch: char| ch.is_whitespace() || ch == '=').unwrap_or(line.len());
        let (key, rest) = line.split_at(end);
        let rest = rest.trim_start().strip_prefix('=').unwrap_or(rest);
        if key.eq_ignore_ascii_case("Host") {
            for alias in shell_words::split(rest).unwrap_or_default() {
                if valid_host_token(&alias) && aliases.len() < MAX_CONNECTIONS {
                    aliases.insert(alias);
                }
            }
        } else if key.eq_ignore_ascii_case("Include") {
            for entry in shell_words::split(rest).unwrap_or_default() {
                let pattern = if let Some(relative) = entry.strip_prefix("~/") {
                    home.join(relative)
                } else {
                    base.join(entry)
                };
                if let Some(matches) = pattern.to_str().and_then(|p| glob::glob(p).ok()) {
                    for include in matches.take(MAX_CONNECTIONS).flatten() {
                        read_aliases(&include, base, home, depth + 1, visited, aliases);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod tests;
