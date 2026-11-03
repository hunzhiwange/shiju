//! 安全下载、校验并安装识矩技能包到受支持的目标目录。

use super::SkillDownloadUrl;
use crate::app::account::SecretSkey;
use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use reqwest::redirect::Policy;
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tempfile::{Builder, NamedTempFile, TempDir};
use vw_config_types::paths::{HOME_CONFIG_DIR_NAME, home_config_dir, tilde_config_path};
use zip::ZipArchive;

const MAX_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;
const SKILL_DIR_NAME: &str = "shiju";
const SKILL_MANIFEST: &str = "SKILL.md";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const SHIJU_ENV_FILE_NAME: &str = ".env";
const SHIJU_KEY_PREFIX: &str = "SHIJU_KEY=";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// 技能安装相对于用户主目录或当前项目生效的范围。
pub(crate) enum SkillInstallScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// 识矩技能包支持写入的代理工具目录。
pub(crate) enum SkillInstallTarget {
    VibeWindow,
    AgentSkills,
    Codex,
    WorkBuddy,
    ClaudeCode,
    OpenCode,
    GeminiCli,
    GitHubCopilot,
    Cursor,
    Windsurf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// 一个安装目标及其全局或项目范围选择。
pub(crate) struct SkillInstallSelection {
    pub(crate) target: SkillInstallTarget,
    pub(crate) scope: SkillInstallScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 已解析并校验到受支持根目录内的安装目的地。
pub(crate) struct ResolvedSkillDestination {
    pub(crate) target: SkillInstallTarget,
    pub(crate) scope: SkillInstallScope,
    pub(crate) skills_root: PathBuf,
    pub(crate) installed_dir: PathBuf,
    base_dir: PathBuf,
    relative_skills_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 单目标安装完成后的路径与版本信息。
pub(crate) struct ShijuInstallReport {
    pub(crate) installed_dir: PathBuf,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 多目标安装中单个选择的成功结果。
pub(crate) struct ShijuInstallItem {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 多目标安装的逐项成功结果。
pub(crate) struct ShijuMultiInstallReport {
    pub(crate) installs: Vec<ShijuInstallItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 已存在识矩安装的根目录与可选版本。
pub(crate) struct ShijuInstallation {
    pub(crate) installed_dir: PathBuf,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 指定目标选择对应的已安装状态。
pub(crate) struct ShijuTargetInstallation {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) installation: Option<ShijuInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 所有目标的安装探测结果。
pub(crate) struct ShijuInstallationsReport {
    pub(crate) targets: Vec<ShijuTargetInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 单个目标卸载的结果。
pub(crate) struct ShijuTargetUninstall {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 多目标卸载的逐项报告。
pub(crate) struct ShijuUninstallReport {
    pub(crate) targets: Vec<ShijuTargetUninstall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 密钥同步是写入新值还是发现技能尚未安装。
pub(crate) enum ShijuKeySyncOutcome {
    Updated,
    SkillNotInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 单个安装目标的密钥同步结果。
pub(crate) struct ShijuTargetKeySync {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) outcome: ShijuKeySyncOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 多目标密钥同步的逐项报告。
pub(crate) struct ShijuKeySyncReport {
    pub(crate) targets: Vec<ShijuTargetKeySync>,
}

/// 将目标选择解析到允许的根目录，并拒绝缺少项目路径的项目范围。
pub(crate) fn resolve_skill_destinations(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<Vec<ResolvedSkillDestination>> {
    ensure_base_directory(home, BaseDirectoryKind::Home)?;
    if selections.iter().any(|selection| selection.scope == SkillInstallScope::Project) {
        ensure_base_directory(
            project_root.context(super::SkillHubError::localized(
                "项目级技能安装需要已打开的项目",
                vec![],
            ))?,
            BaseDirectoryKind::Project,
        )?;
    }

    let mut seen = HashSet::with_capacity(selections.len());
    let mut destinations = Vec::with_capacity(selections.len());
    for selection in selections {
        let base_dir = match selection.scope {
            SkillInstallScope::Global => home,
            SkillInstallScope::Project => project_root.context(super::SkillHubError::localized(
                "项目级技能安装需要已打开的项目",
                vec![],
            ))?,
        };
        let relative_skills_root = relative_skills_root(selection.target, selection.scope);
        let skills_root = base_dir.join(&relative_skills_root);
        let installed_dir = skills_root.join(SKILL_DIR_NAME);
        if !seen.insert(installed_dir.clone()) {
            continue;
        }
        destinations.push(ResolvedSkillDestination {
            target: selection.target,
            scope: selection.scope,
            skills_root,
            installed_dir,
            base_dir: base_dir.to_path_buf(),
            relative_skills_root,
        });
    }
    Ok(destinations)
}

fn relative_skills_root(target: SkillInstallTarget, scope: SkillInstallScope) -> PathBuf {
    let relative = match (target, scope) {
        (SkillInstallTarget::VibeWindow, _) => {
            return Path::new(HOME_CONFIG_DIR_NAME).join("skills");
        }
        (SkillInstallTarget::AgentSkills, _) => ".agents/skills",
        (SkillInstallTarget::Codex, _) => ".codex/skills",
        (SkillInstallTarget::WorkBuddy, _) => ".workbuddy/skills",
        (SkillInstallTarget::ClaudeCode, _) => ".claude/skills",
        (SkillInstallTarget::OpenCode, SkillInstallScope::Global) => ".config/opencode/skills",
        (SkillInstallTarget::OpenCode, SkillInstallScope::Project) => ".opencode/skills",
        (SkillInstallTarget::GeminiCli, _) => ".gemini/skills",
        (SkillInstallTarget::GitHubCopilot, SkillInstallScope::Global) => ".copilot/skills",
        (SkillInstallTarget::GitHubCopilot, SkillInstallScope::Project) => ".github/skills",
        (SkillInstallTarget::Cursor, _) => ".cursor/skills",
        (SkillInstallTarget::Windsurf, SkillInstallScope::Global) => ".codeium/windsurf/skills",
        (SkillInstallTarget::Windsurf, SkillInstallScope::Project) => ".windsurf/skills",
    };
    PathBuf::from(relative)
}

pub(crate) fn shiju_install_dir() -> Result<PathBuf> {
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context(super::SkillHubError::localized("无法确定用户主目录", vec![]))?;
    Ok(home_config_dir(home).join("skills").join(SKILL_DIR_NAME))
}

pub(crate) fn shiju_install_path_label() -> String {
    tilde_config_path("skills/shiju")
}

pub(crate) fn shiju_installation() -> Result<Option<ShijuInstallation>> {
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context(super::SkillHubError::localized("无法确定用户主目录", vec![]))?;
    shiju_installation_at_home(&home)
}

pub(crate) async fn uninstall_shiju_package() -> Result<bool> {
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context(super::SkillHubError::localized("无法确定用户主目录", vec![]))?;
    tokio::task::spawn_blocking(move || uninstall_shiju_at_home(&home))
        .await
        .context(super::SkillHubError::localized("识矩技能卸载任务异常终止", vec![]))?
}

/// 比较纯数字点分版本；格式无效时保守地判定为不可升级。
pub(crate) fn version_is_newer(current: &str, latest: &str) -> bool {
    let Some(current) = numeric_version(current) else {
        return false;
    };
    let Some(latest) = numeric_version(latest) else {
        return false;
    };
    let length = current.len().max(latest.len());
    (0..length)
        .map(|index| {
            (current.get(index).copied().unwrap_or(0), latest.get(index).copied().unwrap_or(0))
        })
        .find(|(current, latest)| current != latest)
        .is_some_and(|(current, latest)| latest > current)
}

fn numeric_version(version: &str) -> Option<Vec<u64>> {
    if version.is_empty() {
        return None;
    }
    version
        .split('.')
        .map(|component| {
            if component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit()) {
                None
            } else {
                component.parse().ok()
            }
        })
        .collect()
}

pub(crate) async fn install_shiju_package(
    download_url: SkillDownloadUrl,
) -> Result<ShijuInstallReport> {
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context(super::SkillHubError::localized("无法确定用户主目录", vec![]))?;
    let mut archive = NamedTempFile::new()
        .context(super::SkillHubError::localized("无法创建识矩技能临时下载文件", vec![]))?;
    download_package(&download_url, archive.as_file_mut()).await?;
    archive
        .flush()
        .context(super::SkillHubError::localized("无法写完识矩技能临时下载文件", vec![]))?;

    tokio::task::spawn_blocking(move || install_shiju_archive_at_home(archive.path(), &home))
        .await
        .context(super::SkillHubError::localized("识矩技能安装任务异常终止", vec![]))?
}

pub(super) async fn download_package(
    download_url: &SkillDownloadUrl,
    destination: &mut fs::File,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(120))
        .build()
        .context(super::SkillHubError::localized("无法创建识矩技能下载客户端", vec![]))?;
    let mut response = client.get(download_url.as_str()).send().await.map_err(|_| {
        anyhow::anyhow!(super::SkillHubError::localized("识矩技能安装包下载失败", vec![]))
    })?;
    if !response.status().is_success() {
        bail!(super::SkillHubError::localized(
            "识矩技能安装包下载失败: HTTP %s",
            vec![(response.status().as_u16()).to_string()]
        ));
    }
    if response.content_length().is_some_and(|length| length > MAX_ARCHIVE_BYTES) {
        bail!(super::SkillHubError::localized("识矩技能安装包超过 100 MiB 限制", vec![]));
    }

