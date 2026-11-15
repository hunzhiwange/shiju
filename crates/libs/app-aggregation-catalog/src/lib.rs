//! Pure aggregation catalogs and rules, plus bounded native OMO reads.

pub mod claude_catalog;
#[cfg(test)]
mod claude_catalog_tests;

pub mod codex_catalog;
#[cfg(test)]
mod codex_catalog_tests;

pub mod codex_classic;
#[cfg(test)]
mod codex_classic_tests;

pub mod codex_review;
#[cfg(test)]
mod codex_review_tests;

pub mod codex_subagent;
#[cfg(test)]
mod codex_subagent_tests;

pub mod model_ids;
#[cfg(test)]
mod model_ids_tests;

pub mod omo_read;
#[cfg(test)]
mod omo_read_tests;
pub mod member_keys;
pub mod member_validation;
