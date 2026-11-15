//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。


use desktop_i18n::settings_models::messages::Feedback;
use crate::app_init::state::{EmbeddingRouteDraft, EmbeddingRoutesSettingsState};

use config_routing::EmbeddingRouteConfig;

use crate::embedding_view_message::EmbeddingRoutesMessage;

fn alibaba_template_route() -> EmbeddingRouteDraft {
    EmbeddingRouteDraft {
        pattern: "semantic".to_string(),
        provider: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        dimensions: "1024".to_string(),
        api_key_input: String::new(),
    }
}

fn doubao_template_route() -> EmbeddingRouteDraft {
    EmbeddingRouteDraft {
        pattern: "multimodal".to_string(),
        provider: "custom:https://ark.cn-beijing.volces.com/api/v3".to_string(),
        model: "doubao-embedding-vision-250615".to_string(),
        dimensions: String::new(),
        api_key_input: String::new(),
    }
}

fn shiju_template_route() -> EmbeddingRouteDraft {
    EmbeddingRouteDraft {
        pattern: "shiju".to_string(),
        provider: "custom:https://router.shiju.cloud/v1".to_string(),
        model: "text-embedding-v4".to_string(),
        dimensions: "1024".to_string(),
        api_key_input: String::new(),
    }
}

fn persist_embedding_routes_settings(state: &mut EmbeddingRoutesSettingsState) -> Option<Vec<EmbeddingRouteConfig>> {
    let mut routes = Vec::with_capacity(state.routes.len());

    for (index, draft) in state.routes.iter().enumerate() {
        let pattern = draft.pattern.trim().to_string();
        let provider = draft.provider.trim().to_string();
        let model = draft.model.trim().to_string();

        if pattern.is_empty() {
            state.save_error = Some(Feedback::owned(
                "第 %s 条路由的 pattern 不能为空",
                vec![(index + 1).to_string()],
            ));
            state.save_success = false;
            return None;
        }
        if provider.is_empty() {
            state.save_error = Some(Feedback::owned(
                "第 %s 条路由的 provider 不能为空",
                vec![(index + 1).to_string()],
            ));
            state.save_success = false;
            return None;
        }
        if model.is_empty() {
            state.save_error = Some(Feedback::owned(
                "第 %s 条路由的 model 不能为空",
                vec![(index + 1).to_string()],
            ));
            state.save_success = false;
            return None;
        }

        let dimensions = if draft.dimensions.trim().is_empty() {
            None
        } else {
            match draft.dimensions.trim().parse::<u32>() {
                Ok(value) if value > 0 => Some(value as usize),
                _ => {
                    state.save_error = Some(Feedback::owned(
                        "第 %s 条路由的 dimensions 必须是大于 0 的整数",
                        vec![(index + 1).to_string()],
                    ));
                    state.save_success = false;
                    return None;
                }
            }
        };

        routes.push(EmbeddingRouteConfig {
            hint: pattern,
            provider,
            model,
            dimensions,
            api_key: trim_to_option(&draft.api_key_input),
        });
    }

    state.save_error = None;
    state.save_success = false;
    Some(routes)
}

#[cfg(test)]
#[path = "embedding_routes_tests.rs"]
mod embedding_routes_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 保存通过校验后返回配置；异步持久化与反馈由宿主处理。
pub fn update(state: &mut EmbeddingRoutesSettingsState, message: EmbeddingRoutesMessage) -> Option<Vec<EmbeddingRouteConfig>> {
    match message {
        EmbeddingRoutesMessage::AddRoute => {
            state.routes.push(EmbeddingRouteDraft::default());
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::InsertAlibabaTemplate => {
            state.routes.push(alibaba_template_route());
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::InsertDoubaoTemplate => {
            state.routes.push(doubao_template_route());
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::InsertShijuTemplate => {
            state.routes.push(shiju_template_route());
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::RemoveRoute(index) => {
            if index < state.routes.len() {
                state.routes.remove(index);
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::PatternChanged(index, value) => {
            if let Some(route) = state.routes.get_mut(index) {
                route.pattern = value;
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::ProviderChanged(index, value) => {
            if let Some(route) = state.routes.get_mut(index) {
                route.provider = value;
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::ModelChanged(index, value) => {
            if let Some(route) = state.routes.get_mut(index) {
                route.model = value;
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::DimensionsChanged(index, value) => {
            if let Some(route) = state.routes.get_mut(index) {
                route.dimensions = value;
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::ApiKeyChanged(index, value) => {
            if let Some(route) = state.routes.get_mut(index) {
                route.api_key_input = value;
            }
            state.save_error = None;
            state.save_success = false;
        }
        EmbeddingRoutesMessage::Save => {
            return persist_embedding_routes_settings(state);
        }
    }

    None
}

fn trim_to_option(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
