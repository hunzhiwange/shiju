//! iFlow 0.5.19 ACP modes; load omits modes and set_mode only acknowledges.
pub const FULL_MODE: &str = "yolo";
pub const RESTRICTED_MODE: &str = "default";

pub fn restricted_mode(mode: &str) -> bool {
    matches!(mode, RESTRICTED_MODE | "plan")
}