    let mut written = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        anyhow::anyhow!(super::SkillHubError::localized("识矩技能安装包下载中断", vec![]))
    })? {
        let chunk_len = u64::try_from(chunk.len())
            .context(super::SkillHubError::localized("识矩技能安装包大小溢出", vec![]))?;
        written = written
            .checked_add(chunk_len)
            .context(super::SkillHubError::localized("识矩技能安装包大小溢出", vec![]))?;
        if written > MAX_ARCHIVE_BYTES {
            bail!(super::SkillHubError::localized("识矩技能安装包超过 100 MiB 限制", vec![]));
        }
        destination
            .write_all(&chunk)
            .context(super::SkillHubError::localized("无法写入识矩技能临时下载文件", vec![]))?;
    }
    Ok(())
}

pub(super) fn install_shiju_archive_at_home(
    archive_path: &Path,
    home: &Path,
) -> Result<ShijuInstallReport> {
    let report = install_shiju_archive_for_targets(
        archive_path,
        home,
        None,
        &[SkillInstallSelection {
            target: SkillInstallTarget::VibeWindow,
            scope: SkillInstallScope::Global,
        }],
    )?;
    let install = report
        .installs
        .into_iter()
        .next()
        .context(super::SkillHubError::localized("识矩技能安装未生成目标报告", vec![]))?;
    Ok(ShijuInstallReport {
        installed_dir: install.destination.installed_dir,
        version: install.version,
    })
}

/// 预检全部目标后提交安装；提交失败会回滚已替换的目标。
pub(crate) fn install_shiju_archive_for_targets(
    archive_path: &Path,
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<ShijuMultiInstallReport> {
    let destinations = resolve_skill_destinations(home, project_root, selections)?;
    if destinations.is_empty() {
        bail!(super::SkillHubError::localized("未选择识矩技能安装目标", vec![]));
    }
    for destination in &destinations {
        if existing_destination_root(destination)?.is_some() {
            ensure_replaceable_target(&destination.installed_dir)?;
        }
    }

    let extracted = tempfile::tempdir()
        .context(super::SkillHubError::localized("无法创建识矩技能临时解压目录", vec![]))?;
    extract_archive(archive_path, extracted.path())?;
    let package_root = find_package_root(extracted.path())?;
    let version = read_manifest_version(&package_root.join(SKILL_MANIFEST))?;

    let mut installs = Vec::with_capacity(destinations.len());
    let mut seen_roots = HashSet::new();
    for destination in &destinations {
        let skills_root = ensure_destination_root(destination)?;
        ensure_replaceable_target(&destination.installed_dir)?;
        if !seen_roots.insert(fs::canonicalize(&skills_root)?) {
            continue;
        }
        let staging = Builder::new()
            .prefix(".shiju-installing-")
            .tempdir_in(&skills_root)
            .with_context(|| {
                super::SkillHubError::localized(
                    "无法在 %s 创建安装暂存目录",
                    vec![(skills_root.display()).to_string()],
                )
            })?;
        let staged_package = staging.path().join("package");
        copy_directory_secure(&package_root, &staged_package)?;
        installs.push(PreparedInstall {
            skills_root,
            target: destination.installed_dir.clone(),
            staging,
            backup: None,
        });
    }

    backup_existing_installs(&mut installs)?;
    commit_installs(&mut installs)?;
    Ok(ShijuMultiInstallReport {
        installs: destinations
            .into_iter()
            .map(|destination| ShijuInstallItem { destination, version: version.clone() })
            .collect(),
    })
}

pub(super) fn shiju_installation_at_home(home: &Path) -> Result<Option<ShijuInstallation>> {
    let report = shiju_installations_for_targets(
        home,
        None,
        &[SkillInstallSelection {
            target: SkillInstallTarget::VibeWindow,
            scope: SkillInstallScope::Global,
        }],
    )?;
    Ok(report.targets.into_iter().next().and_then(|target| target.installation))
}

/// 探测每个目标的现有安装状态。
pub(crate) fn shiju_installations_for_targets(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<ShijuInstallationsReport> {
    let destinations = resolve_skill_destinations(home, project_root, selections)?;
    let targets = destinations
        .into_iter()
        .map(|destination| {
            let installation = installation_at_destination(&destination)?;
            Ok(ShijuTargetInstallation { destination, installation })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ShijuInstallationsReport { targets })
}

fn installation_at_destination(
    destination: &ResolvedSkillDestination,
) -> Result<Option<ShijuInstallation>> {
    if existing_destination_root(destination)?.is_none() {
        return Ok(None);
    }
    let metadata = match fs::symlink_metadata(&destination.installed_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                super::SkillHubError::localized(
                    "无法检查 %s",
                    vec![(destination.installed_dir.display()).to_string()],
                )
            });
        }
    };
    if metadata.file_type().is_symlink() {
        bail!(super::SkillHubError::localized(
            "已安装识矩技能不能是符号链接: %s",
            vec![(destination.installed_dir.display()).to_string()]
        ));
    }
    if !metadata.is_dir() {
        bail!(super::SkillHubError::localized(
            "已安装识矩技能路径不是目录: %s",
            vec![(destination.installed_dir.display()).to_string()]
        ));
    }
    let version = read_manifest_version(&destination.installed_dir.join(SKILL_MANIFEST))?;
    Ok(Some(ShijuInstallation { installed_dir: destination.installed_dir.clone(), version }))
}

pub(super) fn uninstall_shiju_at_home(home: &Path) -> Result<bool> {
    let report = uninstall_shiju_for_targets(
        home,
        None,
        &[SkillInstallSelection {
            target: SkillInstallTarget::VibeWindow,
            scope: SkillInstallScope::Global,
        }],
    )?;
    Ok(report.targets.iter().any(|target| target.removed))
}

/// 将目标移入备份后统一提交卸载，失败时恢复已移动内容。
pub(crate) fn uninstall_shiju_for_targets(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<ShijuUninstallReport> {
    let installations = shiju_installations_for_targets(home, project_root, selections)?;
    let mut removals = Vec::new();
    let mut seen_installs = HashSet::new();
    for target in &installations.targets {
        if target.installation.is_none() {
            continue;
        }
        if !seen_installs.insert(fs::canonicalize(&target.destination.installed_dir)?) {
            continue;
        }
        let backup = Builder::new()
            .prefix(".shiju-removing-")
            .tempdir_in(&target.destination.skills_root)
            .with_context(|| {
                format!("无法在 {} 创建卸载暂存目录", target.destination.skills_root.display())
            })?;
        removals.push(PreparedRemoval { target: target.destination.installed_dir.clone(), backup });
    }
    move_installs_to_removal_backups(&mut removals)?;
    Ok(ShijuUninstallReport {
        targets: installations
            .targets
            .into_iter()
            .map(|target| ShijuTargetUninstall {
                removed: target.installation.is_some(),
                destination: target.destination,
            })
            .collect(),
    })
}

pub(super) fn sync_shiju_key_at_home(
    home: &Path,
    skey: &SecretSkey,
) -> Result<ShijuKeySyncOutcome> {
    let report = sync_shiju_key_for_targets(
        home,
        None,
        &[SkillInstallSelection {
            target: SkillInstallTarget::VibeWindow,
            scope: SkillInstallScope::Global,
        }],
        skey,
    )?;
    Ok(report
        .targets
        .first()
        .map_or(ShijuKeySyncOutcome::SkillNotInstalled, |target| target.outcome))
}

/// 更新多个目标的 `.env` 密钥；失败时回滚已提交的文件。
pub(crate) fn sync_shiju_key_for_targets(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
    skey: &SecretSkey,
) -> Result<ShijuKeySyncReport> {
    let exposed = skey.expose();
    if exposed.bytes().any(|byte| byte == b'\r' || byte == b'\n') {
        bail!(super::SkillHubError::localized("识矩登录信息包含不安全的换行符", vec![]));
    }

    let destinations = resolve_skill_destinations(home, project_root, selections)?;
    let mut outcomes = Vec::with_capacity(destinations.len());
    let mut prepared = Vec::new();
    let mut seen_installs = HashSet::new();
    for destination in destinations {
        let Some(existing) = read_existing_shiju_env(&destination)? else {
            outcomes.push(ShijuTargetKeySync {
                destination,
                outcome: ShijuKeySyncOutcome::SkillNotInstalled,
            });
            continue;
        };
        if !seen_installs.insert(fs::canonicalize(&destination.installed_dir)?) {
            outcomes
                .push(ShijuTargetKeySync { destination, outcome: ShijuKeySyncOutcome::Updated });
            continue;
        }
        let updated = update_shiju_env(&existing.contents, exposed);
        let mut temporary = Builder::new()
            .prefix(".shiju-env-")
            .tempfile_in(&destination.installed_dir)
            .with_context(|| {
                format!("无法在 {} 创建环境临时文件", destination.installed_dir.display())
            })?;
        temporary
            .write_all(updated.as_bytes())
            .context(super::SkillHubError::localized("无法写入识矩环境临时文件", vec![]))?;
        temporary
            .flush()
            .context(super::SkillHubError::localized("无法写完识矩环境临时文件", vec![]))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))
                .context(super::SkillHubError::localized("无法限制识矩环境临时文件权限", vec![]))?;
        }
        temporary
            .as_file()
            .sync_all()
            .context(super::SkillHubError::localized("无法同步识矩环境临时文件", vec![]))?;
        let backup = if existing.existed {
            Some(
                Builder::new()
                    .prefix(".shiju-env-backup-")
                    .tempdir_in(&destination.installed_dir)
                    .with_context(|| {
                        format!("无法在 {} 创建环境备份目录", destination.installed_dir.display())
                    })?,
            )
        } else {
            None
        };
        prepared.push(PreparedKeySync {
            env_path: destination.installed_dir.join(SHIJU_ENV_FILE_NAME),
            temporary: Some(temporary),
            backup,
        });
        outcomes.push(ShijuTargetKeySync { destination, outcome: ShijuKeySyncOutcome::Updated });
    }

