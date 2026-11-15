//! Host configuration adapter for the skill-market routes.
use std::sync::Arc;
use axum::Router;
use gateway_skill_market::routes::SkillMarketConfig;

struct RuntimeSkillMarketConfig;
impl SkillMarketConfig for RuntimeSkillMarketConfig {
    fn skills(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = config_skills::SkillsConfig> + Send + '_>> {
        Box::pin(async { crate::app::agent::config::get().await.skills.clone() })
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_skill_market::routes::router(Arc::new(RuntimeSkillMarketConfig))
}

#[cfg(test)]
fn router_with_registry<S: Clone + Send + Sync + 'static>(registry: Arc<agent_skills::marketplace::provider::SkillMarketProviderRegistry>) -> Router<S> {
    gateway_skill_market::routes::router_with_registry(registry, Arc::new(RuntimeSkillMarketConfig))
}

#[cfg(test)]
#[path = "skill_market_tests.rs"]
mod skill_market_tests;
