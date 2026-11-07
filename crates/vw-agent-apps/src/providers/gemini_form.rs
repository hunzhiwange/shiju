//! Safe catalog projection and write-only credentials for the Gemini editor.
use serde_json::Value;
use vw_api_types::app_providers::{create::ProviderSecretWrite, gemini::*};

fn stored_profile(row: &Value) -> Result<Option<GeminiProfile>, String> {
    row.get("gemini")
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|_| "invalid Gemini profile".into())
        })
        .transpose()
}

pub fn view(row: &Value, managed: bool) -> Result<GeminiFormView, String> {
    let fields = super::super::basic::read(row)?;
    let profile = if managed { None } else { stored_profile(row)? };
    let (config, model) = match profile {
        Some(GeminiProfile { config: GeminiConfig::Official {}, model, .. }) => {
            (GeminiFormKind::Official {}, model)
        }
        Some(GeminiProfile {
            config: GeminiConfig::ThirdParty { base_url, .. }, model, ..
        }) => {
            super::super::validation::endpoint(&base_url)?;
            (GeminiFormKind::ThirdParty { base_url }, model)
        }
        None => (GeminiFormKind::Unsupported {}, None),
    };
    Ok(GeminiFormView { fields, config, model })
}

pub fn profile(request: &GeminiFormWrite, row: Option<&Value>) -> Result<GeminiProfile, String> {
    let old = match row {
        Some(row) => Some(stored_profile(row)?.ok_or("unsupported Gemini form profile")?.config),
        None => None,
    };
    let config = match &request.config {
        GeminiFormConfig::Official {} => GeminiConfig::Official {},
        GeminiFormConfig::ThirdParty { base_url, api_key } => {
            let api_key = match (api_key, old) {
                (ProviderSecretWrite::Replace { value }, _) => value.clone(),
                (
                    ProviderSecretWrite::Keep,
                    Some(GeminiConfig::ThirdParty { base_url: previous, api_key }),
                ) if previous == *base_url => api_key,
                _ => return Err("third-party Gemini requires a key for this endpoint".into()),
            };
            GeminiConfig::ThirdParty { base_url: base_url.clone(), api_key }
        }
    };
    Ok(GeminiProfile { name: request.fields.name.clone(), config, model: request.model.clone() })
}
