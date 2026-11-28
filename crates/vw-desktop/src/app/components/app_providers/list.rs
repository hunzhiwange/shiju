use super::super::system_settings_common::*;
use crate::app::message::app_providers::list::{ListMessage, message};
use crate::app::{Message, i18n::I18n, state::app_providers::list::ListState};
use desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape;
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

fn supplements<'a>(
    state: &'a ListState,
    help: Help<'a>,
    app_id: vw_api_app_identity::AppId,
) -> Vec<(&'a str, Option<Element<'a, Message>>, Element<'a, Message>)> {
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
                let usage = super::super::apps_usage_cards::provider_view(help.i18n,
                    usage_scope.as_ref(),
                    &state.usage,
                    app_id,
                    id,
                );
                supplements.push((id.as_str(), health, usage));
            }
        }
    }
    supplements
}

fn list_state(state: &ListState) -> desktop_provider_apps::components::list::View<'_> {
    desktop_provider_apps::components::list::View {
        scope: state.scope.as_ref(),
        query: &state.query,
        loading: state.loading,
        error: state.error.as_ref(),
        snapshot: state.snapshot.as_ref(),
        editing: state.form.is_some() || state.editor.is_some() || state.tray_switch.is_some(),
    }
}

pub(crate) fn page<'a>(
    state: &'a ListState,
    help: Help<'a>,
    app_id: vw_api_app_identity::AppId,
    title: Element<'a, Message>,
    page: desktop_provider_apps::components::list::Page<'a, Message>,
) -> Element<'a, Message> {
    desktop_provider_apps::components::list::page(help.i18n,
        map,
        list_state(state),
        supplements(state, help, app_id),
        title,
        page,
    )
}

pub(crate) fn overlay<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a ListState,
    help: Help<'a>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(scope) = &state.scope else {
        return base;
    };
    if let Some(picker) = &state.preset_picker {
        use desktop_provider_apps::presets::Event;
        let card = desktop_provider_apps::components::presets::card(i18n, scope.app, picker, |id| {
            state.snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.catalog.rows.iter().any(|row| row.provider.id == id)
            })
        });
        return settings_modal_overlay_with_escape(
            base,
            message(ListMessage::Presets(if picker.family.is_some() || picker.catalog_provider.is_some() {
                Event::BackToCatalog
            } else {
                Event::Close
            })),
            card.map(|event| message(ListMessage::Presets(event))),
        );
    }
    if let Some(form) = &state.omo {
        return crate::app::components::aggregation_omo::overlay(i18n, form, base);
    }
    if let Some(form) = &state.pi_directory {
        return crate::app::components::aggregation_pi::overlay(i18n, form, base);
    }
    if let Some(form) = &state.omp_directory {
        return crate::app::components::aggregation_omp::overlay(i18n, form, base);
    }
    if let Some(form) = &state.model_catalog {
        return crate::app::components::system_settings_providers::catalog::deployments(
            form,
            state.generation,
            help.i18n,
            help.theme,
            base,
        );
    }
    if let Some((close, card)) =
        desktop_provider_apps::components::list::local_card(state, help.provider_help())
    {
        return settings_modal_overlay(
            Some(base),
            super::edit::map(close),
            card.map(super::edit::map),
        );
    }
    if let Some(speed) = &state.endpoint_speed {
        return crate::app::components::agent_apps_diagnostics::endpoints_modal::overlay(i18n,
            speed, base,
        );
    }
    if let Some(form) = &state.form {
        #[cfg(not(target_arch = "wasm32"))]
        let binding =
            crate::app::message::agent_apps_auth::binding::entry(form, scope.app, state.generation);
        #[cfg(target_arch = "wasm32")]
        let binding = None;
        let current = state.snapshot.as_ref().is_some_and(|snapshot| {
            matches!(&snapshot.catalog.in_use,
                vw_api_app_provider_edit::catalog::ProviderSelection::Selected { id } if *id == form.id)
        });
        return super::edit::overlay_with_binding(form, help, base, binding, current);
    }
    if let Some(form) = &state.editor {
        let card = desktop_provider_apps::components::list::editor_card(i18n,
            &form.sort,
            &form.category,
            &form.notes,
            form.saving,
        )
        .map(map_editor);
        return settings_modal_overlay(Some(base), message(ListMessage::Cancel), card);
    }
    base
}

pub(crate) fn map(value: desktop_provider_apps::components::list::ListMessage) -> Message {
    use crate::app::message::{app_providers as events, settings};
    use desktop_provider_apps::components::list::ListMessage as V;
    message(match value {
        V::Presets(event) => ListMessage::Presets(event),
        V::Create => return events::edit::message(events::edit::EditMessage::Create),
        V::OpenEdit { id, copy, delete } => {
            return events::edit::message(events::edit::EditMessage::Open { id, copy, delete });
        }
        V::Terminal(id) => ListMessage::Terminal(id),
        V::UseProvider(id) => ListMessage::UseProvider(id),
        #[cfg(not(target_arch = "wasm32"))]
        V::UsageSettings(app, id) => {
            ListMessage::UsageSettings(super::super::apps_usage_settings::Event::Open(app, id))
        }
        V::Refresh => ListMessage::Refresh,
        V::Universal => ListMessage::Universal(events::universal::Event::Open),
        V::Snippets => ListMessage::Snippets(events::snippets::Event::Open),
        V::Failover => {
            ListMessage::Failover(crate::app::message::agent_apps_high_availability::Event::Open)
        }
        V::Paths => ListMessage::Paths(events::paths::Event::Open),
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
