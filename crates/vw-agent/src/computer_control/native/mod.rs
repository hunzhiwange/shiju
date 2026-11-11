//! Application-scoped observations for a trusted local host session.
//! Observations never prompt. Only explicit trusted UI requests may ask macOS for access.

use crate::computer_control::policy::ComputerControlPolicy;
use base64::Engine;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::computer_control::*;

#[cfg(test)]
mod input_tests;
mod macos_input;
#[cfg(target_os = "macos")]
mod macos_observation;
#[cfg(test)]
mod observation_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeObservationError {
    Control(ControlErrorCode),
    AccessibilityRequired,
    ScreenRecordingRequired,
}
impl NativeObservationError {
    /// Guidance describes a real missing permission; this never requests it.
    pub fn guidance(&self) -> &'static str {
        match self {
            Self::AccessibilityRequired => {
                "Allow the running ShiJu host in System Settings > Privacy & Security > Device Control and Data Access (Accessibility on older macOS), then retry in that same host."
            }
            Self::ScreenRecordingRequired => {
                "Allow the running ShiJu host in System Settings > Privacy & Security > Screen & System Audio Recording, then restart that host if macOS requests it."
            }
            Self::Control(_) => "Native observation unavailable; recheck the session and target.",
        }
    }
    pub fn code(&self) -> ControlErrorCode {
        match self {
            Self::Control(code) => *code,
            Self::AccessibilityRequired | Self::ScreenRecordingRequired => {
                ControlErrorCode::PermissionDenied
            }
        }
    }
}
impl From<ControlErrorCode> for NativeObservationError {
    fn from(code: ControlErrorCode) -> Self {
        Self::Control(code)
    }
}