    backup_existing_env_files(&mut prepared)?;
    commit_key_syncs(&mut prepared)?;
    Ok(ShijuKeySyncReport { targets: outcomes })
}

struct ExistingShijuEnv {
    contents: String,
    existed: bool,
}

fn read_existing_shiju_env(
    destination: &ResolvedSkillDestination,
) -> Result<Option<ExistingShijuEnv>> {
    if installation_at_destination(destination)?.is_none() {
        return Ok(None);
    }

    let env_path = destination.installed_dir.join(SHIJU_ENV_FILE_NAME);
    let existing = match fs::symlink_metadata(&env_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!(super::SkillHubError::localized(
                    "识矩环境文件不能是符号链接: %s",
                    vec![(env_path.display()).to_string()]
                ));
            }
            if !metadata.is_file() {
                bail!(super::SkillHubError::localized(
                    "识矩环境文件不是普通文件: %s",
                    vec![(env_path.display()).to_string()]
                ));
            }
            fs::read_to_string(&env_path).with_context(|| {
                super::SkillHubError::localized(
                    "无法读取识矩环境文件 %s",
                    vec![(env_path.display()).to_string()],
                )
            })?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(ExistingShijuEnv { contents: String::new(), existed: false }));
        }
        Err(error) => {
            return Err(error).with_context(|| {
                super::SkillHubError::localized(
                    "无法检查识矩环境文件 %s",
                    vec![(env_path.display()).to_string()],
                )
            });
        }
    };
    Ok(Some(ExistingShijuEnv { contents: existing, existed: true }))
}

fn update_shiju_env(existing: &str, skey: &str) -> String {
    let mut updated =
        String::with_capacity(existing.len().max(skey.len() + SHIJU_KEY_PREFIX.len()));
    let mut replaced = false;
    for line in existing.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content.starts_with(SHIJU_KEY_PREFIX) {
            if !replaced {
                updated.push_str(SHIJU_KEY_PREFIX);
                updated.push_str(skey);
                if line.ends_with("\r\n") {
                    updated.push_str("\r\n");
                } else if line.ends_with('\n') {
                    updated.push('\n');
                }
                replaced = true;
            }
        } else {
            updated.push_str(line);
        }
    }
    if !replaced {
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(SHIJU_KEY_PREFIX);
        updated.push_str(skey);
        updated.push('\n');
    }
    updated
}

// 仅允许 skills 目录链接到当前安装范围内，工具配置目录仍不得为链接。
fn resolve_skills_root_link(
    destination: &ResolvedSkillDestination,
    path: &Path,
) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path).ok();
    let base = fs::canonicalize(&destination.base_dir)?;
    if path == destination.skills_root
        && let Some(resolved) = resolved
        && resolved.starts_with(&base)
        && resolved.is_dir()
    {
        return Ok(resolved);
    }
    bail!(super::SkillHubError::localized(
        "技能安装目录不能包含符号链接: %s",
        vec![path.display().to_string()]
    ));
}

