//! 技能安装选择、下载写盘与列表分页的领域操作。
use crate::skill_hub::*;
use directories::BaseDirs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
pub const SHIJU_INSTALL_TARGETS: [SkillInstallTarget; 10] = [
    SkillInstallTarget::VibeWindow,
    SkillInstallTarget::AgentSkills,
    SkillInstallTarget::Codex,
    SkillInstallTarget::WorkBuddy,
    SkillInstallTarget::ClaudeCode,
    SkillInstallTarget::OpenCode,
    SkillInstallTarget::GeminiCli,
    SkillInstallTarget::GitHubCopilot,
    SkillInstallTarget::Cursor,
    SkillInstallTarget::Windsurf,
];

pub const MAX_SHIJU_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;

pub fn all_install_selections(include_project: bool) -> Vec<SkillInstallSelection> {
    let mut selections = Vec::with_capacity(if include_project {
        SHIJU_INSTALL_TARGETS.len() * 2
    } else {
        SHIJU_INSTALL_TARGETS.len()
    });
    for target in SHIJU_INSTALL_TARGETS {
        selections.push(SkillInstallSelection { target, scope: SkillInstallScope::Global });
        if include_project {
            selections.push(SkillInstallSelection { target, scope: SkillInstallScope::Project });
        }
    }
    selections
}

pub fn home_directory() -> Result<PathBuf, SkillHubError> {
    BaseDirs::new()
        .map(|directories| directories.home_dir().to_path_buf())
        .ok_or_else(|| SkillHubError::localized("无法确定用户主目录", vec![]))
}

pub fn agent_install_text(
    i18n: desktop_i18n::I18n,
    state: &ShijuInstallGuideState,
) -> String {
    match &state.agent_package {
        LoadState::Loaded(release) => agent_install_text_for_release(i18n, release),
        LoadState::Loading { .. } => i18n.__("正在生成当前设备的安装地址…").to_string(),
        LoadState::NeedsLogin => i18n.__("登录识矩账户后即可生成安装文字。").to_string(),
        LoadState::Error(_) => i18n.__("安装地址生成失败，请稍后重试。").to_string(),
        LoadState::Idle => i18n.__("正在准备安装文字…").to_string(),
    }
}

pub fn agent_install_text_for_release(
    i18n: desktop_i18n::I18n,
    release: &ShijuPackageRelease,
) -> String {
    i18n.format("请帮我安装识矩技能。\n\n下载地址：%s\n\n请下载该 zip，将解压后的技能包放入当前 Agent 的技能目录。", &[release.download_url.as_str().into()]).expect("valid installation text")
}

pub fn dispatch_shiju_download_with(
    release: &ShijuPackageRelease,
    opener: impl FnOnce(&str) -> Result<(), SkillHubError>,
) -> Result<(), SkillHubError> {
    opener(release.download_url.as_str())
        .map_err(|_| SkillHubError::localized("无法打开识矩技能下载地址", vec![]))
}

pub fn open_shiju_install_dir_with(
    directory: &Path,
    opener: impl FnOnce(&Path) -> Result<(), SkillHubError>,
) -> Result<(), SkillHubError> {
    if !directory.is_dir() {
        return Err(SkillHubError::localized("识矩技能安装目录不存在", vec![]));
    }
    opener(directory).map_err(|_| SkillHubError::localized("无法打开识矩技能目录", vec![]))
}

pub fn write_shiju_download_chunks(
    mut archive: NamedTempFile,
    mut chunks: tokio::sync::mpsc::Receiver<ShijuDownloadChunk>,
) -> Result<NamedTempFile, SkillHubError> {
    loop {
        match chunks.blocking_recv() {
            Some(ShijuDownloadChunk::Data(chunk)) => {
                archive.write_all(&chunk).map_err(|_| {
                    SkillHubError::localized("无法写入识矩技能临时下载文件", vec![])
                })?;
            }
            Some(ShijuDownloadChunk::Complete) => {
                archive.flush().map_err(|_| {
                    SkillHubError::localized("无法写完识矩技能临时下载文件", vec![])
                })?;
                return Ok(archive);
            }
            Some(ShijuDownloadChunk::Failed(error)) => return Err(error),
            None => return Err(SkillHubError::localized("识矩技能安装包下载未完成", vec![])),
        }
    }
}

pub fn install_report_matches_selections(
    report: &ShijuMultiInstallReport,
    selections: &[SkillInstallSelection],
) -> bool {
    report.installs.len() == selections.len()
        && selections.iter().all(|selection| {
            report.installs.iter().any(|install| {
                install.destination.scope == selection.scope
                    && install.destination.target == selection.target
            })
        })
}

pub fn page_has_next<T>(state: &LoadState<Page<T>>) -> bool {
    match state {
        LoadState::Loaded(page) => {
            u64::from(page.page).saturating_mul(u64::from(page.page_size)) < page.total
        }
        _ => true,
    }
}

#[derive(Debug)]
/// 下载响应产生的数据块、完成标记或读取错误。
pub enum ShijuDownloadChunk {
    Data(Vec<u8>),
    Complete,
    Failed(SkillHubError),
}

#[cfg(test)]
mod tests;
