use super::super::system_settings_common::*;
use crate::app::message::app_providers::list::{ListMessage, message};
use crate::app::{Message, i18n::I18n, state::app_providers::list::ListState};
use iced::{Element, Theme};
#[derive(Clone, Copy)]
pub(crate) struct Help<'a> {
    i18n: I18n,
    theme: &'a Theme,
    #[cfg(not(target_arch = "wasm32"))]
    usage_app: Option<&'a crate::app::App>,
}

impl<'a> Help<'a> {
    pub(crate) fn provider_help(self) -> desktop_provider_apps::components::help::Help<'a> {
        desktop_provider_apps::components::help::Help {
            i18n: self.i18n,
            theme: self.theme,
            render: super::edit::render_help,
        }
    }

    pub(crate) fn new(i18n: I18n, theme: &'a Theme) -> Self {
        Self {
            i18n,
            theme,
            #[cfg(not(target_arch = "wasm32"))]
            usage_app: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn with_usage(mut self, app: &'a crate::app::App) -> Self {
        self.usage_app = Some(app);
        self
    }
}

pub(crate) fn overlay<'a>(
    state: &'a ListState,
    help: Help<'a>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(scope) = &state.scope else {
        return base;
    };
    let mut supplements: Vec<(&'a str, Option<Element<'a, Message>>, Element<'a, Message>)> =
        Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(app) = help.usage_app {
        let usage_scope = super::super::apps_usage_cards::current_scope(app, false);
        if let Some(snapshot) = &state.snapshot {
            for entry in &snapshot.catalog.rows {
                let id = &entry.provider.id;
                let health = crate::app::message::agent_apps_health::selected(app, id)
                    .then(|| super::super::agent_apps_high_availability::health::view(app));
                let usage = super::super::apps_usage_cards::provider_view(
                    usage_scope.as_ref(),
                    &state.usage,
                    scope.app,
                    id,
                );
                supplements.push((id.as_str(), health, usage));
            }
        }
    }
    // Host-owned usage and health elements keep their messages while provider card messages map locally.
    let card = desktop_provider_apps::components::list::card(
        map,
        desktop_provider_apps::components::list::View {
            scope: state.scope.as_ref(),
            query: &state.query,
            loading: state.loading,
            error: state.error.as_ref(),
            snapshot: state.snapshot.as_ref(),
            editing: state.form.is_some() || state.editor.is_some(),
        },
        supplements,
    );
    let list = settings_modal_overlay(Some(base), message(ListMessage::Close), card);
    if let Some(form) = &state.omo {
        return crate::app::components::aggregation_omo::overlay(form, list);
    }
    if let Some(form) = &state.codex_subagent {
        return crate::app::components::aggregation_subagent::overlay(form, list);
    }
    if let Some(form) = &state.pi_directory {
        return crate::app::components::aggregation_pi::overlay(form, list);
    }
    if let Some(form) = &state.omp_directory {
        return crate::app::components::aggregation_omp::overlay(form, list);
    }
    if let Some(form) = &state.model_catalog {
        return crate::app::components::system_settings_providers::catalog::deployments(
            form,
            state.generation,
            help.i18n,
            help.theme,
            list,
        );
    }
    if let Some(form) = &state.aggregation {
        return super::super::aggregation_list::overlay(form, list);
    }
    if let Some((close, card)) =
        desktop_provider_apps::components::list::local_card(state, help.provider_help())
    {
        return settings_modal_overlay(
            Some(list),
            super::edit::map(close),
            card.map(super::edit::map),
        );
    }
    if let Some(form) = &state.desktop {
        return super::desktop::overlay(form, help, list);
    }
    if let Some(speed) = &state.endpoint_speed {
        return crate::app::components::agent_apps_diagnostics::endpoints_modal::overlay(
            speed, list,
        );
    }
    if let Some(form) = &state.form {
        #[cfg(not(target_arch = "wasm32"))]
        let binding =
            crate::app::message::agent_apps_auth::binding::entry(form, scope.app, state.generation);
        #[cfg(target_arch = "wasm32")]
        let binding = None;
        return super::edit::overlay_with_binding(form, help, list, binding);
    }
    if let Some(form) = &state.editor {
        let card = desktop_provider_apps::components::list::editor_card(
            &form.sort,
            &form.category,
            &form.notes,
            form.saving,
        )
        .map(map_editor);
        return settings_modal_overlay(Some(list), message(ListMessage::Cancel), card);
    }
    list
}

