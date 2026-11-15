//! Explicit isolated native acceptance support; never started by the library.
pub mod fixture;
pub mod gateway;
mod roundtrip;
mod scenarios;
pub use roundtrip::roundtrip;
pub use scenarios::Scenarios;
#[cfg(test)]
mod fixture_tests;
