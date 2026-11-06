//! iFlow 0.5.19 ACP modes; load omits modes and set_mode only acknowledges.
pub(crate) const FULL_MODE: &str = "yolo";
pub(crate) const RESTRICTED_MODE: &str = "default";

pub(crate) fn restricted_mode(mode: &str) -> bool {
    matches!(mode, RESTRICTED_MODE | "plan")
}
