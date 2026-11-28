//! Structured errors from the native directory gateway.
/// Preserve gateway status and structured errors without replaying writes.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationApiError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AggregationApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, .. } => write!(f, "aggregation HTTP {status}"),
        }
    }
}
impl std::error::Error for AggregationApiError {}
