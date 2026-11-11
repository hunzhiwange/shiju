//! 分能力授权；查询/刷新不触发系统授权。
use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Microphone,
    InputMonitoring,
    Accessibility,
    ScreenRecording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Unknown,
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCapability {
    Recording,
    GlobalKeyListening,
    TextInsertion,
    ScreenSharing,
}

impl VoiceCapability {
    pub fn permission(self) -> Permission {
        match self {
            Self::Recording => Permission::Microphone,
            Self::GlobalKeyListening => Permission::InputMonitoring,
            Self::TextInsertion => Permission::Accessibility,
            Self::ScreenSharing => Permission::ScreenRecording,
        }
    }
}

/// 唯一可替代边界是系统权限；不在构造或查询时请求权限。
pub trait PermissionBackend {
    fn query(&self, permission: Permission) -> PermissionStatus;
    fn request(&self, permission: Permission) -> Result<()>;
    fn open_settings(&self, permission: Permission) -> Result<()>;
}

pub struct VoicePermissions<B> {
    backend: B,
}
impl<B: PermissionBackend> VoicePermissions<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
    /// 返回应用或窗口重新获得焦点时调用，永远重新查询 OS。
    pub fn refresh(&self, permission: Permission) -> PermissionStatus {
        self.backend.query(permission)
    }
    /// 仅响应用户明确的授权动作；返回值不代表异步授权完成。
    pub fn request_from_user_action(&self, permission: Permission) -> Result<()> {
        match self.refresh(permission) {
            PermissionStatus::Granted => Ok(()),
            PermissionStatus::Restricted => bail!("{permission:?} permission is restricted"),
            PermissionStatus::Unsupported => bail!("{permission:?} permission is unsupported"),
            PermissionStatus::Denied => self.backend.open_settings(permission),
            PermissionStatus::Unknown | PermissionStatus::NotDetermined => {
                self.backend.request(permission)
            }
        }
    }
    pub fn open_settings_from_user_action(&self, permission: Permission) -> Result<()> {
        self.backend.open_settings(permission)
    }
    /// 在启动能力之前重新查询；只有 granted 可以执行操作。
    pub fn start<T>(
        &self,
        capability: VoiceCapability,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let permission = capability.permission();
        let status = self.refresh(permission);
        if status != PermissionStatus::Granted {
            bail!("{permission:?} permission is {status:?}; cannot start {capability:?}");
        }
        operation()
    }
}

/// 不缓存系统权限；用户从系统设置返回后用 refresh 重新查询。
pub struct SystemPermissions;

#[cfg(target_os = "macos")]
impl PermissionBackend for SystemPermissions {
    fn query(&self, permission: Permission) -> PermissionStatus {
        super::permissions_macos::query(permission)
    }
    fn request(&self, permission: Permission) -> Result<()> {
        super::permissions_macos::request(permission)
    }
    fn open_settings(&self, permission: Permission) -> Result<()> {
        super::permissions_macos::open_settings(permission)
    }
}

#[cfg(not(target_os = "macos"))]
impl PermissionBackend for SystemPermissions {
    fn query(&self, _: Permission) -> PermissionStatus {
        PermissionStatus::Unsupported
    }
    fn request(&self, permission: Permission) -> Result<()> {
        bail!("{permission:?} permission is unsupported on this platform")
    }
    fn open_settings(&self, permission: Permission) -> Result<()> {
        bail!("{permission:?} settings are unsupported on this platform")
    }
}
