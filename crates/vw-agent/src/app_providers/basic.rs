use super::*;
use vw_api_types::app_providers::{basic::*, duplicate::AppProviderDuplicate};
impl AppProviderStore {
    pub fn basic(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<ProviderBasic, String> {
        vw_agent_apps::providers::basic::read(&self.basic_record(tenant, app, id)?)
    }
    fn basic_record(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<serde_json::Value, String> {
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        serde_json::from_str(&raw).map_err(|_| "invalid stored application provider profile".into())
    }
    pub fn write_basic(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &ProviderBasicWrite,
    ) -> Result<ProviderBasic, String> {
        let key = Self::key(app, id)?;
        let insert = |stored: &serde_json::Value| -> Result<bool, String> {
            let updated = vw_agent_apps::providers::basic::apply(
                app,
                stored,
                &request.fields,
                &request.api_key,
            )?;
            self.settings
                .insert_if_absent(tenant, NAMESPACE, &key, &updated.to_string())
                .map_err(|_| "application provider storage unavailable".into())
        };
        match &request.action {
            BasicOperation::Create { draft } => {
                let mut draft = draft.clone();
                draft.name = request.fields.name.clone();
                // The shell's explicit intent is authoritative; never replay a draft's key.
                draft.api_key = request.api_key.clone();
                vw_agent_apps::providers::create::create(app, id, &draft, |profile| {
                    let value = serde_json::to_value(profile)
                        .map_err(|_| "invalid application provider profile")?;
                    // Apply basic fields and reject credential placeholders before insertion.
                    let value = vw_agent_apps::providers::basic::apply(
                        app,
                        &value,
                        &request.fields,
                        &request.api_key,
                    )?;
                    self.settings
                        .insert_if_absent(tenant, NAMESPACE, &key, &value.to_string())
                        .map_err(|_| "application provider storage unavailable".into())
                })?;
            }
            BasicOperation::Copy { source_id } => {
                let stored = self.basic_record(tenant, app, source_id)?;
                vw_agent_apps::providers::duplicate::duplicate(
                    app,
                    source_id,
                    &stored,
                    &AppProviderDuplicate { new_id: id.into(), name: request.fields.name.clone() },
                    insert,
                )?;
            }
            BasicOperation::Update => {
                let before = self
                    .settings
                    .get(tenant, NAMESPACE, &key)
                    .map_err(|_| "application provider storage unavailable")?
                    .ok_or("application provider not found")?;
                let stored = serde_json::from_str(&before)
                    .map_err(|_| "invalid stored application provider profile")?;
                let updated = vw_agent_apps::providers::basic::apply(
                    app,
                    &stored,
                    &request.fields,
                    &request.api_key,
                )?;
                if !self
                    .settings
                    .compare_exchange(tenant, NAMESPACE, &key, &before, &updated.to_string())
                    .map_err(|_| "application provider storage unavailable")?
                {
                    return Err("application provider changed during update".into());
                }
            }
        }
        self.basic(tenant, app, id)
    }
}