fn existing_destination_root(destination: &ResolvedSkillDestination) -> Result<Option<PathBuf>> {
    let mut current = destination.base_dir.clone();
    for component in destination.relative_skills_root.components() {
        let Component::Normal(name) = component else {
            bail!(super::SkillHubError::localized("技能安装目录包含不安全路径", vec![]));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    current = resolve_skills_root_link(destination, &current)?;
                    continue;
                }
                if !metadata.is_dir() {
                    bail!(super::SkillHubError::localized(
                        "技能安装路径不是目录: %s",
                        vec![(current.display()).to_string()]
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| {
                    super::SkillHubError::localized(
                        "无法检查技能安装目录 %s",
                        vec![(current.display()).to_string()],
                    )
                });
            }
        }
    }
    Ok(Some(current))
}

fn read_manifest_version(manifest: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(manifest) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                super::SkillHubError::localized(
                    "无法检查 %s",
                    vec![(manifest.display()).to_string()],
                )
            });
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(super::SkillHubError::localized("识矩技能清单不是安全的普通文件", vec![]));
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        bail!(super::SkillHubError::localized("识矩技能清单超过 64 KiB 限制", vec![]));
    }
    let contents = fs::read_to_string(manifest).with_context(|| {
        super::SkillHubError::localized("无法读取 %s", vec![(manifest.display()).to_string()])
    })?;
    let mut lines = contents.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Ok(None);
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("version:") {
            let value = value.trim().trim_matches(['\'', '"']);
            return Ok(numeric_version(value).map(|_| value.to_string()));
        }
    }
    Ok(None)
}

