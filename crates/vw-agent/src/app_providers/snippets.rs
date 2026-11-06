//! A single per-application snippet row in the existing tenant settings store.
use super::*;
use vw_agent_apps::providers::snippets;
use vw_api_types::app_providers::snippets::ConfigSnippet;
const SNIPPETS: &str = "app_providers.snippets";
impl AppProviderStore {
    pub fn snippet_get(&self, tenant: &TenantScope, app: AppId) -> Result<ConfigSnippet, String> {
        let key = serde_json::to_string(&app).expect("AppId serialization");
        let raw = self
            .settings
            .get(tenant, SNIPPETS, &key)
            .map_err(|_| "application provider storage unavailable")?;
        match raw {
            Some(raw) => {
                let stored: ConfigSnippet = serde_json::from_str(&raw)
                    .map_err(|_| "invalid stored application provider profile")?;
                snippets::normalize(app, stored.content.as_deref().unwrap_or_default())
                    .map_err(|_| "invalid stored application provider profile".into())
            }
            None => snippets::normalize(app, ""),
        }
    }
    pub fn snippet_save(
        &self,
        tenant: &TenantScope,
        app: AppId,
        content: &str,
    ) -> Result<ConfigSnippet, String> {
        let snippet = snippets::normalize(app, content)?;
        let key = serde_json::to_string(&app).expect("AppId serialization");
        self.settings
            .upsert(
                tenant,
                SNIPPETS,
                &key,
                &serde_json::to_string(&snippet).map_err(|_| "invalid snippet")?,
            )
            .map_err(|_| "application provider storage unavailable")?;
        Ok(snippet)
    }
    pub fn snippet_extract_saved(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<ConfigSnippet, String> {
        let record = Self::preview_record(&self.database, tenant, app, id)?;
        snippets::extract(app, record.get("settingsConfig").ok_or("invalid snippet")?)
    }
}
