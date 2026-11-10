//! Typed reads and replacements of execution-host routing settings.
use super::AppRoutingError;
use crate::GatewayClient;
use reqwest::Method;
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{AppRoutingConfig, RoutingConfig},
    id::ProjectId,
};

impl GatewayClient {
    pub async fn app_routing_config(
        &self,
        host: &str,
        token: &str,
    ) -> Result<RoutingConfig, AppRoutingError> {
        self.app_routing_request(Method::GET, &[host, "config"], token, None::<&()>).await
    }

    pub async fn app_routing_set_config(
        &self,
        host: &str,
        token: &str,
        config: &RoutingConfig,
    ) -> Result<RoutingConfig, AppRoutingError> {
        self.app_routing_request(Method::PUT, &[host, "config"], token, Some(config)).await
    }

    pub async fn app_routing_app_config(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<AppRoutingConfig, AppRoutingError> {
        self.app_routing_app_config_request(Method::GET, host, project, app, token, None).await
    }

    pub async fn app_routing_set_app_config(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
        config: &AppRoutingConfig,
    ) -> Result<AppRoutingConfig, AppRoutingError> {
        self.app_routing_app_config_request(Method::PUT, host, project, app, token, Some(config))
            .await
    }

    async fn app_routing_app_config_request(
        &self,
        method: Method,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
        config: Option<&AppRoutingConfig>,
    ) -> Result<AppRoutingConfig, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            method,
            &[host, "projects", project.as_ref(), "apps", app, "config"],
            token,
            config,
        )
        .await
    }
}
