//! 校验识矩 CLI 安装边界并同步本地业务技能缓存。

use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SHIJU_CLI_NAME: &str = "shiju";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 技能缓存拉取与重建两个步骤的退出码。
pub(crate) struct SkillCacheSyncReport {
    /// 远端索引拉取命令的退出码。
    pub(crate) fetch_exit_code: i32,
    /// 本地索引重建命令的退出码。
    pub(crate) rebuild_exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// CLI 解析、执行或结构化输出校验失败。
pub(crate) struct SkillCacheSyncError {
    source: String,
    label: Option<String>,
    value: Option<String>,
}

impl SkillCacheSyncError {
    pub(crate) fn new(source: impl Into<String>) -> Self {
        Self { source: source.into(), label: None, value: None }
    }

    fn labeled(source: &str, label: &str, value: Option<String>) -> Self {
        Self { source: source.into(), label: Some(label.into()), value }
    }

    pub(crate) fn message(&self) -> crate::app::i18n::format::Message {
        use crate::app::i18n::format::Message;
        if let Some(label) = &self.label {
            let mut arguments = vec![Message::new(label.clone(), vec![])];
            if let Some(value) = &self.value {
                arguments.push(Message::literal(value.clone()));
            }
            Message::nested(self.source.clone(), arguments)
        } else {
            Message::new(self.source.clone(), vec![])
        }
    }
}

impl fmt::Display for SkillCacheSyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .message()
            .render(crate::app::i18n::I18n::new(
                vw_config_types::ui::DesktopLocale::SimplifiedChinese,
            ))
            .map_err(|_| fmt::Error)?;
        formatter.write_str(&text)
    }
}

impl From<String> for SkillCacheSyncError {
    fn from(source: String) -> Self {
        Self::new(source)
    }
}
impl From<&str> for SkillCacheSyncError {
    fn from(source: &str) -> Self {
        Self::new(source)
    }
}

impl std::error::Error for SkillCacheSyncError {}

/// 在已验证的安装目录中解析非符号链接识矩 CLI。
pub(super) fn resolve_shiju_cli(installation_root: &Path) -> Result<PathBuf, SkillCacheSyncError> {
    validate_directory(installation_root, "识矩技能安装目录")?;
    let scripts_dir = installation_root.join("scripts");
    validate_directory(&scripts_dir, "识矩 CLI 目录")?;
    let executable = scripts_dir.join(SHIJU_CLI_NAME);
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
            SkillCacheSyncError::labeled("%s不存在", label, None)
        } else {
            SkillCacheSyncError::labeled("无法检查%s", label, None)
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(SkillCacheSyncError::labeled("%s不能包含符号链接", label, None));
    }
    if !metadata.is_dir() {
        return Err(SkillCacheSyncError::labeled("%s不是目录", label, None));
    }
    Ok(())
}

/// 构造关闭交互输入且移除密钥环境变量的缓存同步子进程。
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

/// 依次执行索引拉取和缓存重建，并校验每步的 JSON 成功结果。
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
    .map_err(|_| SkillCacheSyncError::labeled("%s任务异常终止", label, None))?
    .map_err(|_| SkillCacheSyncError::labeled("无法启动%s", label, None))?;
    let exit_code = output.status.code().unwrap_or(-1);
    if !output.status.success() {
        return Err(SkillCacheSyncError::labeled(
            "%s退出码为 %s",
            label,
            Some(exit_code.to_string()),
        ));
    }
    validate_success_json(&output.stdout, label)?;
    Ok(exit_code)
}

fn validate_success_json(stdout: &[u8], label: &str) -> Result<(), SkillCacheSyncError> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|_| SkillCacheSyncError::labeled("%s未返回有效 JSON", label, None))?;
    let object = value
        .as_object()
        .ok_or_else(|| SkillCacheSyncError::labeled("%s返回的 JSON 不是对象", label, None))?;
    let explicit_failure = object.get("success").is_some_and(|value| value == false)
        || object.get("ok").is_some_and(|value| value == false)
        || object
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "error" | "failed"))
        || object.get("error").is_some_and(|error| !error.is_null());
    if explicit_failure {
        return Err(SkillCacheSyncError::labeled("%s报告业务失败", label, None));
    }
    Ok(())
}
