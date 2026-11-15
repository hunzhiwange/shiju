//! Host integration tests for the real local/plugin inventory and bundled initialization.
//! Keep the shared environment lock with all callers that mutate host discovery paths.

pub(crate) mod helpers;
mod load_skills;
mod skills_init;
