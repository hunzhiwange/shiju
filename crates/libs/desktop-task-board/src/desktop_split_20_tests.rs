//! Local selector event, layout and rendering coverage; host composition remains in vw-desktop.
#![cfg(not(target_arch = "wasm32"))]

mod executor;
mod model;
mod reasoning;
mod ui;
