use crate::app::App;
use desktop_chat_input::input_panel::usage as domain;
pub(super) fn input(app: &App) -> domain::Input<'_> {
 domain::Input { steps: &app.active_session_view_state.steps, usage: &app.usage,
 model: app.usage_panel.usage_model_info.as_ref().map(|info| domain::Model {
 input_limit: info.input_limit as u64, context_limit: info.context_limit as u64,
 cost_input_per_million: info.cost_input_per_million, cost_output_per_million: info.cost_output_per_million,
 limit_is_default: info.limit_is_default }) }
}
pub fn get_usage_rate_percent(app: &App) -> f32 { domain::get_usage_rate_percent(input(app)) }
pub fn get_usage_details(app: &App) -> (i64, i64, f64, i64) { domain::get_usage_details(input(app)) }
