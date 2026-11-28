use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use git2::{Config, ConfigLevel, Cred, Error, Repository};
use ssh2_config::{HostParams, ParseRule, SshConfig};

pub(super) struct Credentials {
    agent: bool,
    keys: std::vec::IntoIter<PathBuf>,
    user: Option<String>,
}

impl Credentials {
    pub(super) fn username(&self) -> &str {
        self.user.as_deref().unwrap_or("git")
    }

    pub(super) fn next(&mut self, username: &str) -> Result<Cred, Error> {
        if std::mem::take(&mut self.agent) {
            return Cred::ssh_key_from_agent(username);
        }
        if let Some(key) = self.keys.next() {
            return Cred::ssh_key(username, None, &key, None);
        }
        Err(Error::from_str("SSH 认证失败，请检查网关主机的 IdentityFile 或 SSH agent"))
    }
}

struct Endpoint<'a> {
    scheme: Option<&'a str>,
    user: Option<&'a str>,
    host: &'a str,
    port: Option<u16>,
    path: &'a str,
}

impl<'a> Endpoint<'a> {
    fn parse(url: &'a str) -> Option<Self> {
        let (scheme, authority, path) = match url.split_once("://") {
            Some((scheme @ ("ssh" | "git+ssh" | "ssh+git"), rest)) => {
                let (authority, path) = rest.split_once('/')?;
                (Some(scheme), authority, path)
            }
            Some(_) => return None,
            None => {
                // SCP 风格的仓库路径可以是相对路径，解析时保留原样。
                let authority_end = match url.find('[') {
                    Some(start) if !url[..start].contains([':', '/', '\\']) => url.find(']')? + 1,
                    _ => 0,
                };
                let separator = authority_end + url[authority_end..].find(':')?;
                let (authority, path) = (&url[..separator], &url[separator + 1..]);
                if authority.contains(['/', '\\'])
                    || path.starts_with(':')
                    || (cfg!(windows) && authority.len() == 1)
                {
                    return None;
                }
                (None, authority, path)
            }
        };
        let (user, host) = match authority.rsplit_once('@') {
            Some((user, host)) if !user.contains([':', '@']) => (Some(user), host),
            Some(_) => return None,
            None => (None, authority),
        };
        let (host, port) = if let Some(bracketed) = host.strip_prefix('[') {
            let (host, suffix) = bracketed.split_once(']')?;
            if suffix.is_empty() {
                (host, None)
            } else {
                (host, Some(suffix.strip_prefix(':')?.parse().ok()?))
            }
        } else if let Some((host, port)) = host.split_once(':') {
            (host, Some(port.parse().ok()?))
        } else {
            (host, None)
        };
        (!host.is_empty() && !path.is_empty()).then_some(Self { scheme, user, host, port, path })
    }

    fn resolve(&self, config: &HostParams) -> Result<Option<String>, Error> {
        let host = config.host_name.as_deref().unwrap_or(self.host).replace("%h", self.host);
        let port = self.port.or(config.port);
        if host == self.host && port == self.port {
            return Ok(None);
        }
        let host = if host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{host}]")
        } else if !host.is_empty()
            && host.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        {
            host
        } else {
            return Err(Error::from_str("SSH 配置中的 HostName 无法解析"));
        };
        let user = self.user.map_or_else(String::new, |user| format!("{user}@"));
        let authority = format!("{user}{host}");
        Ok(Some(match (self.scheme, port) {
            (Some(scheme), Some(port)) => format!("{scheme}://{authority}:{port}/{}", self.path),
            (Some(scheme), None) => format!("{scheme}://{authority}/{}", self.path),
            // libgit2 接受 [user@host:port]:path，保留原 SCP 路径语义。
            (None, Some(port)) => format!("[{authority}:{port}]:{}", self.path),
            (None, None) => format!("{authority}:{}", self.path),
        }))
    }
}

