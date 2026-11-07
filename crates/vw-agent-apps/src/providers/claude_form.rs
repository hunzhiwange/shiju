//! Bridge the write-only desktop form to the native Claude profile.
use serde_json::Value;
use vw_api_types::app_providers::{claude::*, create::ProviderSecretWrite};

fn stored_profile(row: &Value) -> Result<Option<ClaudeProfile>, String> {
    row.get("claude")
        .map(|profile| {
            serde_json::from_value(profile.clone())
                .map_err(|_| "invalid stored application provider profile".into())
        })
        .transpose()
}

pub fn view(row: &Value) -> Result<ClaudeFormView, String> {
    let fields = super::super::basic::read(row)?;
    let (config, model) = match stored_profile(row)? {
        Some(ClaudeProfile { config: ClaudeConfig::Official { account: None }, model, .. }) => {
            (ClaudeFormKind::Official {}, model)
        }
        Some(ClaudeProfile {
            config: ClaudeConfig::ThirdParty { base_url, api_key_header, .. },
            model,
            ..
        }) => {
            super::super::validation::endpoint(&base_url)?;
            (ClaudeFormKind::ThirdParty { base_url, api_key_header }, model)
        }
        _ => (ClaudeFormKind::Unsupported {}, None),
    };
    Ok(ClaudeFormView { fields, config, model })
}

pub fn profile(request: &ClaudeFormWrite, row: Option<&Value>) -> Result<ClaudeProfile, String> {
    let old = match row {
        None => None,
        Some(row) => match stored_profile(row)? {
            Some(
                profile @ ClaudeProfile {
                    config:
                        ClaudeConfig::Official { account: None } | ClaudeConfig::ThirdParty { .. },
                    ..
                },
            ) => Some(profile.config),
            _ => return Err("unsupported Claude form profile".into()),
        },
    };
    let config = match &request.config {
        ClaudeFormConfig::Official {} => ClaudeConfig::Official { account: None },
        ClaudeFormConfig::ThirdParty { base_url, api_key_header, api_key } => {
            let api_key = match (api_key, old) {
                (ProviderSecretWrite::Replace { value }, _) => value.clone(),
                // A key saved for one upstream must not be silently sent to another.
                (
                    ProviderSecretWrite::Keep,
                    Some(ClaudeConfig::ThirdParty { base_url: previous, api_key, .. }),
                ) if previous == *base_url => api_key,
                _ => return Err("third-party Claude requires a key for this endpoint".into()),
            };
            ClaudeConfig::ThirdParty {
                base_url: base_url.clone(),
                api_key,
                api_key_header: *api_key_header,
            }
        }
    };
    let profile =
        ClaudeProfile { name: request.fields.name.clone(), config, model: request.model.clone() };
    super::settings(&profile).map_err(|_| "invalid Claude form configuration")?;
    Ok(profile)
}
