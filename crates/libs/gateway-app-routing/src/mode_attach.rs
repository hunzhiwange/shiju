use super::AppRoutingError;
use crate::GatewayAppRoutingClient;
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::{RoutingAttachment, RoutingModeRequest},
    vw_api_app_routing::RoutingModeResult,
    vw_api_transport::id::ProjectId,
};

impl GatewayAppRoutingClient {
    pub async fn app_routing_attachment(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<RoutingAttachment, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            reqwest::Method::GET,
            &[host, "projects", project.as_ref(), "apps", app, "mode"],
            token,
            None::<&()>,
        )
        .await
    }

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
            &[host, "projects", project.as_ref(), "apps", app, "mode"],
            token,
            Some(request),
        )
        .await
    }
}
