//! Local preferences are usable without a paired/reachable gateway.
use super::*;
use crate::app::voice::{
    local,
    permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions},
    preferences::VoiceEngine,
};
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct LocalResources {
    pub voices: Vec<local::SystemVoice>,
    pub languages: Vec<String>,
    pub capability: Option<local::Capabilities>,
    pub microphone_permission: Option<PermissionStatus>,
    pub speech_permission: Option<PermissionStatus>,
    pub warning: Option<String>,
}
/// Queries only: neither authorization, model download nor gateway access.
pub(crate) fn resources(language: Option<&str>) -> LocalResources {
    let mut result = LocalResources {
        microphone_permission: Some(SystemPermissions.query(Permission::Microphone)),
        speech_permission: Some(SystemPermissions.query(Permission::SpeechRecognition)),
        ..Default::default()
    };
    match local::voices() {
        Ok(voices) => result.voices = voices,
        Err(_) => {
            result.warning = Some(
                "无法读取已安装的 Apple 系统声音，请在 macOS 辅助功能朗读设置检查声音资源。".into(),
            )
        }
    }
    let mut candidates: Vec<String> = result.voices.iter().map(|v| v.language.clone()).collect();
    candidates.extend(["zh-CN".into(), "en-US".into()]);
    candidates.extend(language.map(str::to_owned));
    candidates.sort();
    candidates.dedup();
    result.languages = candidates
        .into_iter()
        .filter(|tag| local::capabilities(Some(tag)).is_ok_and(|c| c.supports_on_device))
        .collect();
    match local::capabilities(language) {
        Ok(capability) => result.capability = Some(capability),
        Err(_) => result.warning = Some("所选语言的本机识别器不可用，请选择受支持语言或在 macOS 键盘／听写设置检查本机资源。不会回退联网识别。".into()),
    }
    result
}
pub(super) async fn cloud(endpoint: GatewayEndpoint) -> Result<VoiceConfig, String> {
    tokio::time::timeout(Duration::from_secs(10), async {
        GatewayClient::new(endpoint)?.voice_config::<VoiceConfig>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "读取云端语音设置超时，请检查网关连接。".to_owned())?
}
pub(super) async fn load(endpoint: GatewayEndpoint) -> Result<Loaded, String> {
    load_at(endpoint, Preferences::path()?).await
}
pub(super) async fn load_at(
    endpoint: GatewayEndpoint,
    path: std::path::PathBuf,
) -> Result<Loaded, String> {
    let (local, devices, warning, resources) = tokio::task::spawn_blocking(move || {
        let (local, mut warning) = match Preferences::read(&path) {
            Ok(local) => (local, None),
            Err(error) => (Preferences::default(), Some(format!("{error} 保存将重置本机偏好。"))),
        };
        let devices = input_devices().unwrap_or_else(|_| {
            warning = Some("无法列出麦克风；可先保存偏好，使用前请重新连接设备。".into());
            vec![]
        });
        let resources = if local.engine == VoiceEngine::Local {
            resources(local.local_language.as_deref())
        } else {
            LocalResources::default()
        };
        (local, devices, warning, resources)
    })
    .await
    .map_err(|_| "无法读取本机语音设置。".to_owned())?;
    let (config, cloud_ready, cloud_error) = if local.engine == VoiceEngine::Cloud {
        match cloud(endpoint).await {
            Ok(config) => (config, true, None),
            Err(error) => (VoiceConfig::default(), false, Some(error)),
        }
    } else {
        (VoiceConfig::default(), false, None)
    };
    Ok(Loaded { config, local, devices, warning, resources, cloud_ready, cloud_error })
}
pub(super) async fn save(
    edit: Edit,
    endpoint: GatewayEndpoint,
    cloud_ready: bool,
    path: &Path,
) -> Result<(), String> {
    edit.local.validate()?;
    if edit.local.engine == VoiceEngine::Cloud {
        if !cloud_ready {
            return Err("云端设置尚未加载，请重新读取网关设置或选择 macOS 本机。".into());
        }
        let patch = edit.patch()?;
        tokio::time::timeout(Duration::from_secs(10), async {
            GatewayClient::new(endpoint)?.voice_settings_patch(&patch).await
        })
        .await
        .map_err(|_| "保存云端语音设置超时，请检查网关。".to_owned())??;
    }
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || edit.local.write(&path))
        .await
        .map_err(|_| "无法保存本机语音偏好，请重试。".to_owned())?
}
