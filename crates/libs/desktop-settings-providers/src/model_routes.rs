use crate::state::ModelRoute;
use desktop_i18n::settings_models::messages::Feedback;
use config_routing::ModelRouteConfig;

pub fn validate_routes(input: &[ModelRoute]) -> Result<Vec<ModelRouteConfig>, Feedback> {
    let mut routes = Vec::with_capacity(input.len());
    for (idx, route) in input.iter().enumerate() {
        let pattern = route.pattern.trim();
        let provider = route.provider.trim();
        let model = route.model.trim();
        let priority = route.priority_input.trim();

        if pattern.is_empty() && provider.is_empty() && model.is_empty() && priority.is_empty() {
            continue;
        }
        if pattern.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 pattern", vec![(idx + 1).to_string()]));
        }
        if provider.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 provider", vec![(idx + 1).to_string()]));
        }
        if model.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 model", vec![(idx + 1).to_string()]));
        }
        if !priority.is_empty() && priority.parse::<u32>().is_err() {
            return Err(Feedback::owned(
                "第 %s 条路由的 priority 不是有效数字",
                vec![(idx + 1).to_string()],
            ));
        }

        routes.push(ModelRouteConfig {
            hint: pattern.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            max_tokens: None,
            api_key: None,
        });
    }
    Ok(routes)
}


#[cfg(test)]
mod model_routes_tests;
