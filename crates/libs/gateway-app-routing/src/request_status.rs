use super::AppRoutingError;
use crate::GatewayAppRoutingClient;
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::RoutingRequestsStatus,
    vw_api_transport::id::ProjectId,
};
impl GatewayAppRoutingClient {
    pub async fn app_routing_requests(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<RoutingRequestsStatus, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            reqwest::Method::GET,
            &[host, "projects", project.as_ref(), "apps", app, "requests"],
            token,
            None::<&()>,
        )
        .await
    }
}
