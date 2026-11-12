//! Run only saved project commands, with durable progress in Git's checkout metadata.
use super::Error;
use crate::shell::tokio_system_command;
use crate::tools::shell::processes::termination;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Status {
    #[default]
    Pending,
    Running,
    Error,
    Ready,
}

#[derive(Default, Serialize, Deserialize)]
struct Progress {
    status: Status,
    project_done: bool,
    extra_done: bool,
    extra_command: String,
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}
fn state_path(directory: &str) -> Result<PathBuf, Error> {
    let repo = git2::Repository::open_ext(
        directory,
        git2::RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )
    .map_err(invalid)?;
    Ok(repo.path().join("shiju-environment.json"))
}
fn read(path: &Path) -> Result<Progress, Error> {
    serde_json::from_slice(&std::fs::read(path)?).map_err(invalid)
}
fn save(path: &Path, progress: &Progress) -> Result<(), Error> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(&serde_json::to_vec(progress).map_err(invalid)?)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

/// Called under the existing repository lifecycle lock, before the checkout is exposed.
pub(super) fn pending(directory: &str) -> Result<(), Error> {
    save(&state_path(directory)?, &Progress::default())
}

/// Read checkout initialization status; existing/external trees have no managed setup state.
pub fn status(directory: &str) -> Result<Option<Status>, Error> {
    if directory.is_empty() {
        return Ok(None);
    }
    let path = match git2::Repository::discover(directory) {
        Ok(repo) => repo.path().join("shiju-environment.json"),
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(error) => return Err(invalid(error)),
    };
    match std::fs::metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
        Ok(_) => Ok(Some(read(&path)?.status)),
    }
}

/// Block chat creation until all saved setup stages succeed.
pub fn ensure_ready(directory: &str) -> Result<(), Error> {
    match status(directory)? {
        None | Some(Status::Ready) => Ok(()),
        Some(_) => {
            Err(invalid("Worktree environment is not ready; retry failed initialization first"))
        }
    }
}

// Persist cancellation as Error as well as ordinary failures; child/tree drop kills descendants.
struct Attempt {
    path: PathBuf,
    progress: Progress,
    directory: String,
    project_id: String,
    interrupted: bool,
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if self.progress.status == Status::Running {
            self.progress.status = Status::Error;
            let _ = save(&self.path, &self.progress);
            if self.interrupted {
                let _ = crate::bus::publish(
                    super::event::FAILED,
                    serde_json::json!({"projectId": self.project_id, "message": "Worktree initialization was interrupted"}),
                    Some(self.directory.clone()),
                );
            }
        }
    }
}

pub(super) async fn prepare(directory: &str, project_id: &str, extra: &str) -> Result<(), Error> {
    initialize(directory, project_id, extra, false).await
}
pub(super) async fn retry(directory: &str, project_id: &str) -> Result<(), Error> {
    initialize(directory, project_id, "", true).await
}
async fn initialize(
    directory: &str,
    project_id: &str,
    extra: &str,
    retry: bool,
) -> Result<(), Error> {
    let path = state_path(directory)?;
    // Independent OS lock also protects direct retries and duplicate requests.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))?;
    lock.try_lock().map_err(|_| invalid("Worktree environment initialization is busy"))?;
    let mut progress = match std::fs::metadata(&path) {
        Ok(_) => read(&path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !retry => Progress::default(),
        Err(error) => return Err(error.into()),
    };
    if progress.status == Status::Ready {
        return Ok(());
    }
    if (retry && progress.status != Status::Error) || (!retry && progress.status != Status::Pending)
    {
        return Err(invalid("Only failed environment initialization can be retried"));
    }
    if progress.status == Status::Pending {
        progress.extra_command = extra.to_owned();
    }
    let mut attempt = Attempt {
        path,
        progress,
        directory: directory.into(),
        project_id: project_id.into(),
        interrupted: true,
    };
    let result = async {
        attempt.progress.status = Status::Running;
        save(&attempt.path, &attempt.progress)?;
        if !attempt.progress.project_done {
            let command = super::native::project_start(project_id).await?;
            run_command(directory, &command, TIMEOUT).await?;
            attempt.progress.project_done = true;
            save(&attempt.path, &attempt.progress)?;
        }
        if !attempt.progress.extra_done {
            run_command(directory, &attempt.progress.extra_command, TIMEOUT).await?;
            attempt.progress.extra_done = true;
            save(&attempt.path, &attempt.progress)?;
        }
        attempt.progress.extra_command.clear();
        attempt.progress.status = Status::Ready;
        if let Err(error) = save(&attempt.path, &attempt.progress) {
            attempt.progress.status = Status::Running;
            return Err(error);
        }
        Ok(())
    }
    .await;
    attempt.interrupted = false;
    result
}

pub(super) async fn run_command(
    directory: &str,
    command: &str,
    timeout: Duration,
) -> Result<(), Error> {
    if command.trim().is_empty() {
        return Ok(());
    }
    // Preserve the existing saved-command execution boundary and shell selection.
    let mut process = if cfg!(windows) {
        let mut process = tokio_system_command("cmd");
        process.arg("/c").arg(command);
        process
    } else {
        let mut process = tokio_system_command("bash");
        process.arg("-lc").arg(command);
        process
    };
    process
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let (mut child, tree) = termination::spawn(&mut process).map_err(invalid)?;
    let result = tokio::time::timeout(timeout, child.wait()).await;
    drop(tree);
    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(invalid(format!(
            "Worktree initialization command failed (exit {:?})",
            status.code()
        ))),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => {
            let _ = child.wait().await;
            Err(invalid("Worktree initialization command timed out"))
        }
    }
}