struct PreparedInstall {
    skills_root: PathBuf,
    target: PathBuf,
    staging: TempDir,
    backup: Option<TempDir>,
}

struct PreparedRemoval {
    target: PathBuf,
    backup: TempDir,
}

struct PreparedKeySync {
    env_path: PathBuf,
    temporary: Option<NamedTempFile>,
    backup: Option<TempDir>,
}

#[derive(Clone, Copy)]
enum BaseDirectoryKind {
    Home,
    Project,
}

fn ensure_base_directory(path: &Path, kind: BaseDirectoryKind) -> Result<()> {
    let read_source = match kind {
        BaseDirectoryKind::Home => "无法读取用户主目录 %s",
        BaseDirectoryKind::Project => "无法读取项目根目录 %s",
    };
    let metadata = fs::symlink_metadata(path).with_context(|| {
        super::SkillHubError::localized(read_source, vec![path.display().to_string()])
    })?;
    if metadata.file_type().is_symlink() {
        let source = match kind {
            BaseDirectoryKind::Home => "用户主目录不能是符号链接",
            BaseDirectoryKind::Project => "项目根目录不能是符号链接",
        };
        bail!(super::SkillHubError::localized(source, vec![]));
    }
    if !metadata.is_dir() {
        let source = match kind {
            BaseDirectoryKind::Home => "用户主目录不是目录",
            BaseDirectoryKind::Project => "项目根目录不是目录",
        };
        bail!(super::SkillHubError::localized(source, vec![]));
    }
    Ok(())
}

