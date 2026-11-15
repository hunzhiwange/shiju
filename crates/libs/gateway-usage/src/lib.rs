//! Canonical usage ledger and host maintenance endpoints.
mod usage;
mod read;
mod scan;
mod events;
mod prices;
pub mod host;
pub use host::Host;

pub fn router(host: Host) -> axum::Router {
    read::router(host).merge(scan::router(host)).merge(events::router(host)).merge(prices::router(host))
}
