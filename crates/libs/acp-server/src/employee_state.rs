use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use agent_client_protocol::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shiju_client::employee_conversations::EmployeeTarget;
use uuid::Uuid;

const MAX_STATE_BYTES: u64 = 32 * 1024;

pub(crate) struct StateStore {
    directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SessionState {
    pub cwd: PathBuf,
    pub cloud_id: Option<String>,
    pub pending: Option<PendingSend>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PendingSend {
    pub key: String,
    pub digest: String,
    pub confirmed: Option<ConfirmedSend>,
    pub cancel_requested: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ConfirmedSend {
    pub turn_id: String,
    pub user_message_id: String,
}

impl SessionState {
    pub fn new(cwd: PathBuf, cloud_id: Option<String>) -> Self {
        Self { cwd, cloud_id, pending: None }
    }
}

impl StateStore {
    pub fn open(root: &Path, account: &str, target: &EmployeeTarget) -> Result<Self> {
        if !root.is_absolute()
            || account.trim().is_empty()
            || matches!(target.employee_id.trim(), "" | "." | "..")
        {
            return Err(Error::new(
                -32602,
                "Invalid employee ACP account, target or state directory",
            ));
        }
        let scope = serde_json::to_vec(&(account, target)).map_err(|_| storage_error())?;
        let directory = root.join(digest(&scope));
        fs::create_dir_all(root).map_err(|_| storage_error())?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(storage_error()),
        }
        let metadata = fs::symlink_metadata(&directory).map_err(|_| storage_error())?;
        if !metadata.is_dir() {
            return Err(storage_error());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            // SAFETY: geteuid takes no pointers and reads the current identity.
            if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
                return Err(storage_error());
            }
        }
        let store = Self { directory: directory.canonicalize().map_err(|_| storage_error())? };
        tempfile::NamedTempFile::new_in(&store.directory).map_err(|_| storage_error())?;
        Ok(store)
    }

    pub fn cursor_scope(&self, cwd: Option<&Path>) -> String {
        // ACP CWD values must be representable as JSON strings.
        let scope =
            serde_json::to_vec(&(self.directory.to_string_lossy(), cwd.and_then(Path::to_str)))
                .expect("serializable cursor scope");
        digest(&scope)
    }

    pub fn lease(&self, id: &str) -> Result<File> {
        validate_id(id)?;
        let file = private_file()
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join(format!("{id}.lock")))
            .map_err(|_| storage_error())?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => Error::new(-32600, "Employee ACP session is busy"),
            std::fs::TryLockError::Error(_) => storage_error(),
        })?;
        Ok(file)
    }

    pub fn read(&self, id: &str) -> Result<Option<SessionState>> {
        validate_id(id)?;
        let file = match private_file().open(self.directory.join(format!("{id}.json"))) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(storage_error()),
        };
        if !file.metadata().map_err(|_| storage_error())?.is_file() {
            return Err(storage_error());
        }
        let mut bytes = Vec::new();
        file.take(MAX_STATE_BYTES + 1).read_to_end(&mut bytes).map_err(|_| storage_error())?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(storage_error());
        }
        serde_json::from_slice(&bytes).map(Some).map_err(|_| storage_error())
    }

    pub fn write(&self, id: &str, state: &SessionState) -> Result<()> {
        validate_id(id)?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| storage_error())?;
        let data = serde_json::to_vec(state).map_err(|_| storage_error())?;
        if data.len() as u64 > MAX_STATE_BYTES {
            return Err(storage_error());
        }
        temporary.write_all(&data).map_err(|_| storage_error())?;
        temporary.as_file().sync_all().map_err(|_| storage_error())?;
        temporary
            .persist(self.directory.join(format!("{id}.json")))
            .map_err(|_| storage_error())?;
        #[cfg(unix)]
        File::open(&self.directory).and_then(|dir| dir.sync_all()).map_err(|_| storage_error())?;
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<(String, SessionState)>> {
        let mut sessions = Vec::new();
        for entry in fs::read_dir(&self.directory).map_err(|_| storage_error())? {
            let entry = entry.map_err(|_| storage_error())?;
            let name = entry.file_name();
            let Some(id) = name.to_str().and_then(|name| name.strip_suffix(".json")) else {
                continue;
            };
            if validate_id(id).is_ok()
                && let Some(state) = self.read(id)?
            {
                sessions.push((id.to_owned(), state));
            }
        }
        sessions.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Ok(sessions)
    }
}

fn private_file() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options
}

pub(crate) fn cloud_id(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|uuid| uuid.hyphenated().to_string() == id)
}

fn validate_id(id: &str) -> Result<()> {
    if cloud_id(id.strip_prefix("local-").unwrap_or(id)) { Ok(()) } else { Err(missing_session()) }
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn missing_session() -> Error {
    Error::new(-32602, "Employee ACP session is unavailable in this account, target or directory")
}

fn storage_error() -> Error {
    Error::new(
        -32603,
        "Employee ACP state directory is inaccessible or its state could not be saved",
    )
}
