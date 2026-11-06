//! Native application business logic for VibeWindow.
//!
//! Domain implementations belong in this crate; storage and runtime adapters
//! remain with callers. The preserved `upstream` snapshot is reference material
//! and is not part of this library's module tree or build.

pub mod icons;

pub mod apps;

pub mod providers;

pub mod auth;
