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
#[path = "model_routes_tests.rs"]
mod model_routes_tests;

/// 保存所需的有效路由与跨页查询分类投影。
pub struct RouteProjection { pub rules: Vec<config_routing::ClassificationRule>, pub patch: serde_json::Value }
pub fn project_routes(routes: &[ModelRoute]) -> RouteProjection {
    use config_routing::ClassificationRule;
    let ui_routes = routes
        .iter()
        .filter(|route| {
            !route.pattern.trim().is_empty()
                || !route.provider.trim().is_empty()
                || !route.model.trim().is_empty()
                || !route.priority_input.trim().is_empty()
        })
        .map(|route| config_ui::ModelRoute {
            pattern: route.pattern.trim().to_string(),
            provider: route.provider.trim().to_string(),
            model: route.model.trim().to_string(),
            priority: route.priority_input.trim().parse::<u32>().unwrap_or(0),
        })
        .collect::<Vec<_>>();

    let model_routes = ui_routes
        .iter()
        .map(|route| ModelRouteConfig {
            hint: route.pattern.clone(),
            provider: route.provider.clone(),
            model: route.model.clone(),
            max_tokens: None,
            api_key: None,
        })
        .collect::<Vec<_>>();

    let rules = routes
        .iter()
        .filter_map(|route| {
            let hint = route.pattern.trim();
            if hint.is_empty() {
                return None;
            }
            Some(ClassificationRule {
                hint: hint.to_string(),
                keywords: vec![hint.to_string()],
                patterns: Vec::new(),
                min_length: None,
                max_length: None,
                priority: route.priority_input.trim().parse::<i32>().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();

    let patch = serde_json::json!({
            "app_ui": {"system_settings": {"model_routes": ui_routes}},
            "model_routes": model_routes,
            "query_classification": {"enabled": !rules.is_empty(), "rules": rules}
        });
    RouteProjection { rules, patch }
}

pub fn edit_routes(routes: &mut Vec<ModelRoute>, message: crate::view_message::ModelRoutesMessage) {
    match message {
        crate::view_message::ModelRoutesMessage::AddRoute => {
            routes.push(ModelRoute::default())
        }
        crate::view_message::ModelRoutesMessage::RemoveRoute(index) => {
            if index < routes.len() {
                routes.remove(index);
            }
        }
        crate::view_message::ModelRoutesMessage::PatternChanged(index, value) => {
            if let Some(route) = routes.get_mut(index) {
                route.pattern = value;
            }
        }
        crate::view_message::ModelRoutesMessage::ProviderChanged(index, value) => {
            if let Some(route) = routes.get_mut(index) {
                route.provider = value;
            }
        }
        crate::view_message::ModelRoutesMessage::ModelChanged(index, value) => {
            if let Some(route) = routes.get_mut(index) {
                route.model = value;
            }
        }
        crate::view_message::ModelRoutesMessage::PriorityChanged(index, value) => {
            if let Some(route) = routes.get_mut(index) {
                route.priority_input = value;
            }
        }
    }
}

pub fn loaded_routes(
    routes: Vec<ModelRouteConfig>,
    classification: &config_routing::QueryClassificationConfig,
) -> (Vec<ModelRoute>, Option<Feedback>) {
    let routes = routes.into_iter().map(|route| ModelRoute {
        priority_input: classification.rules.iter()
            .find(|rule| rule.hint == route.hint)
            .map(|rule| rule.priority.to_string())
            .unwrap_or_else(|| "0".into()),
        pattern: route.hint,
        provider: route.provider,
        model: route.model,
    }).collect::<Vec<_>>();
    let error = if routes.is_empty() && !classification.rules.is_empty() {
        Some(Feedback::owned("已检测到 query_classification 规则，但桌面模型路由列表为空", vec![]))
    } else {
        None
    };
    (routes, error)
}
