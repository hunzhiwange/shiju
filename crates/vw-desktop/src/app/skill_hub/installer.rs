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
pub(crate) enum SkillInstallScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
pub(crate) struct SkillInstallSelection {
    pub(crate) target: SkillInstallTarget,
    pub(crate) scope: SkillInstallScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedSkillDestination {
    pub(crate) target: SkillInstallTarget,
    pub(crate) scope: SkillInstallScope,
    pub(crate) skills_root: PathBuf,
    pub(crate) installed_dir: PathBuf,
    base_dir: PathBuf,
    relative_skills_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuInstallReport {
    pub(crate) installed_dir: PathBuf,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuInstallItem {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuMultiInstallReport {
    pub(crate) installs: Vec<ShijuInstallItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuInstallation {
    pub(crate) installed_dir: PathBuf,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuTargetInstallation {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) installation: Option<ShijuInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuInstallationsReport {
    pub(crate) targets: Vec<ShijuTargetInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuTargetUninstall {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuUninstallReport {
    pub(crate) targets: Vec<ShijuTargetUninstall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShijuKeySyncOutcome {
    Updated,
    SkillNotInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuTargetKeySync {
    pub(crate) destination: ResolvedSkillDestination,
    pub(crate) outcome: ShijuKeySyncOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuKeySyncReport {
    pub(crate) targets: Vec<ShijuTargetKeySync>,
}

pub(crate) fn resolve_skill_destinations(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<Vec<ResolvedSkillDestination>> {
    ensure_base_directory(home, "用户主目录")?;
    if selections.iter().any(|selection| selection.scope == SkillInstallScope::Project) {
        ensure_base_directory(
            project_root.context("项目级技能安装需要已打开的项目")?,
            "项目根目录",
        )?;
    }

    let mut seen = HashSet::with_capacity(selections.len());
    let mut destinations = Vec::with_capacity(selections.len());
    for selection in selections {
        let base_dir = match selection.scope {
            SkillInstallScope::Global => home,
            SkillInstallScope::Project => project_root.context("项目级技能安装需要已打开的项目")?,
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
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    Ok(home_config_dir(home).join("skills").join(SKILL_DIR_NAME))
}

pub(crate) fn shiju_install_path_label() -> String {
    tilde_config_path("skills/shiju")
}

pub(crate) fn shiju_installation() -> Result<Option<ShijuInstallation>> {
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    shiju_installation_at_home(&home)
}

pub(crate) async fn uninstall_shiju_package() -> Result<bool> {
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    tokio::task::spawn_blocking(move || uninstall_shiju_at_home(&home))
        .await
        .context("识矩技能卸载任务异常终止")?
}

pub(crate) async fn sync_shiju_key(skey: SecretSkey) -> Result<ShijuKeySyncOutcome> {
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    tokio::task::spawn_blocking(move || sync_shiju_key_at_home(&home, &skey))
        .await
        .context("识矩登录信息同步任务异常终止")?
}

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
    let home =
        BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()).context("无法确定用户主目录")?;
    let mut archive = NamedTempFile::new().context("无法创建识矩技能临时下载文件")?;
    download_package(&download_url, archive.as_file_mut()).await?;
    archive.flush().context("无法写完识矩技能临时下载文件")?;

    tokio::task::spawn_blocking(move || install_shiju_archive_at_home(archive.path(), &home))
        .await
        .context("识矩技能安装任务异常终止")?
}

async fn download_package(
    download_url: &SkillDownloadUrl,
    destination: &mut fs::File,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(120))
        .build()
        .context("无法创建识矩技能下载客户端")?;
    let mut response = client
        .get(download_url.as_str())
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("识矩技能安装包下载失败"))?;
    if !response.status().is_success() {
        bail!("识矩技能安装包下载失败: HTTP {}", response.status().as_u16());
    }
    if response.content_length().is_some_and(|length| length > MAX_ARCHIVE_BYTES) {
        bail!("识矩技能安装包超过 100 MiB 限制");
    }

    let mut written = 0u64;
    while let Some(chunk) =
        response.chunk().await.map_err(|_| anyhow::anyhow!("识矩技能安装包下载中断"))?
    {
        let chunk_len = u64::try_from(chunk.len()).context("识矩技能安装包大小溢出")?;
        written = written.checked_add(chunk_len).context("识矩技能安装包大小溢出")?;
        if written > MAX_ARCHIVE_BYTES {
            bail!("识矩技能安装包超过 100 MiB 限制");
        }
        destination.write_all(&chunk).context("无法写入识矩技能临时下载文件")?;
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
    let install = report.installs.into_iter().next().context("识矩技能安装未生成目标报告")?;
    Ok(ShijuInstallReport {
        installed_dir: install.destination.installed_dir,
        version: install.version,
    })
}

pub(crate) fn install_shiju_archive_for_targets(
    archive_path: &Path,
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<ShijuMultiInstallReport> {
    let destinations = resolve_skill_destinations(home, project_root, selections)?;
    if destinations.is_empty() {
        bail!("未选择识矩技能安装目标");
    }
    for destination in &destinations {
        if existing_destination_root(destination)?.is_some() {
            ensure_replaceable_target(&destination.installed_dir)?;
        }
    }

    let extracted = tempfile::tempdir().context("无法创建识矩技能临时解压目录")?;
    extract_archive(archive_path, extracted.path())?;
    let package_root = find_package_root(extracted.path())?;
    let version = read_manifest_version(&package_root.join(SKILL_MANIFEST))?;

    let mut installs = Vec::with_capacity(destinations.len());
    for destination in &destinations {
        let skills_root = ensure_destination_root(destination)?;
        ensure_replaceable_target(&destination.installed_dir)?;
        let staging = Builder::new()
            .prefix(".shiju-installing-")
            .tempdir_in(&skills_root)
            .with_context(|| format!("无法在 {} 创建安装暂存目录", skills_root.display()))?;
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
            return Err(error)
                .with_context(|| format!("无法检查 {}", destination.installed_dir.display()));
        }
    };
    if metadata.file_type().is_symlink() {
        bail!("已安装识矩技能不能是符号链接: {}", destination.installed_dir.display());
    }
    if !metadata.is_dir() {
        bail!("已安装识矩技能路径不是目录: {}", destination.installed_dir.display());
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

pub(crate) fn uninstall_shiju_for_targets(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
) -> Result<ShijuUninstallReport> {
    let installations = shiju_installations_for_targets(home, project_root, selections)?;
    let mut removals = Vec::new();
    for target in &installations.targets {
        if target.installation.is_none() {
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
            target: SkillInstallTarget::WorkBuddy,
            scope: SkillInstallScope::Global,
        }],
        skey,
    )?;
    Ok(report
        .targets
        .first()
        .map_or(ShijuKeySyncOutcome::SkillNotInstalled, |target| target.outcome))
}

pub(crate) fn sync_shiju_key_for_targets(
    home: &Path,
    project_root: Option<&Path>,
    selections: &[SkillInstallSelection],
    skey: &SecretSkey,
) -> Result<ShijuKeySyncReport> {
    let exposed = skey.expose();
    if exposed.bytes().any(|byte| byte == b'\r' || byte == b'\n') {
        bail!("识矩登录信息包含不安全的换行符");
    }

    let destinations = resolve_skill_destinations(home, project_root, selections)?;
    let mut outcomes = Vec::with_capacity(destinations.len());
    let mut prepared = Vec::new();
    for destination in destinations {
        let Some(existing) = read_existing_shiju_env(&destination)? else {
            outcomes.push(ShijuTargetKeySync {
                destination,
                outcome: ShijuKeySyncOutcome::SkillNotInstalled,
            });
            continue;
        };
        let updated = update_shiju_env(&existing.contents, exposed);
        let mut temporary = Builder::new()
            .prefix(".shiju-env-")
            .tempfile_in(&destination.installed_dir)
            .with_context(|| {
                format!("无法在 {} 创建环境临时文件", destination.installed_dir.display())
            })?;
        temporary.write_all(updated.as_bytes()).context("无法写入识矩环境临时文件")?;
        temporary.flush().context("无法写完识矩环境临时文件")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))
                .context("无法限制识矩环境临时文件权限")?;
        }
        temporary.as_file().sync_all().context("无法同步识矩环境临时文件")?;
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
                bail!("识矩环境文件不能是符号链接: {}", env_path.display());
            }
            if !metadata.is_file() {
                bail!("识矩环境文件不是普通文件: {}", env_path.display());
            }
            fs::read_to_string(&env_path)
                .with_context(|| format!("无法读取识矩环境文件 {}", env_path.display()))?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(ExistingShijuEnv { contents: String::new(), existed: false }));
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查识矩环境文件 {}", env_path.display()));
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

fn existing_destination_root(destination: &ResolvedSkillDestination) -> Result<Option<PathBuf>> {
    let mut current = destination.base_dir.clone();
    for component in destination.relative_skills_root.components() {
        let Component::Normal(name) = component else {
            bail!("技能安装目录包含不安全路径");
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    bail!("技能安装目录不能包含符号链接: {}", current.display());
                }
                if !metadata.is_dir() {
                    bail!("技能安装路径不是目录: {}", current.display());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法检查技能安装目录 {}", current.display()));
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
            return Err(error).with_context(|| format!("无法检查 {}", manifest.display()));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("识矩技能清单不是安全的普通文件");
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        bail!("识矩技能清单超过 64 KiB 限制");
    }
    let contents =
        fs::read_to_string(manifest).with_context(|| format!("无法读取 {}", manifest.display()))?;
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

fn ensure_base_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("无法读取{label} {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label}不能是符号链接");
    }
    if !metadata.is_dir() {
        bail!("{label}不是目录");
    }
    Ok(())
}

fn ensure_destination_root(destination: &ResolvedSkillDestination) -> Result<PathBuf> {
    let mut current = destination.base_dir.clone();
    for component in destination.relative_skills_root.components() {
        let Component::Normal(name) = component else {
            bail!("技能安装目录包含不安全路径");
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    bail!("技能安装目录不能包含符号链接: {}", current.display());
                }
                if !metadata.is_dir() {
                    bail!("技能安装路径不是目录: {}", current.display());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .with_context(|| format!("无法创建技能安装目录 {}", current.display()))?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法检查技能安装目录 {}", current.display()));
            }
        }
    }
    Ok(current)
}

fn ensure_replaceable_target(target: &Path) -> Result<()> {
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("已安装识矩技能不能是符号链接: {}", target.display());
            }
            if !metadata.is_dir() {
                bail!("已安装识矩技能路径不是目录: {}", target.display());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查 {}", target.display()));
        }
    }
    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::metadata(archive_path).context("无法读取识矩技能安装包")?;
    if metadata.len() > MAX_ARCHIVE_BYTES {
        bail!("识矩技能安装包超过 100 MiB 限制");
    }
    let file = fs::File::open(archive_path).context("无法打开识矩技能安装包")?;
    let mut archive = ZipArchive::new(file).context("识矩技能安装包不是有效 ZIP")?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        bail!("识矩技能安装包文件数量超过限制");
    }

    let mut seen = HashSet::with_capacity(archive.len());
    let mut declared_size = 0u64;
    let mut extracted_size = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("无法读取识矩技能安装包第 {} 项", index + 1))?;
        let entry_name = entry.name().to_string();
        if entry_name.contains('\\') {
            bail!("识矩技能安装包包含不安全路径");
        }
        let relative = entry.enclosed_name().context("识矩技能安装包包含不安全路径")?;
        if relative.as_os_str().is_empty() || !seen.insert(relative.clone()) {
            bail!("识矩技能安装包包含无效或重复路径");
        }
        let unix_mode = entry.unix_mode();
        if unix_mode.is_some_and(|mode| mode & 0o170000 == 0o120000) {
            bail!("识矩技能安装包不能包含符号链接");
        }
        if unix_mode.is_some_and(|mode| {
            let kind = mode & 0o170000;
            kind != 0 && kind != 0o040000 && kind != 0o100000
        }) {
            bail!("识矩技能安装包包含不支持的文件类型");
        }
        declared_size =
            declared_size.checked_add(entry.size()).context("识矩技能安装包解压大小溢出")?;
        if declared_size > MAX_EXTRACTED_BYTES {
            bail!("识矩技能安装包解压后超过 512 MiB 限制");
        }

        let output = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)
                .with_context(|| format!("无法创建解压目录 {}", output.display()))?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("无法创建解压目录 {}", parent.display()))?;
        }
        let mut file =
            fs::File::create(&output).with_context(|| format!("无法创建 {}", output.display()))?;
        let remaining = MAX_EXTRACTED_BYTES
            .checked_sub(extracted_size)
            .context("识矩技能安装包解压大小溢出")?;
        let copied = std::io::copy(&mut entry.by_ref().take(remaining + 1), &mut file)
            .with_context(|| format!("无法解压 {}", relative.display()))?;
        if copied > remaining {
            bail!("识矩技能安装包解压后超过 512 MiB 限制");
        }
        extracted_size =
            extracted_size.checked_add(copied).context("识矩技能安装包解压大小溢出")?;
        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output, fs::Permissions::from_mode(mode & 0o777))
                .with_context(|| format!("无法设置 {} 的权限", output.display()))?;
        }
    }
    Ok(())
}

