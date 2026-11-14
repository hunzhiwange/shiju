//! Read-only canonical usage ledger endpoints.
mod read;
mod scan;
pub(crate) fn router() -> axum::Router { read::router().merge(scan::router()) }
