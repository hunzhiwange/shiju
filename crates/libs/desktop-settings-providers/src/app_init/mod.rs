pub mod settings;
pub mod state;

pub fn apply_input_preferences(state: &mut crate::state::ModelSettingsState, cfg: &serde_json::Value) {
        if let Some(ids) = cfg.get("input_model_collapsed_provider_ids").and_then(|v| v.as_array())
        {
            state.input_collapsed_provider_ids =
                ids.iter().filter_map(|id| id.as_str().map(str::to_owned)).collect();
        }
}
