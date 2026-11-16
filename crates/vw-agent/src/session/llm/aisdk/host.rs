//! Host authorization preserves the application tenant, project and credential boundaries.
pub(super) struct NativeHost;
impl llm_runtime::aisdk::host::Host for NativeHost {
    fn begin_usage(&self, provider: &str, model: &str) -> Option<agent_usage::native::NativeRequest> {
        crate::usage_bridge::native::NativeRequest::begin(provider, model, None)
    }
    fn authorize_model_request<'a>(&'a self, model: &'a str)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            let config = crate::config::get().await;
            crate::hooks::authorize_model_request(&config, model).await
        })
    }
}
