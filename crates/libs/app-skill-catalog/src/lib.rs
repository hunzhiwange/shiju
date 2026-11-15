//! Skill paths, bounded archives, repository discovery and job snapshots.

pub mod app_paths;
pub mod archive;
pub mod discovery;
pub mod discovery_cache;
pub mod job_cancel;
pub mod jobs;
pub mod market_search;
pub mod repos;

#[cfg(test)]
mod app_paths_tests;

#[cfg(test)]
mod discovery_tests;

#[cfg(test)]
mod job_cancel_tests;

#[cfg(test)]
mod jobs_tests;

#[cfg(test)]
mod market_search_tests;

#[cfg(test)]
mod repos_tests;
