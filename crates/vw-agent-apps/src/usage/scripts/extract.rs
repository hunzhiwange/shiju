//! Validates and normalizes script output before it leaves the execution host.

use std::fmt;

use serde_json::Value;
use vw_api_types::apps_usage::UsageScriptPackage;

use super::{config::PreparedUsageScript, runtime::RuntimeError};
use crate::apps::lifecycle::operations::Operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractError {
    Runtime(RuntimeError),
    EmptyResult,
    InvalidPackage,
    MissingValues,
    SensitiveResult,
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => error.fmt(f),
            Self::EmptyResult => f.write_str("usage script returned no packages"),
            Self::InvalidPackage => f.write_str("usage script returned an invalid package"),
            Self::MissingValues => f.write_str("usage script package is missing usage values"),
            Self::SensitiveResult => f.write_str("usage script package contains credentials"),
        }
    }
}

impl std::error::Error for ExtractError {}

impl PreparedUsageScript<'_> {
    /// Run the extractor and return only validated package fields.
    pub async fn extract_packages(
        &self,
        response: &str,
    ) -> Result<Vec<UsageScriptPackage>, ExtractError> {
        self.extract_packages_for(response, None).await
    }

    pub(super) async fn extract_packages_for(
        &self,
        response: &str,
        operation: Option<&Operation>,
    ) -> Result<Vec<UsageScriptPackage>, ExtractError> {
        let result = self.extract_for(response, operation).await.map_err(ExtractError::Runtime)?;
        let packages = match result {
            Value::Array(items) if items.is_empty() => return Err(ExtractError::EmptyResult),
            Value::Array(items) => items,
            item => vec![item],
        };
        packages.into_iter().map(|item| validate_package(item, self.credentials)).collect()
    }
}

fn validate_package(
    item: Value,
    credentials: [Option<&str>; 2],
) -> Result<UsageScriptPackage, ExtractError> {
    if !item.is_object() {
        return Err(ExtractError::InvalidPackage);
    }
    let package: UsageScriptPackage =
        serde_json::from_value(item).map_err(|_| ExtractError::InvalidPackage)?;
    // An explicitly invalid plan may only carry a reason, as in the New API template.
    if package.is_valid != Some(false)
        && package.total.is_none()
        && package.used.is_none()
        && package.remaining.is_none()
    {
        return Err(ExtractError::MissingValues);
    }
    let text = [&package.plan_name, &package.extra, &package.invalid_message, &package.unit];
    if text
        .into_iter()
        .filter_map(Option::as_deref)
        .any(|field| credentials.into_iter().flatten().any(|secret| field.contains(secret)))
    {
        return Err(ExtractError::SensitiveResult);
    }
    Ok(package)
}
