//! Shared production routing HTTP registration.
pub fn router() -> axum::Router {
    use axum::{
        extract::DefaultBodyLimit,
        http::{header, HeaderValue},
        response::Response,
        routing::{get, post},
    };

    use super::{get_client_host, put_client_host, status, health, request_status};
    use super::lifecycle_api::RoutingControl;
    use axum::{Router, middleware};
    use super::config_api::*;
    use super::lifecycle_api::{start, stop};
    use super::outbound_api::*;
    use super::outbound_probe::*;
    use super::outbound_scan::*;
    use super::mode_attach::attach_mode;
    Router::new()
        .route("/app-routing/hosts/{host}/client-host", get(get_client_host::<RoutingControl>).put(put_client_host::<RoutingControl>))
        .route("/app-routing/hosts/{host}/status", get(status::<RoutingControl>))
        .route("/app-routing/hosts/{host}/projects/{project}/apps/{app}/mode", post(attach_mode))
        .route("/app-routing/hosts/{host}/projects/{project}/apps/{app}/requests", get(request_status::<RoutingControl>))
        .route(
            "/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/scans",
            post(start_outbound_scan::<RoutingControl>),
        )
        .route(
            "/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/scans/{task}",
            get(outbound_scan_status::<RoutingControl>).delete(cancel_outbound_scan::<RoutingControl>),
        )
        .route("/app-routing/hosts/{host}/health", get(health::<RoutingControl>))
        .route("/app-routing/hosts/{host}/start", post(start))
        .route("/app-routing/hosts/{host}/stop", post(stop))
        .route("/app-routing/hosts/{host}/config", get(get_config).put(put_config))
        .route("/app-routing/hosts/{host}/egress", get(get_egress).put(put_egress))
        .route(
            "/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/probes",
            post(start_outbound_probe::<RoutingControl>),
        )
        .route(
            "/app-routing/hosts/{host}/projects/{project}/apps/{app}/egress/probes/{task}",
            get(outbound_probe_status::<RoutingControl>).delete(cancel_outbound_probe::<RoutingControl>),
        )
        .route(
            "/app-routing/hosts/{host}/projects/{project}/apps/{app}/config",
            get(get_app_config).put(put_app_config),
        )
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::map_response(async |mut response: Response| {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}
