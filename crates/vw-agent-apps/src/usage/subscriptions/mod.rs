//! Official subscription queries on the authorized execution host.

pub mod agy;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod gemini;
pub mod grok;
pub mod kimi;

#[cfg(test)]
mod agy_tests;

#[cfg(test)]
mod claude_tests;

#[cfg(test)]
pub(super) mod codex_tests;

#[cfg(test)]
mod copilot_tests;

#[cfg(test)]
mod gemini_tests;

#[cfg(test)]
mod grok_tests;

#[cfg(test)]
mod kimi_tests;
