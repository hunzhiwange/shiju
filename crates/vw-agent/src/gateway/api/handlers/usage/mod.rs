//! Canonical usage ledger and host maintenance endpoints.
mod read;
mod scan;
mod events;
mod prices;
pub(crate) fn router() -> axum::Router { read::router().merge(scan::router()).merge(events::router()).merge(prices::router()) }
