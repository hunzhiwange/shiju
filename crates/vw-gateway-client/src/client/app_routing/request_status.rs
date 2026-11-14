use super::AppRoutingError;
use crate::GatewayClient;
use vw_api_types::{agent_apps::AppId, app_routing::RoutingRequestsStatus, id::ProjectId};
impl GatewayClient {
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
            &[host, "projects", project.as_str(), "apps", app, "requests"],
            token,
            None::<&()>,
        )
        .await
    }
}
