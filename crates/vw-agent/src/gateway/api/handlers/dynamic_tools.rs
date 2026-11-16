//! Bind the domain HTTP adapter to the concrete metadata IO and tool factories.
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    gateway_dynamic_tools::router::<gateway_dynamic_tools::session::SessionDynamicToolsHost<
        crate::session::session_io::NativeSessionIo,
        crate::session::runtime::dynamic_hosts::HostTools,
    >, S>()
}
