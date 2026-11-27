//! Explicit, one-time WeCom conversation binding for the notification-only robot.
use super::runtime::{Registration, WeComRuntimeScope, credential_digest, registry};
use agent_config::schema::WeComConfig;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(super) struct Pairing {
    code: String,
    expires_at: Instant,
}

pub fn start_cron_binding(scope: &WeComRuntimeScope, bot_id: &str, secret: &str) -> Result<String> {
    ensure!(WeComConfig::is_cron_instance(&scope.instance), "企微定时任务机器人未配置");
    let mut entries = registry().lock().unwrap_or_else(|err| err.into_inner());
    let entry = entries
        .get_mut(scope)
        .filter(|entry| entry.credentials == Some(credential_digest(bot_id, secret)))
        .filter(|entry| entry.sender.as_ref().is_some_and(|sender| !sender.is_closed()))
        .context("企微定时任务机器人尚未连接，请稍后重试")?;
    let code = format!("绑定 {}", Uuid::new_v4().simple());
    entry.pairing =
        Some(Pairing { code: code.clone(), expires_at: Instant::now() + Duration::from_secs(300) });
    Ok(code)
}

pub fn cron_binding_pending(scope: &WeComRuntimeScope, bot_id: &str, secret: &str) -> bool {
    let entries = registry().lock().unwrap_or_else(|err| err.into_inner());
    entries
        .get(scope)
        .filter(|entry| entry.credentials == Some(credential_digest(bot_id, secret)))
        .and_then(|entry| entry.pairing.as_ref())
        .is_some_and(|pairing| pairing.expires_at > Instant::now())
}

pub fn cron_recipient(
    scope: &WeComRuntimeScope,
    bot_id: &str,
    secret: &str,
) -> Result<Option<String>> {
    match fs::read(binding_path(scope)) {
        Ok(bytes) => {
            let (fingerprint, recipient): (String, String) = serde_json::from_slice(&bytes)
                .context("企微定时任务接收会话绑定无效，请重新绑定")?;
            let expected = hex(&credential_digest(bot_id, secret));
            Ok((fingerprint == expected && !recipient.trim().is_empty()).then_some(recipient))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(_) => anyhow::bail!("企微定时任务接收会话读取失败"),
    }
}

pub fn forget_cron_binding(scope: &WeComRuntimeScope) -> Result<()> {
    let mut entries = registry().lock().unwrap_or_else(|err| err.into_inner());
    if let Some(entry) = entries.get_mut(scope) {
        entry.pairing = None;
    }
    match fs::remove_file(binding_path(scope)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => anyhow::bail!("企微定时任务接收会话解绑失败"),
    }
}

impl WeComRuntimeScope {
    pub(super) fn is_cron(&self) -> bool {
        WeComConfig::is_cron_instance(&self.instance)
    }
}

impl Registration {
    pub(super) fn is_cron(&self) -> bool {
        self.scope.is_cron()
    }

    pub(super) fn bind_cron_chat(&self, text: &str, recipient: &str) -> Result<()> {
        if !self.is_cron() || recipient.trim().is_empty() {
            return Ok(());
        }
        let mut entries = registry().lock().unwrap_or_else(|err| err.into_inner());
        let Some(entry) =
            entries.get_mut(&self.scope).filter(|entry| entry.generation == self.generation)
        else {
            return Ok(());
        };
        let Some(pairing) = entry.pairing.as_ref() else {
            return Ok(());
        };
        if pairing.expires_at <= Instant::now() || text.trim() != pairing.code {
            return Ok(());
        }
        let Some(credentials) = entry.credentials else {
            return Ok(());
        };
        let path = binding_path(&self.scope);
        let payload = serde_json::to_vec(&(hex(&credentials), recipient))?;
        write_atomic(&path, &payload)
            .map_err(|_| anyhow::anyhow!("企微定时任务接收会话保存失败"))?;
        entry.pairing = None;
        Ok(())
    }
}

fn binding_path(scope: &WeComRuntimeScope) -> PathBuf {
    let mut hash = Sha256::new();
    for value in [
        scope.tenant.platform_id().as_ref().as_bytes(),
        scope.tenant.company_id().as_ref().as_bytes(),
        scope.tenant.user_id().as_ref().as_bytes(),
        scope.config_store.as_os_str().as_encoded_bytes(),
        scope.config_source.as_os_str().as_encoded_bytes(),
    ] {
        hash.update(value.len().to_be_bytes());
        hash.update(value);
    }
    scope.workspace.join("cron").join(format!("wecom-binding-{}.json", hex(&hash.finalize())))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write_atomic(path: &std::path::Path, payload: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().expect("binding file has a parent");
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".wecom-binding-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(payload)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
