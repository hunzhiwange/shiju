//! Populate existing application forms from the gateway's public catalog projection.
use super::{Availability, Preset, desktop, edit, entries, fill_codex, fill_grok};
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::{ModelDevPresetModel, ModelDevProviderPreset};
use vw_api_app_provider_profiles::codex_catalog::CodexModelMapping;

pub fn remove_native_duplicates(app: AppId, rows: &mut Vec<ModelDevProviderPreset>) {
    let native = entries(app);
    rows.retain(|row| {
        !native.iter().any(|entry| {
            let preset = entry.preset;
            preset.availability() == Availability::Direct
                && match preset {
                    Preset::Claude(source) if app == AppId::Claude => {
                        source.provider().api_key_header == row.api_key_header
                    }
                    _ => true,
                }
                && preset.endpoint().is_some_and(|base| {
                    base.trim_end_matches('/') == row.base_url.trim_end_matches('/')
                })
        })
    });
}

pub fn fill_desktop(form: &mut desktop::Form, preset: &ModelDevProviderPreset) {
    form.name.clone_from(&preset.name);
    form.base_url.clone_from(&preset.base_url);
}

pub fn fill_editor(
    form: &mut edit::EditForm,
    preset: &ModelDevProviderPreset,
    model: &ModelDevPresetModel,
) {
    form.fields.name.clone_from(&preset.name);
    if let Some(draft) = &mut form.claude {
        draft.kind = crate::state::claude::Kind::ThirdParty;
        draft.base_url.clone_from(&preset.base_url);
        draft.api_key_header = preset.api_key_header;
        draft.model.clone_from(&model.id);
    } else if let Some(draft) = &mut form.codex {
        fill_codex(
            draft,
            &preset.base_url,
            Some(&model.id),
            None,
            None,
            vec![CodexModelMapping { slug: model.id.clone(), display_name: model.name.clone() }],
        );
    } else if let Some(draft) = &mut form.gemini {
        draft.kind = crate::state::gemini::Kind::ThirdParty;
        draft.base_url.clone_from(&preset.base_url);
        draft.model.clone_from(&model.id);
    } else if let Some(draft) = &mut form.grok
        && let Some(context) = model.context_window.and_then(|value| i64::try_from(value).ok())
    {
        fill_grok(draft, &preset.base_url, &preset.name, &model.id, context);
    }
}