fn ensure_destination_root(destination: &ResolvedSkillDestination) -> Result<PathBuf> {
    let mut current = destination.base_dir.clone();
    for component in destination.relative_skills_root.components() {
        let Component::Normal(name) = component else {
            bail!(super::SkillHubError::localized("技能安装目录包含不安全路径", vec![]));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    current = resolve_skills_root_link(destination, &current)?;
                    continue;
                }
                if !metadata.is_dir() {
                    bail!(super::SkillHubError::localized(
                        "技能安装路径不是目录: %s",
                        vec![(current.display()).to_string()]
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).with_context(|| {
                    super::SkillHubError::localized(
                        "无法创建技能安装目录 %s",
                        vec![(current.display()).to_string()],
                    )
                })?;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    super::SkillHubError::localized(
                        "无法检查技能安装目录 %s",
                        vec![(current.display()).to_string()],
                    )
                });
            }
        }
    }
    Ok(current)
}

fn ensure_replaceable_target(target: &Path) -> Result<()> {
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!(super::SkillHubError::localized(
                    "已安装识矩技能不能是符号链接: %s",
                    vec![(target.display()).to_string()]
                ));
            }
            if !metadata.is_dir() {
                bail!(super::SkillHubError::localized(
                    "已安装识矩技能路径不是目录: %s",
                    vec![(target.display()).to_string()]
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                super::SkillHubError::localized("无法检查 %s", vec![(target.display()).to_string()])
            });
        }
    }
    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    // 解压同时限制条目数、路径与累计体积，避免路径穿越和压缩炸弹。
    let metadata = fs::metadata(archive_path)
        .context(super::SkillHubError::localized("无法读取识矩技能安装包", vec![]))?;
    if metadata.len() > MAX_ARCHIVE_BYTES {
        bail!(super::SkillHubError::localized("识矩技能安装包超过 100 MiB 限制", vec![]));
    }
    let file = fs::File::open(archive_path)
        .context(super::SkillHubError::localized("无法打开识矩技能安装包", vec![]))?;
    let mut archive = ZipArchive::new(file)
        .context(super::SkillHubError::localized("识矩技能安装包不是有效 ZIP", vec![]))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        bail!(super::SkillHubError::localized("识矩技能安装包文件数量超过限制", vec![]));
    }

    let mut seen = HashSet::with_capacity(archive.len());
    let mut declared_size = 0u64;
    let mut extracted_size = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).with_context(|| {
            super::SkillHubError::localized(
                "无法读取识矩技能安装包第 %s 项",
                vec![(index + 1).to_string()],
            )
        })?;
        let entry_name = entry.name().to_string();
        if entry_name.contains('\\') {
            bail!(super::SkillHubError::localized("识矩技能安装包包含不安全路径", vec![]));
        }
        let relative = entry
            .enclosed_name()
            .context(super::SkillHubError::localized("识矩技能安装包包含不安全路径", vec![]))?;
        if relative.as_os_str().is_empty() || !seen.insert(relative.clone()) {
            bail!(super::SkillHubError::localized("识矩技能安装包包含无效或重复路径", vec![]));
        }
        let unix_mode = entry.unix_mode();
        if unix_mode.is_some_and(|mode| mode & 0o170000 == 0o120000) {
            bail!(super::SkillHubError::localized("识矩技能安装包不能包含符号链接", vec![]));
        }
        if unix_mode.is_some_and(|mode| {
            let kind = mode & 0o170000;
            kind != 0 && kind != 0o040000 && kind != 0o100000
        }) {
            bail!(super::SkillHubError::localized("识矩技能安装包包含不支持的文件类型", vec![]));
        }
        declared_size = declared_size
            .checked_add(entry.size())
            .context(super::SkillHubError::localized("识矩技能安装包解压大小溢出", vec![]))?;
        if declared_size > MAX_EXTRACTED_BYTES {
            bail!(super::SkillHubError::localized("识矩技能安装包解压后超过 512 MiB 限制", vec![]));
        }

        let output = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&output).with_context(|| {
                super::SkillHubError::localized(
                    "无法创建解压目录 %s",
                    vec![(output.display()).to_string()],
                )
            })?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).with_context(|| {
                super::SkillHubError::localized(
                    "无法创建解压目录 %s",
                    vec![(parent.display()).to_string()],
                )
            })?;
        }
        let mut file = fs::File::create(&output).with_context(|| {
            super::SkillHubError::localized("无法创建 %s", vec![(output.display()).to_string()])
        })?;
        let remaining = MAX_EXTRACTED_BYTES
            .checked_sub(extracted_size)
            .context(super::SkillHubError::localized("识矩技能安装包解压大小溢出", vec![]))?;
        let copied = std::io::copy(&mut entry.by_ref().take(remaining + 1), &mut file)
            .with_context(|| {
                super::SkillHubError::localized(
                    "无法解压 %s",
                    vec![(relative.display()).to_string()],
                )
            })?;
        if copied > remaining {
            bail!(super::SkillHubError::localized("识矩技能安装包解压后超过 512 MiB 限制", vec![]));
        }
        extracted_size = extracted_size
            .checked_add(copied)
            .context(super::SkillHubError::localized("识矩技能安装包解压大小溢出", vec![]))?;
        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output, fs::Permissions::from_mode(mode & 0o777)).with_context(
                || {
                    super::SkillHubError::localized(
                        "无法设置 %s 的权限",
                        vec![(output.display()).to_string()],
                    )
                },
            )?;
        }
    }
    Ok(())
}

