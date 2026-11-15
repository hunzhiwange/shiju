//! HTTP 入口使用的配置与真实宿主操作。认证策略和投递能力由组合层注入。
use std::sync::Arc;
use agent_config::runtime::Config;
use axum::{http::{HeaderMap, StatusCode}, Json};

pub type AuthenticationResult = Result<(), (StatusCode, Json<serde_json::Value>)>;

#[derive(Clone)]
pub struct CronState {
    pub config: Arc<parking_lot::Mutex<Config>>,
    pub authenticate: Arc<dyn Fn(&HeaderMap) -> AuthenticationResult + Send + Sync>,
    pub delivery: Arc<dyn agent_cron::delivery::DeliveryHost>,
}
