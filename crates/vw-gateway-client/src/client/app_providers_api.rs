//! Tenant application-provider records; no relationship to model-provider auth APIs.
use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, AppProviderWrite},
};
impl GatewayClient {
    pub async fn app_provider_save(
        &self,
        app: AppId,
        id: &str,
        profile: &AppProviderWrite,
    ) -> Result<AppProviderSummary, String> {
        let path = provider_path(app, id)?;
        let request =
            self.client.put(format!("{}{}", self.endpoint.base_url(), path)).json(profile);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider save transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider save failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider save response".into())
    }
    pub async fn app_provider_get(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<AppProviderSummary, String> {
        self.get_json(&provider_path(app, id)?, &[]).await
    }
}
fn provider_path(app: AppId, id: &str) -> Result<String, String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid application provider identity".into());
    }
    Ok(format!("{}/{id}", provider_collection_path(app)))
}

#[path = "app_providers/catalog.rs"]
mod catalog;
#[cfg(test)]
#[path = "app_providers/catalog_tests.rs"]
mod catalog_tests;

fn provider_collection_path(app: AppId) -> String {
    let app = serde_json::to_string(&app).expect("AppId serialization");
    format!("/v1/app-providers/{}", app.trim_matches('"'))
}

#[path = "app_providers/create.rs"]
mod create;
#[cfg(test)]
#[path = "app_providers/create_tests.rs"]
mod create_tests;

#[path = "app_providers/update.rs"]
mod update;
#[cfg(test)]
#[path = "app_providers/update_tests.rs"]
mod update_tests;

#[path = "app_providers/delete.rs"]
mod delete;
#[cfg(test)]
#[path = "app_providers/delete_tests.rs"]
mod delete_tests;

#[path = "app_providers/duplicate.rs"]
mod duplicate;
#[cfg(test)]
#[path = "app_providers/duplicate_tests.rs"]
mod duplicate_tests;

#[path = "app_providers/presentation.rs"]
mod presentation;
#[cfg(test)]
#[path = "app_providers/presentation_tests.rs"]
mod presentation_tests;

#[path = "app_providers/endpoints.rs"]
mod endpoints;
#[cfg(test)]
#[path = "app_providers/endpoints_tests.rs"]
mod endpoints_tests;

#[path = "app_providers/icons.rs"]
mod icons;
#[cfg(test)]
#[path = "app_providers/icons_tests.rs"]
mod icons_tests;

#[path = "app_providers/universal.rs"]
mod universal;
#[cfg(test)]
#[path = "app_providers/universal_tests.rs"]
mod universal_tests;

#[path = "app_providers/universal_sync.rs"]
mod universal_sync;
#[cfg(test)]
#[path = "app_providers/universal_sync_tests.rs"]
mod universal_sync_tests;

#[path = "app_providers/paths.rs"]
mod paths;
#[cfg(test)]
#[path = "app_providers/paths_tests.rs"]
mod paths_tests;

#[path = "app_providers/preview.rs"]
mod preview;
#[cfg(test)]
#[path = "app_providers/preview_tests.rs"]
mod preview_tests;

#[cfg(test)]
#[path = "app_providers/validation_tests.rs"]
mod validation_tests;

#[path = "app_providers/editor_merge.rs"]
mod editor_merge;
#[cfg(test)]
#[path = "app_providers/editor_merge_tests.rs"]
mod editor_merge_tests;

#[path = "app_providers/snippets.rs"]
mod snippets;
#[cfg(all(test, target_os = "macos"))]
#[path = "app_providers/snippets_tests.rs"]
mod snippets_tests;

#[path = "app_providers/claude.rs"]
mod claude;
#[cfg(test)]
#[path = "app_providers/claude_tests.rs"]
mod claude_tests;

#[cfg(test)]
#[path = "app_providers/claude_login_tests.rs"]
mod claude_login_tests;

#[cfg(test)]
#[path = "app_providers/codex_tests.rs"]
mod codex_tests;

#[path = "app_providers/codex_catalog.rs"]
mod codex_catalog;
#[cfg(test)]
#[path = "app_providers/codex_catalog_tests.rs"]
mod codex_catalog_tests;

#[path = "app_providers/gemini.rs"]
mod gemini;
#[cfg(test)]
#[path = "app_providers/gemini_tests.rs"]
mod gemini_tests;

#[path = "app_providers/claude_desktop.rs"]
mod claude_desktop;
#[path = "app_providers/grok.rs"]
mod grok;
#[cfg(test)]
#[path = "app_providers/grok_tests.rs"]
mod grok_tests;

#[cfg(test)]
#[path = "app_providers/claude_desktop_tests.rs"]
mod claude_desktop_tests;

#[path = "app_providers/desktop_mapping.rs"]
mod desktop_mapping;
#[cfg(test)]
#[path = "app_providers/desktop_mapping_tests.rs"]
mod desktop_mapping_tests;

#[path = "app_providers/opencode.rs"]
mod opencode;
#[cfg(test)]
#[path = "app_providers/opencode_tests.rs"]
mod opencode_tests;

#[path = "app_providers/hermes.rs"]
mod hermes;
#[cfg(test)]
#[path = "app_providers/hermes_tests.rs"]
mod hermes_tests;
#[path = "app_providers/openclaw.rs"]
mod openclaw;
#[cfg(test)]
#[path = "app_providers/openclaw_tests.rs"]
mod openclaw_tests;

#[path = "app_providers/pi.rs"]
mod pi;
#[cfg(test)]
#[path = "app_providers/pi_tests.rs"]
mod pi_tests;

#[path = "app_providers/omp.rs"]
mod omp;
#[cfg(test)]
#[path = "app_providers/omp_tests.rs"]
mod omp_tests;

#[path = "app_providers/mcode.rs"]
mod mcode;
#[cfg(test)]
#[path = "app_providers/mcode_tests.rs"]
mod mcode_tests;

#[path = "app_providers/kimi.rs"]
mod kimi;
#[cfg(test)]
#[path = "app_providers/kimi_tests.rs"]
mod kimi_tests;

#[path = "app_providers/dsh.rs"]
mod dsh;
#[cfg(test)]
#[path = "app_providers/dsh_tests.rs"]
mod dsh_tests;
