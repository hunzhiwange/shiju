//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::update_embedding_routes_config_async;
use crate::app::{App, Message, state::EmbeddingRouteDraft};
use iced::Task;
use vw_config_types::routing::EmbeddingRouteConfig;

use super::messages::{EmbeddingRoutesMessage, SettingsMessage};

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

fn persist_embedding_routes_settings(app: &mut App) -> Option<Task<Message>> {
    let mut routes = Vec::with_capacity(app.embedding_routes_settings.routes.len());

    for (index, draft) in app.embedding_routes_settings.routes.iter().enumerate() {
        let pattern = draft.pattern.trim().to_string();
        let provider = draft.provider.trim().to_string();
        let model = draft.model.trim().to_string();

        if pattern.is_empty() {
            app.embedding_routes_settings.save_error =
                Some(format!("第 {} 条路由的 pattern 不能为空", index + 1));
            app.embedding_routes_settings.save_success = false;
            return None;
        }
        if provider.is_empty() {
            app.embedding_routes_settings.save_error =
                Some(format!("第 {} 条路由的 provider 不能为空", index + 1));
            app.embedding_routes_settings.save_success = false;
            return None;
        }
        if model.is_empty() {
            app.embedding_routes_settings.save_error =
                Some(format!("第 {} 条路由的 model 不能为空", index + 1));
            app.embedding_routes_settings.save_success = false;
            return None;
        }

        let dimensions = if draft.dimensions.trim().is_empty() {
            None
        } else {
            match draft.dimensions.trim().parse::<u32>() {
                Ok(value) if value > 0 => Some(value as usize),
                _ => {
                    app.embedding_routes_settings.save_error =
                        Some(format!("第 {} 条路由的 dimensions 必须是大于 0 的整数", index + 1));
                    app.embedding_routes_settings.save_success = false;
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

    app.embedding_routes_settings.save_error = None;
    app.embedding_routes_settings.save_success = true;
    Some(update_embedding_routes_config_async(move |cfg| {
        *cfg = routes;
    }))
}

#[cfg(test)]
#[path = "embedding_routes_tests.rs"]
mod embedding_routes_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::EmbeddingRoutes(route_message) => {
            match route_message {
                EmbeddingRoutesMessage::AddRoute => {
                    app.embedding_routes_settings.routes.push(EmbeddingRouteDraft::default());
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::InsertAlibabaTemplate => {
                    app.embedding_routes_settings.routes.push(alibaba_template_route());
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::InsertDoubaoTemplate => {
                    app.embedding_routes_settings.routes.push(doubao_template_route());
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::InsertShijuTemplate => {
                    app.embedding_routes_settings.routes.push(shiju_template_route());
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::RemoveRoute(index) => {
                    if index < app.embedding_routes_settings.routes.len() {
                        app.embedding_routes_settings.routes.remove(index);
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::PatternChanged(index, value) => {
                    if let Some(route) = app.embedding_routes_settings.routes.get_mut(index) {
                        route.pattern = value;
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::ProviderChanged(index, value) => {
                    if let Some(route) = app.embedding_routes_settings.routes.get_mut(index) {
                        route.provider = value;
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::ModelChanged(index, value) => {
                    if let Some(route) = app.embedding_routes_settings.routes.get_mut(index) {
                        route.model = value;
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::DimensionsChanged(index, value) => {
                    if let Some(route) = app.embedding_routes_settings.routes.get_mut(index) {
                        route.dimensions = value;
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::ApiKeyChanged(index, value) => {
                    if let Some(route) = app.embedding_routes_settings.routes.get_mut(index) {
                        route.api_key_input = value;
                    }
                    app.embedding_routes_settings.save_error = None;
                    app.embedding_routes_settings.save_success = false;
                }
                EmbeddingRoutesMessage::Save => {
                    return persist_embedding_routes_settings(app).unwrap_or_else(Task::none);
                }
            }

            Task::none()
        }
        _ => Task::none(),
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
