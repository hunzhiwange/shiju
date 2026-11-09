//! Runs the complete host-side script query under one cancellation signal.

use std::fmt;

use reqwest::Url;
use vw_api_types::apps_usage::UsageScriptPackage;

use super::{config::PreparedUsageScript, extract::ExtractError, network::NetworkError};
use crate::apps::lifecycle::operations::Operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    Cancelled,
    Network(NetworkError),
    Extract(ExtractError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("usage script cancelled"),
            Self::Network(error) => error.fmt(f),
            Self::Extract(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RunError {}

impl PreparedUsageScript<'_> {
    /// Run a query using an operation dedicated to this request. Cancellation
    /// waits for QuickJS workers to exit. The caller owns operation status and
    /// coordinates any later cache commit with cancellation.
    pub async fn run(
        &self,
        timeout_secs: u64,
        authorized_origins: &[Url],
        operation: &Operation,
    ) -> Result<Vec<UsageScriptPackage>, RunError> {
        if operation.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        let result = self.build_request_for(Some(operation)).await;
        if operation.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        let candidate = result.map_err(|error| RunError::Network(NetworkError::Runtime(error)))?;
        let response = tokio::select! {
            biased;
            () = operation.cancelled() => return Err(RunError::Cancelled),
            result = self.send_candidate(candidate, timeout_secs, authorized_origins) => result,
        };
        if operation.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        let response = response.map_err(RunError::Network)?;
        let result = self.extract_packages_for(&response, Some(operation)).await;
        if operation.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        result.map_err(RunError::Extract)
    }
}