fn find_package_root(extracted: &Path) -> Result<PathBuf> {
    if extracted.join(SKILL_MANIFEST).is_file() {
        return Ok(extracted.to_path_buf());
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(extracted).context("无法检查识矩技能包根目录")? {
        let entry = entry.context("无法读取识矩技能包根目录")?;
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).with_context(|| format!("无法检查 {}", path.display()))?;
        if metadata.is_dir() && path.join(SKILL_MANIFEST).is_file() {
            candidates.push(path);
        }
    }
    match candidates.as_slice() {
        [root] => Ok(root.clone()),
        [] => bail!("识矩技能安装包缺少 SKILL.md"),
        _ => bail!("识矩技能安装包包含多个技能根目录"),
    }
}

fn copy_directory_secure(source: &Path, destination: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(source).with_context(|| format!("无法读取 {}", source.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("识矩技能源目录不安全");
    }
    fs::create_dir(destination)
        .with_context(|| format!("无法创建暂存目录 {}", destination.display()))?;
    for entry in fs::read_dir(source).with_context(|| format!("无法读取 {}", source.display()))?
    {
        let entry = entry.with_context(|| format!("无法读取 {}", source.display()))?;
        let source_path = entry.path();
        let target_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .with_context(|| format!("无法读取 {}", source_path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!("识矩技能包不能包含符号链接");
        }
        if metadata.is_dir() {
            copy_directory_secure(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!("无法复制 {} 到 {}", source_path.display(), target_path.display())
            })?;
            fs::set_permissions(&target_path, metadata.permissions())
                .with_context(|| format!("无法设置 {} 的权限", target_path.display()))?;
        } else {
            bail!("识矩技能包包含不支持的文件类型");
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
        if let Err(error) = fs::rename(&installs[index].target, &previous)
            .with_context(|| format!("无法备份已有识矩技能 {}", installs[index].target.display()))
        {
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
        if let Err(error) = fs::rename(&staged_package, &installs[index].target)
            .with_context(|| format!("无法安装识矩技能到 {}", installs[index].target.display()))
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
            fs::remove_dir_all(&install.target)
                .with_context(|| format!("无法清理 {}", install.target.display()))?;
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
            fs::rename(&previous, &install.target)
                .with_context(|| format!("无法恢复 {}", install.target.display()))?;
        }
    }
    Ok(())
}

fn move_installs_to_removal_backups(removals: &mut [PreparedRemoval]) -> Result<()> {
    for index in 0..removals.len() {
        let previous = removals[index].backup.path().join("previous");
        if let Err(error) = fs::rename(&removals[index].target, &previous)
            .with_context(|| format!("无法暂存待卸载识矩技能 {}", removals[index].target.display()))
        {
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
            fs::rename(&previous, &removal.target)
                .with_context(|| format!("无法恢复 {}", removal.target.display()))?;
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
        if let Err(error) = fs::rename(&syncs[index].env_path, &previous)
            .with_context(|| format!("无法备份识矩环境文件 {}", syncs[index].env_path.display()))
        {
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
        let temporary = syncs[index].temporary.take().context("识矩环境临时文件缺失")?;
        if let Err(error) = temporary
            .persist(&syncs[index].env_path)
            .map_err(|error| error.error)
            .with_context(|| format!("无法替换识矩环境文件 {}", syncs[index].env_path.display()))
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
                return Err(error).with_context(|| format!("无法清理 {}", sync.env_path.display()));
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
            fs::rename(&previous, &sync.env_path)
                .with_context(|| format!("无法恢复 {}", sync.env_path.display()))?;
        }
    }
    Ok(())
}
