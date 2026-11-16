//! Chat-input popover decisions; loading and persistence are explicit host effects.
use crate::components::model_hover_tooltip::ModelPopoverHover;
use desktop_chat_input::popovers::{self, Popover, Visibility};
use std::collections::BTreeSet;
#[derive(Clone)]
pub enum Message {
    Toggle(Popover),
    Close(Popover),
    CloseAll,
    HoverChanged(Option<ModelPopoverHover>),
    ToggleProvider(String),
}
pub struct Context<'a> {
    pub visibility: &'a mut Visibility,
    pub hover: &'a mut Option<ModelPopoverHover>,
    pub collapsed_providers: &'a mut BTreeSet<String>,
}
#[derive(Default, Clone, Copy)]
pub struct Inputs {
    pub requesting: bool,
    pub models_empty: bool,
    pub models_loading: bool,
    pub acp_agents_empty: bool,
}
pub enum Effect {
    RefreshModelSources { load_models: bool },
    LoadAcpAgents,
    SaveCollapsedProviders(serde_json::Value),
}
pub fn update(app: Context<'_>, inputs: Inputs, message: Message) -> Option<Effect> {
    match message {
        Message::ToggleProvider(id) => {
            if !app.collapsed_providers.insert(id.clone()) {
                app.collapsed_providers.remove(&id);
            }
            *app.hover = None;
            return Some(Effect::SaveCollapsedProviders(serde_json::Value::Array(
                app.collapsed_providers.iter().cloned().map(serde_json::Value::String).collect(),
            )));
        }
        Message::Toggle(Popover::SendMode) if !inputs.requesting => {
            app.visibility.send_mode = false
        }
        Message::Toggle(target) => {
            *app.visibility = popovers::toggle(*app.visibility, target);
            match target {
                Popover::Model if app.visibility.model => {
                    return Some(Effect::RefreshModelSources {
                        load_models: !inputs.models_loading && inputs.models_empty,
                    });
                }
                Popover::Acp if app.visibility.acp && inputs.acp_agents_empty => {
                    return Some(Effect::LoadAcpAgents);
                }
                _ => {}
            }
        }
        Message::Close(target) => {
            *app.visibility = popovers::close(*app.visibility, target);
            if matches!(target, Popover::Model) {
                *app.hover = None;
            }
        }
        Message::CloseAll => {
            *app.visibility = Visibility::default();
            *app.hover = None;
        }
        Message::HoverChanged(hover) => *app.hover = hover,
    }
    None
}
