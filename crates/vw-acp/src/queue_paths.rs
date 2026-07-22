//! 队列目录、锁文件与套接字路径的统一计算。

use std::env;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[cfg(test)]
pub(crate) static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn short_hash(value: &str, length: usize) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hex = format!("{digest:x}");
    hex[..length].to_string()
}

pub fn queue_key_for_session(session_id: &str) -> String {
    short_hash(session_id, 24)
}

pub fn default_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(path) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(path));
        }

        let home_drive = env::var_os("HOMEDRIVE")?;
        let home_path = env::var_os("HOMEPATH")?;
        let mut joined = PathBuf::from(home_drive);
        joined.push(home_path);
        Some(joined)
    }

    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(PathBuf::from)
    }
}

pub fn queue_base_dir(home_dir: impl AsRef<Path>) -> PathBuf {
    vw_config_types::paths::home_config_dir(home_dir).join("acp").join("queues")
}

pub fn default_queue_base_dir() -> Option<PathBuf> {
    default_home_dir().map(queue_base_dir)
}

pub fn queue_socket_base_dir(home_dir: impl AsRef<Path>) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let _ = home_dir;
        None
    }

    #[cfg(not(windows))]
    {
        Some(
            env::temp_dir()
                .join(format!("vwacp-{}", short_hash(&home_dir.as_ref().to_string_lossy(), 10))),
        )
    }
}

pub fn default_queue_socket_base_dir() -> Option<PathBuf> {
    default_home_dir().and_then(queue_socket_base_dir)
}

pub fn queue_lock_file_path(session_id: &str, home_dir: impl AsRef<Path>) -> PathBuf {
    queue_base_dir(home_dir).join(format!("{}.lock", queue_key_for_session(session_id)))
}

pub fn default_queue_lock_file_path(session_id: &str) -> Option<PathBuf> {
    default_home_dir().map(|home_dir| queue_lock_file_path(session_id, home_dir))
}

pub fn queue_socket_path(session_id: &str, home_dir: impl AsRef<Path>) -> PathBuf {
    let key = queue_key_for_session(session_id);

    #[cfg(windows)]
    {
        let _ = home_dir;
        PathBuf::from(format!(r"\\.\pipe\vwacp-{key}"))
    }

    #[cfg(not(windows))]
    {
        queue_socket_base_dir(home_dir).unwrap_or_else(env::temp_dir).join(format!("{key}.sock"))
    }
}

pub fn default_queue_socket_path(session_id: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(queue_socket_path(session_id, PathBuf::new()))
    }

    #[cfg(not(windows))]
    {
        default_home_dir().map(|home_dir| queue_socket_path(session_id, home_dir))
    }
}

#[cfg(test)]
#[path = "queue_paths_tests.rs"]
mod queue_paths_tests;
