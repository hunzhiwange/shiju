//! External history uses the gateway's tenant and trusted execution-host policy.
mod catalog;
pub(super) use catalog::router;

mod scan;

mod messages;

mod stream;

mod content;

mod images;

mod delete;

mod resume;

mod maintenance;

mod compression;

mod markdown;
