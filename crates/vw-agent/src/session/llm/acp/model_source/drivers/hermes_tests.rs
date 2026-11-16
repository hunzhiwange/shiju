


#[tokio::test]
async fn acp_model_source_hermes_rejects_missing_credentials_and_disabled_model_before_launch() {
    llm_runtime::acp::acceptance_support::negative::hermes_rejects_native_account::<super::super::super::super::request::RuntimeHost>().await;
}