pub(super) fn settings(url: &[u8]) -> Result<(Credentials, Option<String>), Error> {
    let mut credentials = Credentials { agent: true, keys: Vec::new().into_iter(), user: None };
    let Some(endpoint) = std::str::from_utf8(url).ok().and_then(Endpoint::parse) else {
        return Ok((credentials, None));
    };
    let Some(home) = std::env::home_dir() else {
        return Ok((credentials, None));
    };
    let ssh_dir = home.join(".ssh");
    let config = match File::open(ssh_dir.join("config")) {
        Ok(file) => SshConfig::default()
            .parse(
                &mut BufReader::new(file),
                ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS,
            )
            .map_err(|_| Error::from_str("无法解析网关主机的 SSH 配置"))?,
        Err(error) if error.kind() == ErrorKind::NotFound => SshConfig::default(),
        Err(_) => return Err(Error::from_str("无法读取网关主机的 SSH 配置")),
    };
    let params = config.query(endpoint.host.to_ascii_lowercase());
    let configured = |name: &str| {
        params
            .ignored_fields
            .iter()
            .chain(&params.unsupported_fields)
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .and_then(|(_, values)| values.first())
            .map(String::as_str)
    };
    if params
        .proxy_jump
        .as_ref()
        .is_some_and(|hosts| hosts.iter().any(|host| !host.eq_ignore_ascii_case("none")))
        || configured("ProxyCommand").is_some_and(|value| !value.eq_ignore_ascii_case("none"))
        || ["Match", "HostKeyAlias", "UserKnownHostsFile", "GlobalKnownHostsFile"]
            .iter()
            .any(|name| configured(name).is_some())
        || configured("IdentityAgent")
            .is_some_and(|value| !value.eq_ignore_ascii_case("none") && value != "SSH_AUTH_SOCK")
    {
        return Err(Error::from_str("此 SSH 主机的代理、认证或主机密钥配置超出内置 SSH 支持范围"));
    }
    credentials.agent = params.pubkey_authentication != Some(false)
        && !configured("IdentitiesOnly").is_some_and(|value| value.eq_ignore_ascii_case("yes"))
        && !configured("IdentityAgent").is_some_and(|value| value.eq_ignore_ascii_case("none"));
    credentials.user = params.user.clone();
    if params.pubkey_authentication != Some(false) {
        let keys = params.identity_file.as_ref().map_or_else(
            || ["id_rsa", "id_ecdsa", "id_ed25519"].map(|name| ssh_dir.join(name)).to_vec(),
            |paths| paths.iter().filter_map(|path| identity_path(&home, path)).collect(),
        );
        credentials.keys =
            keys.into_iter().filter(|path| path.is_file()).collect::<Vec<_>>().into_iter();
    }
    Ok((credentials, endpoint.resolve(&params)?))
}

fn identity_path(home: &Path, path: &Path) -> Option<PathBuf> {
    if path == Path::new("none") {
        return None;
    }
    if let Ok(relative) = path.strip_prefix("~") {
        return Some(home.join(relative));
    }
    path.is_absolute().then(|| path.to_owned())
}

pub(super) fn override_endpoint(
    repo: &Repository,
    name: &str,
    pushing: bool,
    endpoint: &str,
) -> Result<tempfile::TempDir, Error> {
    // 仅覆盖当前 Repository 实例，保留命名远端的 refspec 与跟踪分支。
    let temp = tempfile::tempdir().map_err(|_| Error::from_str("无法创建临时 Git 配置"))?;
    let path = temp.path().join("config");
    let key = if pushing { "pushurl" } else { "url" };
    Config::open(&path)?.set_str(&format!("remote.{name}.{key}"), endpoint)?;
    repo.config()?.add_file(&path, ConfigLevel::App, false)?;
    Ok(temp)
}

#[cfg(test)]
#[path = "remote_ssh_tests.rs"]
mod tests;