fn find_package_root(extracted: &Path) -> Result<PathBuf> {
    if extracted.join(SKILL_MANIFEST).is_file() {
        return Ok(extracted.to_path_buf());
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(extracted)
        .context(super::SkillHubError::localized("无法检查识矩技能包根目录", vec![]))?
    {
        let entry =
            entry.context(super::SkillHubError::localized("无法读取识矩技能包根目录", vec![]))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).with_context(|| {
            super::SkillHubError::localized("无法检查 %s", vec![(path.display()).to_string()])
        })?;
        if metadata.is_dir() && path.join(SKILL_MANIFEST).is_file() {
            candidates.push(path);
        }
    }
    match candidates.as_slice() {
        [root] => Ok(root.clone()),
        [] => bail!(super::SkillHubError::localized("识矩技能安装包缺少 SKILL.md", vec![])),
        _ => bail!(super::SkillHubError::localized("识矩技能安装包包含多个技能根目录", vec![])),
    }
}

fn copy_directory_secure(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).with_context(|| {
        super::SkillHubError::localized("无法读取 %s", vec![(source.display()).to_string()])
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!(super::SkillHubError::localized("识矩技能源目录不安全", vec![]));
    }
    fs::create_dir(destination).with_context(|| {
        super::SkillHubError::localized(
            "无法创建暂存目录 %s",
            vec![(destination.display()).to_string()],
        )
    })?;
    for entry in fs::read_dir(source).with_context(|| {
        super::SkillHubError::localized("无法读取 %s", vec![(source.display()).to_string()])
    })? {
        let entry = entry.with_context(|| {
            super::SkillHubError::localized("无法读取 %s", vec![(source.display()).to_string()])
        })?;
        let source_path = entry.path();
        let target_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).with_context(|| {
            super::SkillHubError::localized(
                "无法读取 %s",
                vec![(source_path.display()).to_string()],
            )
        })?;
        if metadata.file_type().is_symlink() {
            bail!(super::SkillHubError::localized("识矩技能包不能包含符号链接", vec![]));
        }
        if metadata.is_dir() {
            copy_directory_secure(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!("无法复制 {} 到 {}", source_path.display(), target_path.display())
            })?;
            fs::set_permissions(&target_path, metadata.permissions()).with_context(|| {
                super::SkillHubError::localized(
                    "无法设置 %s 的权限",
                    vec![(target_path.display()).to_string()],
                )
            })?;
        } else {
            bail!(super::SkillHubError::localized("识矩技能包包含不支持的文件类型", vec![]));
        }
    }
    Ok(())
}

fn backup_existing_installs(installs: &mut [PreparedInstall]) -> Result<()> {
    for index in 0..installs.len() {
        if !installs[index].target.exists() {
            continue;
        }
        let backup = Builder::new()
            .prefix(".shiju-backup-")
            .tempdir_in(&installs[index].skills_root)
            .with_context(|| {
                format!("无法在 {} 创建备份目录", installs[index].skills_root.display())
            });
        let backup = match backup {
            Ok(backup) => backup,
            Err(error) => {
                return match restore_backups(installs) {
                    Ok(()) => Err(error),
                    Err(rollback_error) => {
                        Err(error.context(format!("备份失败后的回滚也失败: {rollback_error}")))
                    }
                };
            }
        };
        let previous = backup.path().join("previous");
        if let Err(error) = fs::rename(&installs[index].target, &previous).with_context(|| {
            super::SkillHubError::localized(
                "无法备份已有识矩技能 %s",
                vec![(installs[index].target.display()).to_string()],
            )
        }) {
            return match restore_backups(installs) {
                Ok(()) => Err(error),
                Err(rollback_error) => {
                    Err(error.context(format!("备份失败后的回滚也失败: {rollback_error}")))
                }
            };
        }
        installs[index].backup = Some(backup);
    }
    Ok(())
}

