pub mod edit;
pub mod agy;
pub mod claude;
pub mod codex;
pub mod dsh;
pub mod gemini;
pub mod grok;
pub mod hermes;
pub mod kimi;
pub mod mcode;
pub mod omp;
pub mod openclaw;
pub mod opencode;
pub mod pi;
pub mod icons;
pub mod endpoints;
pub mod env;
pub mod desktop;

#[cfg(test)]
mod edit_test_state;

#[cfg(test)]
mod agy_tests;

#[cfg(test)]
mod claude_tests;

#[cfg(test)]
mod codex_tests;

pub mod provider_update;

#[cfg(test)]
mod desktop_tests;

#[cfg(test)]
mod dsh_tests;

#[cfg(test)]
mod edit_tests;

#[cfg(test)]
mod gemini_tests;

#[cfg(test)]
mod grok_tests;

#[cfg(test)]
mod hermes_tests;

#[cfg(test)]
mod kimi_tests;

#[cfg(test)]
mod endpoints_tests;

#[cfg(test)]
mod env_tests;

#[cfg(test)]
mod icons_tests;

pub mod list;

pub mod leave;
