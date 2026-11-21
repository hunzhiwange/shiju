mod skill_boundary;
mod skill_gateway;
pub mod skill_hub;
pub use skill_boundary::*;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn open_url(url: &str) -> bool {
    open::that(url).is_ok()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn open_url(url: &str) -> bool {
    desktop_account::browser::open_external(url)
}