/// Process start time prevents a recycled PID from selecting a different instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationInstance {
    pub bundle_id: String,
    pub process_id: u32,
    pub start_seconds: u64,
    pub start_microseconds: u64,
}
impl ApplicationInstance {
    pub fn target(&self) -> ComputerControlTarget {
        ComputerControlTarget::NativeApp {
            bundle_id: self.bundle_id.clone(),
            process_id: self.process_id,
        }
    }
}
/// Only the two system permissions required by native observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePermission {
    Accessibility,
    ScreenRecording,
}
impl NativePermission {
    pub fn settings_url(self) -> &'static str {
        match self {
            Self::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            Self::ScreenRecording => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePermissions {
    pub accessibility: bool,
    pub screen_recording: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
struct WindowGeometry {
    window_id: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
/// Quartz global logical points (top left), including negative display origins.
/// Pixel ratios may differ after a bounded resize; never assume a Retina scale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScreenshotGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
}
impl ScreenshotGeometry {
    fn new(
        w: WindowGeometry,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Result<Self, NativeObservationError> {
        if ![w.x, w.y, w.width, w.height].iter().all(|v| v.is_finite())
            || w.width <= 0.0
            || w.height <= 0.0
            || pixel_width == 0
            || pixel_height == 0
        {
            return Err(ControlErrorCode::BackendFailure.into());
        }
        Ok(Self { x: w.x, y: w.y, width: w.width, height: w.height, pixel_width, pixel_height })
    }
    pub fn screen_point(&self, x: u32, y: u32) -> Result<(f64, f64), NativeObservationError> {
        if ![self.x, self.y, self.width, self.height].iter().all(|v| v.is_finite())
            || self.width <= 0.0
            || self.height <= 0.0
            || x >= self.pixel_width
            || y >= self.pixel_height
        {
            return Err(ControlErrorCode::InvalidRequest.into());
        }
        Ok((
            self.x + f64::from(x) * self.width / f64::from(self.pixel_width),
            self.y + f64::from(y) * self.height / f64::from(self.pixel_height),
        ))
    }
}
struct PlatformObservation {
    tree: serde_json::Value,
    window: Option<WindowGeometry>,
}
// The only simulated boundary in tests is the OS. Policy and media stay real.
#[async_trait::async_trait]
trait ObservationPlatform: Send + Sync {
    fn request_permission(
        &self,
        permission: NativePermission,
    ) -> Result<(), NativeObservationError>;
    fn permissions(&self) -> NativePermissions;
    fn applications(&self) -> Result<Vec<ApplicationInstance>, NativeObservationError>;
    fn identity(&self, pid: u32) -> Result<ApplicationInstance, NativeObservationError>;
    fn read(
        &self,
        app: &ApplicationInstance,
    ) -> Result<PlatformObservation, NativeObservationError>;
    async fn capture(
        &self,
        app: &ApplicationInstance,
        window: WindowGeometry,
    ) -> Result<Vec<u8>, NativeObservationError>;
}
fn platform() -> Result<Box<dyn ObservationPlatform>, NativeObservationError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos_observation::MacOsObservation))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(ControlErrorCode::Unsupported.into())
    }
}
fn local_user() -> String {
    #[cfg(unix)]
    {
        // SAFETY: geteuid has no preconditions.
        format!("uid:{}", unsafe { libc::geteuid() })
    }
    #[cfg(not(unix))]
    {
        String::new()
    }
}
pub fn native_permissions() -> Result<NativePermissions, NativeObservationError> {
    Ok(platform()?.permissions())
}
/// Called only after an explicit user action in the owning host. Never call this
/// from observation, enumeration or a model tool request. An OS prompt is not a grant.
/// The owning .app must be registered with Launch Services so macOS can resolve
/// its bundle ID when showing the permissions list; a valid signature alone is insufficient.
pub fn request_native_permission(
    permission: NativePermission,
) -> Result<NativePermissions, NativeObservationError> {
    request_permission_with(platform()?.as_ref(), permission)
}
fn request_permission_with(
    backend: &dyn ObservationPlatform,
    permission: NativePermission,
) -> Result<NativePermissions, NativeObservationError> {
    let before = backend.permissions();
    let granted = match permission {
        NativePermission::Accessibility => before.accessibility,
        NativePermission::ScreenRecording => before.screen_recording,
    };
    if !granted {
        backend.request_permission(permission)?;
    }
    Ok(backend.permissions())
}
pub fn available_applications(
    policy: &RwLock<ComputerControlPolicy>,
) -> Result<Vec<ApplicationInstance>, NativeObservationError> {
    enumerate_with(policy, platform()?.as_ref())
}
fn enumerate_with(
    policy: &RwLock<ComputerControlPolicy>,
    backend: &dyn ObservationPlatform,
) -> Result<Vec<ApplicationInstance>, NativeObservationError> {
    let applications = backend.applications()?;
    let policy = policy.read();
    let user = local_user();
    Ok(applications
        .into_iter()
        .filter(|a| policy.authorize(&user, &a.target(), false, policy.version()).is_ok())
        .collect())
}
struct StoredScreenshot {
    reference: ScreenshotReference,
    png: Vec<u8>,
    window: WindowGeometry,
    geometry: ScreenshotGeometry,
}
/// Created only by trusted host code after issuing its target-bound session.
/// Drop on disconnect/stop. Cancellation, lock state and live policy are rechecked
/// before reading and before delivery; screenshots live only in this session.
pub struct NativeObserver {
    policy: Arc<RwLock<ComputerControlPolicy>>,
    locked: Arc<AtomicBool>,
    session_id: ControlSessionId,
    version: PermissionVersion,
    app: ApplicationInstance,
    cancelled: CancellationToken,
    backend: Box<dyn ObservationPlatform>,
    media: Mutex<Option<StoredScreenshot>>,
    observe_gate: tokio::sync::Mutex<()>,
}
impl NativeObserver {
    pub fn new(
        policy: Arc<RwLock<ComputerControlPolicy>>,
        locked: Arc<AtomicBool>,
        session_id: ControlSessionId,
        version: PermissionVersion,
        target: ComputerControlTarget,
        cancelled: CancellationToken,
    ) -> Result<Self, NativeObservationError> {
        Self::with_platform(policy, locked, session_id, version, target, cancelled, platform()?)
    }
    fn with_platform(
        policy: Arc<RwLock<ComputerControlPolicy>>,
        locked: Arc<AtomicBool>,
        session_id: ControlSessionId,
        version: PermissionVersion,
        target: ComputerControlTarget,
        cancelled: CancellationToken,
        backend: Box<dyn ObservationPlatform>,
    ) -> Result<Self, NativeObservationError> {
        if cancelled.is_cancelled() {
            return Err(ControlErrorCode::Cancelled.into());
        }
        if locked.load(Ordering::SeqCst) {
            return Err(ControlErrorCode::Unsupported.into());
        }
        policy.read().authorize(&local_user(), &target, false, version)?;
        let ComputerControlTarget::NativeApp { bundle_id, process_id } = target else {
            return Err(ControlErrorCode::InvalidRequest.into());
        };
        let app = backend.identity(process_id)?;
        if app.bundle_id != bundle_id || app.process_id != process_id {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        Ok(Self {
            policy,
            locked,
            session_id,
            version,
            app,
            cancelled,
            backend,
            media: Mutex::new(None),
            observe_gate: tokio::sync::Mutex::new(()),
        })
    }
    pub fn application(&self) -> &ApplicationInstance {
        &self.app
    }
    fn check(&self, screenshot: bool) -> Result<(), NativeObservationError> {
        if self.cancelled.is_cancelled() {
            return Err(ControlErrorCode::Cancelled.into());
        }
        // This backend has no locked execution mechanism, even when policy allows it.
        if self.locked.load(Ordering::SeqCst) {
            return Err(ControlErrorCode::Unsupported.into());
        }
        self.policy.read().authorize(&local_user(), &self.app.target(), false, self.version)?;
        if self.backend.identity(self.app.process_id)? != self.app {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        let permissions = self.backend.permissions();
        if !permissions.accessibility {
            return Err(NativeObservationError::AccessibilityRequired);
        }
        if screenshot && !permissions.screen_recording {
            return Err(NativeObservationError::ScreenRecordingRequired);
        }
        Ok(())
    }
    pub async fn observe(&self, screenshot: bool) -> Result<ControlOutput, NativeObservationError> {
        let _gate = self.observe_gate.lock().await;
        *self.media.lock() = None;
        self.check(screenshot)?;
        let observed = self.backend.read(&self.app)?;
        self.check(screenshot)?;
        let mut screenshots = Vec::new();
        let mut geometry = None;
        let mut media = None;
        if screenshot {
            let window = observed
                .window
                .ok_or(NativeObservationError::Control(ControlErrorCode::TargetChanged))?;
            let bytes = tokio::select! {
                biased;
                _ = self.cancelled.cancelled() => return Err(ControlErrorCode::Cancelled.into()),
                result = self.backend.capture(&self.app,window) => result?,
            };
            let (png, width, height) = bounded_png(&bytes)?;
            let screenshot_geometry = ScreenshotGeometry::new(window, width, height)?;
            geometry = Some(screenshot_geometry);
            let reference = ScreenshotReference::new(
                uuid::Uuid::new_v4().to_string(),
                ScreenshotMimeType::Png,
                png.len() as u64,
                width,
                height,
            )
            .map_err(|_| NativeObservationError::Control(ControlErrorCode::BackendFailure))?;
            screenshots.push(reference.clone());
            media =
                Some(StoredScreenshot { reference, png, window, geometry: screenshot_geometry });
        }
        let text = serde_json::to_string(&serde_json::json!({"session_id": self.session_id, "application":self.app,"tree":observed.tree,"screenshot_geometry":geometry})).map_err(|_| NativeObservationError::Control(ControlErrorCode::BackendFailure))?;
        if text.len() > 128 * 1024 {
            return Err(ControlErrorCode::BackendFailure.into());
        }
        self.check(screenshot)?;
        *self.media.lock() = media;
        Ok(ControlOutput::Observation { text, screenshots })
    }
    /// Called by the owning session's image resolver. IDs are not bearer tokens.
    /// Returning the existing image data-URI format allows model image delivery.
    pub fn resolve_screenshot(
        &self,
        reference: &ScreenshotReference,
    ) -> Result<String, NativeObservationError> {
        if let Err(error) = self.check(true) {
            *self.media.lock() = None;
            return Err(error);
        }
        let media = self.media.lock();
        let stored = media
            .as_ref()
            .filter(|stored| stored.reference == *reference)
            .ok_or(NativeObservationError::Control(ControlErrorCode::PermissionDenied))?;
        let data = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&stored.png)
        );
        drop(media);
        if let Err(error) = self.check(true) {
            *self.media.lock() = None;
            return Err(error);
        }
        Ok(data)
    }
}
fn bounded_png(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), NativeObservationError> {
    use std::io::Cursor;
    let failure = || NativeObservationError::Control(ControlErrorCode::BackendFailure);
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(failure());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| failure())?;
    let image = if image.width() > 2048 || image.height() > 2048 {
        image.thumbnail(2048, 2048)
    } else {
        image
    };
    let (width, height) = (image.width(), image.height());
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).map_err(|_| failure())?;
    let png = png.into_inner();
    if png.is_empty() || png.len() as u64 > MAX_SCREENSHOT_BYTES {
        return Err(failure());
    }
    Ok((png, width, height))
}
