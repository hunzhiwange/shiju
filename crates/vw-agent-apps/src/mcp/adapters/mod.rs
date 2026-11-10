//! Application-specific projections for native MCP configurations.

pub mod claude;
#[cfg(test)]
mod claude_tests;

pub mod hermes;
#[cfg(test)]
mod hermes_tests;

pub mod gemini;

#[cfg(test)]
mod gemini_tests;

pub mod kimi;
#[cfg(test)]
mod kimi_tests;

pub mod grokbuild;
#[cfg(test)]
mod grokbuild_tests;

pub mod codex;
#[cfg(test)]
mod codex_tests;

pub mod opencode;

pub mod agy;
#[cfg(test)]
mod agy_tests;

pub mod pi;
#[cfg(test)]
mod pi_tests;

pub mod dsh;

pub mod mcode;
#[cfg(test)]
mod mcode_tests;

pub mod omp;
#[cfg(test)]
mod omp_tests;