fn commit_installs(installs: &mut [PreparedInstall]) -> Result<()> {
    let mut committed = 0usize;
    for index in 0..installs.len() {
        let staged_package = installs[index].staging.path().join("package");
        if let Err(error) =
            fs::rename(&staged_package, &installs[index].target).with_context(|| {
                super::SkillHubError::localized(
                    "无法安装识矩技能到 %s",
                    vec![(installs[index].target.display()).to_string()],
                )
            })
        {
            let rollback_error = rollback_commits(installs, committed).err();
            return match rollback_error {
                Some(rollback_error) => {
                    Err(error.context(format!("安装失败后的回滚也失败: {rollback_error}")))
                }
                None => Err(error),
            };
        }
        committed += 1;
    }
    Ok(())
}

fn rollback_commits(installs: &mut [PreparedInstall], committed: usize) -> Result<()> {
    for install in installs.iter().take(committed) {
        if install.target.exists() {
            fs::remove_dir_all(&install.target).with_context(|| {
                super::SkillHubError::localized(
                    "无法清理 %s",
                    vec![(install.target.display()).to_string()],
                )
            })?;
        }
    }
    restore_backups(installs)
}

fn restore_backups(installs: &mut [PreparedInstall]) -> Result<()> {
    for install in installs {
        let Some(backup) = install.backup.as_ref() else {
            continue;
        };
        let previous = backup.path().join("previous");
        if previous.exists() {
            fs::rename(&previous, &install.target).with_context(|| {
                super::SkillHubError::localized(
                    "无法恢复 %s",
                    vec![(install.target.display()).to_string()],
                )
            })?;
        }
    }
    Ok(())
}

fn move_installs_to_removal_backups(removals: &mut [PreparedRemoval]) -> Result<()> {
    for index in 0..removals.len() {
        let previous = removals[index].backup.path().join("previous");
        if let Err(error) = fs::rename(&removals[index].target, &previous).with_context(|| {
            super::SkillHubError::localized(
                "无法暂存待卸载识矩技能 %s",
                vec![(removals[index].target.display()).to_string()],
            )
        }) {
            return match restore_removal_backups(removals) {
                Ok(()) => Err(error),
                Err(rollback_error) => {
                    Err(error.context(format!("卸载失败后的回滚也失败: {rollback_error}")))
                }
            };
        }
    }
    Ok(())
}

fn restore_removal_backups(removals: &mut [PreparedRemoval]) -> Result<()> {
    for removal in removals.iter_mut().rev() {
        let previous = removal.backup.path().join("previous");
        if previous.exists() {
            fs::rename(&previous, &removal.target).with_context(|| {
                super::SkillHubError::localized(
                    "无法恢复 %s",
                    vec![(removal.target.display()).to_string()],
                )
            })?;
        }
    }
    Ok(())
}

fn backup_existing_env_files(syncs: &mut [PreparedKeySync]) -> Result<()> {
    for index in 0..syncs.len() {
        let Some(backup) = syncs[index].backup.as_ref() else {
            continue;
        };
        let previous = backup.path().join("previous");
        if let Err(error) = fs::rename(&syncs[index].env_path, &previous).with_context(|| {
            super::SkillHubError::localized(
                "无法备份识矩环境文件 %s",
                vec![(syncs[index].env_path.display()).to_string()],
            )
        }) {
            return match restore_env_backups(syncs) {
                Ok(()) => Err(error),
                Err(rollback_error) => {
                    Err(error.context(format!("环境备份失败后的回滚也失败: {rollback_error}")))
                }
            };
        }
    }
    Ok(())
}

fn commit_key_syncs(syncs: &mut [PreparedKeySync]) -> Result<()> {
    for index in 0..syncs.len() {
        let temporary = syncs[index]
            .temporary
            .take()
            .context(super::SkillHubError::localized("识矩环境临时文件缺失", vec![]))?;
        if let Err(error) = temporary
            .persist(&syncs[index].env_path)
            .map_err(|error| error.error)
            .with_context(|| {
                super::SkillHubError::localized(
                    "无法替换识矩环境文件 %s",
                    vec![(syncs[index].env_path.display()).to_string()],
                )
            })
        {
            let rollback_error = rollback_key_syncs(syncs, index).err();
            return match rollback_error {
                Some(rollback_error) => {
                    Err(error.context(format!("环境同步失败后的回滚也失败: {rollback_error}")))
                }
                None => Err(error),
            };
        }
    }
    Ok(())
}

fn rollback_key_syncs(syncs: &mut [PreparedKeySync], committed: usize) -> Result<()> {
    for sync in syncs.iter().take(committed) {
        match fs::remove_file(&sync.env_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    super::SkillHubError::localized(
                        "无法清理 %s",
                        vec![(sync.env_path.display()).to_string()],
                    )
                });
            }
        }
    }
    restore_env_backups(syncs)
}

fn restore_env_backups(syncs: &mut [PreparedKeySync]) -> Result<()> {
    for sync in syncs {
        let Some(backup) = sync.backup.as_ref() else {
            continue;
        };
        let previous = backup.path().join("previous");
        if previous.exists() {
            fs::rename(&previous, &sync.env_path).with_context(|| {
                super::SkillHubError::localized(
                    "无法恢复 %s",
                    vec![(sync.env_path.display()).to_string()],
                )
            })?;
        }
    }
    Ok(())
}
