use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
const SHIJU_CLI_NAME: &str = "shiju.exe";
#[cfg(not(windows))]
const SHIJU_CLI_NAME: &str = "shiju";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillCacheSyncReport {
    pub(crate) fetch_exit_code: i32,
    pub(crate) rebuild_exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCacheSyncError {
    message: String,
}

impl SkillCacheSyncError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for SkillCacheSyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SkillCacheSyncError {}

pub(super) fn resolve_shiju_cli(installation_root: &Path) -> Result<PathBuf, SkillCacheSyncError> {
    validate_directory(installation_root, "识矩技能安装目录")?;
    let bin_dir = installation_root.join("bin");
    validate_directory(&bin_dir, "识矩 CLI 目录")?;
    let executable = bin_dir.join(SHIJU_CLI_NAME);
    let metadata = fs::symlink_metadata(&executable).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            SkillCacheSyncError::new("识矩 CLI 不存在，请重新安装主识矩技能")
        } else {
            SkillCacheSyncError::new("无法检查识矩 CLI")
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(SkillCacheSyncError::new("识矩 CLI 不能是符号链接"));
    }
    if !metadata.is_file() {
        return Err(SkillCacheSyncError::new("识矩 CLI 不是普通文件"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(SkillCacheSyncError::new("识矩 CLI 不可执行"));
        }
    }
    let canonical_root = fs::canonicalize(installation_root)
        .map_err(|_| SkillCacheSyncError::new("无法验证识矩技能安装目录"))?;
    let canonical_executable =
        fs::canonicalize(&executable).map_err(|_| SkillCacheSyncError::new("无法验证识矩 CLI"))?;
    if !canonical_executable.starts_with(&canonical_root) {
        return Err(SkillCacheSyncError::new("识矩 CLI 越出已安装主识矩技能目录"));
    }
    Ok(canonical_executable)
}

fn validate_directory(path: &Path, label: &str) -> Result<(), SkillCacheSyncError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            SkillCacheSyncError::new(format!("{label}不存在"))
        } else {
            SkillCacheSyncError::new(format!("无法检查{label}"))
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(SkillCacheSyncError::new(format!("{label}不能包含符号链接")));
    }
    if !metadata.is_dir() {
        return Err(SkillCacheSyncError::new(format!("{label}不是目录")));
    }
    Ok(())
}

pub(super) fn build_cache_sync_command(
    executable: &Path,
    installation_root: &Path,
    arguments: &[&str],
) -> Command {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(installation_root)
        .env_remove("SHIJU_KEY")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(crate) async fn sync_business_skill_cache(
    installation_root: &Path,
    skill_id: u64,
) -> Result<SkillCacheSyncReport, SkillCacheSyncError> {
    let executable = resolve_shiju_cli(installation_root)?;
    let installation_root = installation_root.to_path_buf();
    let fetch_skill_id = skill_id.to_string();
    let fetch_exit_code = run_step(
        executable.clone(),
        installation_root.clone(),
        vec!["skill:fetch".to_string(), "--skill-id".to_string(), fetch_skill_id],
        "skill:fetch",
    )
    .await?;
    let rebuild_exit_code = run_step(
        executable,
        installation_root,
        vec!["index".to_string(), "rebuild".to_string()],
        "索引重建",
    )
    .await?;
    Ok(SkillCacheSyncReport { fetch_exit_code, rebuild_exit_code })
}

async fn run_step(
    executable: PathBuf,
    installation_root: PathBuf,
    arguments: Vec<String>,
    label: &'static str,
) -> Result<i32, SkillCacheSyncError> {
    let output = tokio::task::spawn_blocking(move || {
        let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        build_cache_sync_command(&executable, &installation_root, &arguments).output()
    })
    .await
    .map_err(|_| SkillCacheSyncError::new(format!("{label}任务异常终止")))?
    .map_err(|_| SkillCacheSyncError::new(format!("无法启动{label}")))?;
    let exit_code = output.status.code().unwrap_or(-1);
    if !output.status.success() {
        return Err(SkillCacheSyncError::new(format!("{label}退出码为 {exit_code}")));
    }
    validate_success_json(&output.stdout, label)?;
    Ok(exit_code)
}

fn validate_success_json(stdout: &[u8], label: &str) -> Result<(), SkillCacheSyncError> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|_| SkillCacheSyncError::new(format!("{label}未返回有效 JSON")))?;
    let object = value
        .as_object()
        .ok_or_else(|| SkillCacheSyncError::new(format!("{label}返回的 JSON 不是对象")))?;
    let explicit_failure = object.get("success").is_some_and(|value| value == false)
        || object.get("ok").is_some_and(|value| value == false)
        || object
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "error" | "failed"))
        || object.get("error").is_some_and(|error| !error.is_null());
    if explicit_failure {
        return Err(SkillCacheSyncError::new(format!("{label}报告业务失败")));
    }
    Ok(())
}
