//! Gateway 能力目录、集成、Hook 与诊断服务。

pub mod capabilities;
pub mod diagnostics;
pub mod hooks;
pub mod integrations;

#[cfg(test)]
mod diagnostics_tests;
#[cfg(test)]
mod hooks_tests;
