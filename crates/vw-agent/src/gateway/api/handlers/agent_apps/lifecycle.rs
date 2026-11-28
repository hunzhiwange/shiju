//! Compose native application lifecycle operations and prompt synchronization.
pub(super) use gateway_agent_apps::lifecycle::status;

#[cfg(target_os = "macos")]
mod prompts {
    use super::super::super::agent_app_prompt_sync::{Job, Jobs};
    use axum::{
        http::HeaderMap,
        response::{IntoResponse, Response},
    };
    use std::sync::Arc;
    use vw_api_transport::TenantScope;

    struct PromptOperation(Arc<Job>);
    impl gateway_types::app_operations::AppOperation for PromptOperation {
        fn operation(&self) -> &vw_agent_apps::apps::lifecycle::operations::Operation {
            &self.0.operation
        }

        fn response(&self, id: String) -> Response {
            self.0.response(id)
        }
    }

    struct Sources(Option<Jobs>);
    impl gateway_types::app_operations::AppOperationSource for Sources {
        fn resolve(
            &self,
            tenant: TenantScope,
            headers: HeaderMap,
            host: String,
            id: String,
            policy: Option<std::path::PathBuf>,
            _: bool,
        ) -> gateway_types::app_operations::Resolution<'_> {
            Box::pin(async move {
                let Some(jobs) = &self.0 else { return Ok(None) };
                let entry = jobs
                    .owned(&id, &tenant, &host, &headers, policy.as_deref())
                    .map_err(IntoResponse::into_response)?;
                Ok(entry.map(|entry| {
                    Arc::new(PromptOperation(entry))
                        as Arc<dyn gateway_types::app_operations::AppOperation>
                }))
            })
        }
    }

    pub(super) async fn inject_sources(
        mut request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> Response {
        let jobs = request.extensions().get::<Jobs>().cloned();
        request
            .extensions_mut()
            .insert(gateway_types::app_operations::OperationSources(Arc::new(Sources(jobs))));
        next.run(request).await
    }
}

pub(super) fn router() -> axum::Router {
    router_with_policy(
        std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from),
    )
}

pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> axum::Router {
    let router = gateway_agent_apps::lifecycle::router_with_policy(policy);
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(prompts::inject_sources));
    router
}
