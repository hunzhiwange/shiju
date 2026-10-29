//! 仅准备当前环境的原生技能，并在写盘时读取最新账户凭据。

use super::installer::{
    download_package, install_shiju_archive_at_home, shiju_installation_at_home,
    sync_shiju_key_at_home,
};
use super::{ShijuInstallation, ShijuPackageId, SkillHubClient};
use crate::app::account::SecretSkey;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Default)]
pub(crate) struct NativeShijuSkillState {
    account: Arc<Mutex<NativeSkillAccount>>,
}

#[derive(Debug, Default)]
struct NativeSkillAccount {
    credential: Option<SecretSkey>,
    revision: u64,
    generation: u64,
    busy: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct NativeShijuSkillPrepared {
    pub(crate) revision: u64,
    pub(crate) installation: Option<ShijuInstallation>,
}

impl NativeShijuSkillState {
    /// 与密钥写盘共用锁，账户替换完成后不再允许旧凭据落盘。
    pub(crate) fn set_credential(&self, credential: Option<SecretSkey>) {
        let mut account = self.account.lock();
        if account.credential != credential {
            account.revision = account.revision.wrapping_add(1);
            account.credential = credential;
        }
    }

    pub(crate) fn begin(&self) -> Option<(u64, u64)> {
        let mut account = self.account.lock();
        if account.busy {
            return None;
        }
        account.busy = true;
        account.generation = account.generation.wrapping_add(1);
        Some((account.generation, account.revision))
    }

    pub(crate) fn finish(&self, generation: u64) -> bool {
        let mut account = self.account.lock();
        if !account.busy || account.generation != generation {
            return false;
        }
        account.busy = false;
        true
    }

    pub(crate) fn revision(&self) -> u64 {
        self.account.lock().revision
    }
}

fn native_installation(home: &Path) -> Result<Option<ShijuInstallation>> {
    Ok(shiju_installation_at_home(home)?
        .filter(|installation| installation.installed_dir.join("SKILL.md").is_file()))
}

/// 只在缺失时下载；下载边界可替换，路径校验、安装和密钥写盘始终走真实安装器。
pub(crate) async fn prepare_native_shiju_skill<F, Fut>(
    state: NativeShijuSkillState,
    home: PathBuf,
    download: F,
) -> Result<NativeShijuSkillPrepared>
where
    F: FnOnce(SecretSkey) -> Fut,
    Fut: Future<Output = Result<NamedTempFile>>,
{
    let inspect_home = home.clone();
    let installation = tokio::task::spawn_blocking(move || native_installation(&inspect_home))
        .await
        .context("识矩技能检测任务异常终止")??;
    if installation.is_none() {
        let (credential, revision) = {
            let account = state.account.lock();
            (account.credential.clone(), account.revision)
        };
        let Some(credential) = credential else {
            return Ok(NativeShijuSkillPrepared { revision, installation: None });
        };
        let archive = download(credential).await?;
        let install_home = home.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            // 下载期间用户可能已手动安装，不覆盖现有安装及其配置。
            if native_installation(&install_home)?.is_none() {
                install_shiju_archive_at_home(archive.path(), &install_home)?;
            }
            Ok(())
        })
        .await
        .context("识矩技能安装任务异常终止")??;
    }

    tokio::task::spawn_blocking(move || {
        // 锁仅覆盖本地同步，不跨越网络请求；退出或切换后使用最新账户。
        let account = state.account.lock();
        if let Some(credential) = &account.credential {
            sync_shiju_key_at_home(&home, credential)?;
        }
        Ok(NativeShijuSkillPrepared {
            revision: account.revision,
            installation: native_installation(&home)?,
        })
    })
    .await
    .context("识矩登录信息同步任务异常终止")?
}

pub(crate) async fn download_native_shiju_skill(credential: SecretSkey) -> Result<NamedTempFile> {
    let package_id = ShijuPackageId::current().context("当前平台不支持识矩技能自动安装")?;
    let release = SkillHubClient::new(credential)?.latest_shiju_package(package_id).await?;
    let mut archive = NamedTempFile::new().context("无法创建识矩技能临时下载文件")?;
    download_package(&release.download_url, archive.as_file_mut()).await?;
    archive.flush().context("无法写完识矩技能临时下载文件")?;
    Ok(archive)
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
