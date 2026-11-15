//! QR authorization routes supplied by the channel gateway domain.
pub(crate) use gateway_channel::router;

#[cfg(test)]
#[path = "channel_qr_tests.rs"]
mod channel_qr_tests;
