use crate::models::{ModelsLoaded, ModelStatusChange, collect_model_status_changes, collect_provider_model_status_changes, set_local_model_status, model_detail};
use desktop_i18n::settings_models::messages::Feedback;
#[derive(Debug, Clone)]
pub enum ModelMessage {
    ModelsRefresh,
    ModelsRefreshed(ModelsLoaded),
    ModelQueryChanged(String),
    ModelToggle(String, String, bool),
    ModelsSetAll(bool),
    ProviderModelsSetAll(String, bool),
    ProviderModelsReset(String),
    ProviderModelsResetCompleted(Vec<ModelStatusChange>, ModelsLoaded),
    ProviderModelsExpansionToggled(String),
    ModelStatusesSaved(Vec<ModelStatusChange>, Result<(), Feedback>),
    ModelDetailOpen(String, String),
    ModelDetailClose,
    ModelDetailToggleRaw,
}
pub enum ModelEffect {
    None,
    Refresh,
    QueryChanged,
    Save(Vec<ModelStatusChange>),
    Reset(String, Vec<ModelStatusChange>),
}
pub fn update(state: &mut crate::state::ModelSettingsState, message: ModelMessage) -> ModelEffect {
    match message {
        ModelMessage::ModelsRefresh => {
            if state.saving {
                return ModelEffect::None;
            }
            state.loading = true;
            state.save_error = None;
            state.detail_modal = None;
            ModelEffect::Refresh
        }
        ModelMessage::ModelsRefreshed(res) => match res {
            Ok(providers) => {
                state.loading = false;
                state.providers = providers;
                state.detail_modal = None;
                ModelEffect::None
            }
            Err(e) => {
                state.loading = false;
                state.save_error = Some((e).into());
                state.detail_modal = None;
                ModelEffect::None
            }
        },
        ModelMessage::ModelQueryChanged(v) => {
            let changed = state.query != v;
            state.query = v;
            if changed { ModelEffect::QueryChanged } else { ModelEffect::None }
        }
        ModelMessage::ModelToggle(provider_id, model_id, enabled) => {
            let change = state
                .providers
                .iter()
                .find(|provider| provider.id == provider_id)
                .and_then(|provider| provider.models.iter().find(|model| model.id == model_id))
                .filter(|model| model.enabled != enabled)
                .map(|model| ModelStatusChange {
                    provider_id,
                    model_id,
                    previous_enabled: model.enabled,
                    enabled,
                });
            ModelEffect::Save(change.into_iter().collect())
        }
        ModelMessage::ModelsSetAll(enabled) => {
            let changes = collect_model_status_changes(&state.providers, enabled);
            ModelEffect::Save(changes)
        }
        ModelMessage::ProviderModelsSetAll(provider_id, enabled) => {
            let changes = collect_provider_model_status_changes(
                &state.providers,
                &provider_id,
                enabled,
                false,
            );
            ModelEffect::Save(changes)
        }
        ModelMessage::ProviderModelsReset(provider_id) => {
            let changes = collect_provider_model_status_changes(
                &state.providers,
                &provider_id,
                false,
                true,
            );
            ModelEffect::Reset(provider_id, changes)
        }
        ModelMessage::ProviderModelsResetCompleted(changes, result) => {
            state.saving = false;
            match result {
                Ok(providers) => {
                    state.providers = providers;
                }
                Err(error) => {
                    for change in &changes {
                        set_local_model_status(
                            &mut state.providers,
                            change,
                            change.previous_enabled,
                        );
                    }
                    state.save_error = Some((error).into());
                }
            }
            ModelEffect::None
        }
        ModelMessage::ProviderModelsExpansionToggled(provider_id) => {
            if !state.expanded_provider_ids.insert(provider_id.clone()) {
                state.expanded_provider_ids.remove(&provider_id);
            }
            ModelEffect::None
        }
        ModelMessage::ModelStatusesSaved(changes, result) => {
            state.saving = false;
            if let Err(error) = result {
                for change in &changes {
                    set_local_model_status(
                        &mut state.providers,
                        change,
                        change.previous_enabled,
                    );
                }
                state.save_error = Some((error).into());
            }
            ModelEffect::None
        }
        ModelMessage::ModelDetailOpen(provider_id, model_id) => {
            let provider = state.providers.iter().find(|p| p.id == provider_id);
            let Some(provider) = provider else {
                state.save_error = Some(Feedback::owned("未找到提供商", vec![]));
                return ModelEffect::None;
            };
            let model = provider.models.iter().find(|m| m.id == model_id);
            let Some(model) = model else {
                state.save_error = Some(Feedback::owned("未找到模型", vec![]));
                return ModelEffect::None;
            };

            state.detail_modal = Some(
                model_detail(provider, model),
            );
            ModelEffect::None
        }
        ModelMessage::ModelDetailClose => {
            state.detail_modal = None;
            ModelEffect::None
        }
        ModelMessage::ModelDetailToggleRaw => {
            if let Some(m) = &mut state.detail_modal {
                m.show_raw = !m.show_raw;
            }
            ModelEffect::None
        }
    }
}
