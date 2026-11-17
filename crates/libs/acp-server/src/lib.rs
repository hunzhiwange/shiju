//! 宿主无关的 ACP 服务端协议适配。

#![allow(clippy::result_large_err)]

pub mod server;
pub use server::*;

pub mod aionrs;
