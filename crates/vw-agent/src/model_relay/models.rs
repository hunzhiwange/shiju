//! Local discovery projection. Callers supply only the authorized saved catalog.
use serde_json::{Value, json};
use vw_api_types::{agent_apps::AppId, model_catalog::ModelCatalogEntry};

pub(crate) fn is_path(app: AppId, path: &str) -> bool {
    matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop)
        && (matches!(path, "/models" | "/v1/models")
            || app == AppId::ClaudeDesktop && path == "/claude-desktop/v1/models")
}

pub(crate) fn project(app: AppId, items: &[ModelCatalogEntry]) -> Value {
    let data: Vec<Value> = items.iter().map(|item| {
        let id = item.identity.model_id.as_str();
        let label = item.display_alias.as_deref().unwrap_or(id);
        if app == AppId::Codex {
            json!({"id":id,"object":"model","owned_by":"shiju"})
        } else {
            json!({"id":id,"type":"model","display_name":label})
        }
    }).collect();
    if app == AppId::Codex {
        json!({"object":"list","data":data})
    } else {
        json!({"data":data,"has_more":false,
            "first_id":items.first().map(|m|m.identity.model_id.as_str()),
            "last_id":items.last().map(|m|m.identity.model_id.as_str())})
    }
}
