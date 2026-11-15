//! Tenant application-provider records; no relationship to model-provider auth APIs.
use super::GatewayClient;
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_save(
        &self,
        app: AppId,
        id: &str,
        profile: &AppProviderWrite,
    ) -> Result<AppProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_save(app, id, profile).await
    }
    pub async fn app_provider_get(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<AppProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_get(app, id).await
    }
}

#[path = "app_providers/catalog.rs"]
mod catalog;


#[path = "app_providers/create.rs"]
mod create;

#[path = "app_providers/update.rs"]
mod update;

#[path = "app_providers/delete.rs"]
mod delete;

#[path = "app_providers/duplicate.rs"]
mod duplicate;

#[path = "app_providers/presentation.rs"]
mod presentation;

#[path = "app_providers/endpoints.rs"]
mod endpoints;

#[path = "app_providers/icons.rs"]
mod icons;

#[path = "app_providers/universal.rs"]
mod universal;

#[path = "app_providers/universal_sync.rs"]
mod universal_sync;

#[path = "app_providers/paths.rs"]
mod paths;

#[path = "app_providers/preview.rs"]
mod preview;


#[path = "app_providers/editor_merge.rs"]
mod editor_merge;

#[path = "app_providers/snippets.rs"]
mod snippets;

#[path = "app_providers/claude.rs"]
mod claude;

#[cfg(test)]
#[path = "app_providers/claude_login_tests.rs"]
mod claude_login_tests;


#[path = "app_providers/codex_catalog.rs"]
mod codex_catalog;
#[cfg(test)]
#[path = "app_providers/codex_catalog_tests.rs"]
mod codex_catalog_tests;

#[path = "app_providers/gemini.rs"]
mod gemini;

#[path = "app_providers/claude_desktop.rs"]
mod claude_desktop;
#[path = "app_providers/grok.rs"]
mod grok;

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

#[path = "app_providers/agy.rs"]
mod agy;

#[path = "app_providers/import.rs"]
mod import;

#[path = "app_providers/mode_state.rs"]
mod mode_state;

#[path = "app_providers/mode_apply.rs"]
mod mode_apply;

#[path = "app_providers/env_diagnose.rs"]
mod env_diagnose;

#[path = "app_providers/env_clean.rs"]
mod env_clean;

#[path = "app_providers/env_restore.rs"]
mod env_restore;

#[path = "app_providers/operations.rs"]
mod operations;

#[path = "app_providers/basic.rs"]
mod basic;

#[path = "app_providers/request_options.rs"]
mod request_options;
