//! Automatic download preferences and native condition checks.
use serde::{Deserialize, Serialize};

pub const CONFIG_KEY: &str = "app_update_automatic_download";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadPolicy {
    pub enabled: bool,
    pub idle: bool,
    pub night: bool,
    pub wifi: bool,
    pub plugged_in: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl DownloadPolicy {
    fn allows(
        self,
        hour: u32,
        idle: impl FnOnce() -> bool,
        wifi: impl FnOnce() -> bool,
        plugged_in: impl FnOnce() -> bool,
    ) -> bool {
        let at_night = !(7..23).contains(&hour);
        self.enabled
            && ((!self.idle && !self.night) || (self.night && at_night) || (self.idle && idle()))
            && (!self.wifi || wifi())
            && (!self.plugged_in || plugged_in())
    }

    pub(crate) fn allows_on_device(self, hour: u32) -> bool {
        self.allows(hour, device::idle, device::wifi, device::plugged_in)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod device;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "automatic_tests.rs"]
mod tests;
