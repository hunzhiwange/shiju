mod lifecycle_api;

pub(crate) use lifecycle_api::{configure, health, start, status, stop};

#[cfg(test)]
mod lifecycle_api_tests;