pub(super) fn map(value: desktop_provider_apps::components::list::ListMessage) -> Message {
    use crate::app::message::{app_providers as events, settings};
    use desktop_provider_apps::components::list::ListMessage as V;
    message(match value {
        V::Create => return events::edit::message(events::edit::EditMessage::Create),
        V::OpenEdit { id, copy, delete } => {
            return events::edit::message(events::edit::EditMessage::Open { id, copy, delete });
        }
        V::Close => ListMessage::Close,
        V::Refresh => ListMessage::Refresh,
        V::Universal => ListMessage::Universal(events::universal::Event::Open),
        V::Snippets => ListMessage::Snippets(events::snippets::Event::Open),
        V::Env => ListMessage::Env(events::env::Event::Open),
        V::Aggregation => ListMessage::Aggregation(settings::aggregation_list::Event::Open),
        V::Failover => {
            ListMessage::Failover(crate::app::message::agent_apps_high_availability::Event::Open)
        }
        V::Mode => ListMessage::Mode(events::mode::Event::Open),
        V::Paths => ListMessage::Paths(events::paths::Event::Open),
        V::CodexSubagent => ListMessage::CodexSubagent(settings::aggregation_subagent::Event::Open),
        V::PiDirectory => ListMessage::PiDirectory(settings::aggregation_pi::Event::Open),
        V::OmpDirectory => ListMessage::OmpDirectory(settings::aggregation_omp::Event::Open),
        V::QueryInput(value) => ListMessage::QueryInput(value),
        V::Edit(value) => ListMessage::Edit(value),
        V::Endpoints(value) => ListMessage::Endpoints(events::endpoints::Event::Open(value)),
        V::ModelDiagnostics(value) => ListMessage::ModelDiagnostics(
            crate::app::message::agent_apps_diagnostics::model::Event::Open(value),
        ),
        V::FailoverHealth(value) => {
            ListMessage::FailoverHealth(crate::app::message::agent_apps_health::Event::Open(value))
        }
        V::Preview(value) => ListMessage::Preview(events::preview::Event::Open(value)),
        V::DesktopAggregation(value) => {
            ListMessage::DesktopAggregation(settings::aggregation_desktop::Event::Open(value))
        }
        V::Desktop(value) => ListMessage::Desktop(events::desktop::Event::Open(value)),
        V::Icons(value) => ListMessage::Icons(events::icons::Event::Open(value)),
        V::ModelCatalog(value) => {
            ListMessage::ModelCatalog(settings::providers::catalog_api::Event::Open(value))
        }
        V::HermesMember { id, add } => ListMessage::HermesMember { id, add },
        V::PiMember { id, add } => ListMessage::PiMember { id, add },
        V::KimiMember { id, add } => ListMessage::KimiMember { id, add },
        V::McodeMember { id, add } => ListMessage::McodeMember { id, add },
        V::DshMember { id, add } => ListMessage::DshMember { id, add },
        V::OmpMember { id, add } => ListMessage::OmpMember { id, add },
        V::OpenClawMember { id, add } => ListMessage::OpenClawMember { id, add },
        V::OpenCodeMember { id, add } => ListMessage::OpenCodeMember { id, add },
        V::Omo(slim) => ListMessage::Omo(settings::aggregation_omo::Event::Open(if slim {
            crate::app::state::aggregation_omo::Variant::Slim
        } else {
            crate::app::state::aggregation_omo::Variant::Standard
        })),
    })
}
fn map_editor(value: desktop_provider_apps::Message) -> Message {
    super::edit::map(value)
}
