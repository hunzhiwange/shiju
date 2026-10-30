//! Confirmation progress for one channel send operation, without message content or credentials.

use std::fmt;

/// Whether the failed part was never submitted, explicitly rejected, or has an unknown outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeliveryFailureKind {
    NotSent,
    Rejected,
    Unknown,
}

/// A confirmed prefix belongs only to this send call; this is not cross-call deduplication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChannelDeliveryError {
    pub confirmed_parts: usize,
    pub total_parts: usize,
    /// Zero-based index of the part that could not be confirmed.
    pub failed_part_index: usize,
    pub kind: DeliveryFailureKind,
}

impl fmt::Display for ChannelDeliveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "channel delivery {:?}: confirmed {}/{} parts, failed part {}",
            self.kind, self.confirmed_parts, self.total_parts, self.failed_part_index
        )
    }
}

impl std::error::Error for ChannelDeliveryError {}

#[cfg(test)]
#[path = "outbound_delivery_tests.rs"]
mod tests;
