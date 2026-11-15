//! 问题路由组合与可信租户上下文适配。

struct QuestionHost;

impl gateway_question::QuestionHost for QuestionHost {
    fn current_tenant_scope() -> vw_api_transport::TenantScope {
        crate::session::ui_store::current_tenant_scope()
    }
}

pub(crate) fn router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    gateway_question::router::<S, QuestionHost>()
}
