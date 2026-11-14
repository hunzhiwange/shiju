use super::AppRoutingError;
use crate::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{RoutingModeRequest, RoutingModeResult},
    id::ProjectId,
};

impl GatewayClient {
    /// Apply proxy, direct or detach through the authorized native transaction.
    pub async fn apply_app_routing_mode(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
        request: &RoutingModeRequest,
    ) -> Result<RoutingModeResult, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            reqwest::Method::POST,
            &[host, "projects", project.as_str(), "apps", app, "mode"],
            token,
            Some(request),
        )
        .await
    }
}
