//! Recovery test fixture composition.
pub struct RecoveryConfig { _config: llm_runtime::recovery_test_support::RecoveryConfig }
impl RecoveryConfig {
    pub async fn new(api: &str, key: Option<&str>) -> Self {
        super::prompt::ensure_host();
        Self { _config: llm_runtime::recovery_test_support::RecoveryConfig::new(api, key).await }
    }
}
#[path = "../../../../libs/llm-runtime/src/recovery_test_support_tests.rs"]
mod tests;
