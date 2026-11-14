//! Canonical usage ledger and host maintenance endpoints.
mod read;
mod scan;
mod prices;
pub(crate) fn router() -> axum::Router { read::router().merge(scan::router()).merge(prices::router()) }
