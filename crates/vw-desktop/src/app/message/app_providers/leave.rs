use super::{
    edit::{self, EditMessage},
    list::{self, ListMessage},
};
use crate::app::{
    App, Message,
    message::{SettingsMessage, ViewMessage},
};
use iced::{Element, Length, Task};

#[derive(Clone, Debug)]
pub enum Choice {
    Cancel,
    Discard,
    Save,
}

fn busy(app: &App) -> bool {
    app.app_providers.form.as_ref().is_some_and(|f| f.saving)
        || app.app_providers.editor.as_ref().is_some_and(|f| f.saving)
}

/// Runs before dispatch so cancelling never mutates the current page or host.
pub(crate) fn intercept(app: &mut App, message: &Message) -> bool {
    use EditMessage as E;
    use ListMessage as L;
    use SettingsMessage as S;
    let edits = matches!(
        message,
        Message::Settings(S::AppProviderEdit(
            E::Id(_)
                | E::Name(_)
                | E::Category(_)
                | E::Notes(_)
                | E::Website(_)
                | E::Key(_)
                | E::Replace(_)
                | E::Clear(_)
                | E::Claude(_)
                | E::Codex(_)
                | E::Gemini(_)
                | E::Grok(_)
                | E::Hermes(_)
                | E::Pi(_)
                | E::Kimi(_)
                | E::Mcode(_)
                | E::Dsh(_)
                | E::Omp(_)
                | E::OpenClaw(_)
                | E::OpenCode(_)
                | E::Agy(_)
        )) | Message::Settings(S::AppProviders(L::Sort(_) | L::Category(_) | L::Notes(_)))
    );
    if edits
        && !busy(app)
        && (app.app_providers.form.is_some() || app.app_providers.editor.is_some())
    {
        app.app_providers.leave.dirty = true;
    }
    let leaving = matches!(
        message,
        Message::Settings(S::AppProviders(L::Close | L::Open(_) | L::Cancel))
            | Message::Settings(S::AppProviderEdit(E::Cancel | E::Create | E::Open { .. }))
            | Message::Settings(S::TabSelected(_) | S::SystemTabSelected(_))
            | Message::Settings(S::AppsCatalog(
                crate::app::message::settings::AppsCatalogMessage::HostInput(_)
            ))
            | Message::View(
                ViewMessage::ToggleSystemSettings
                    | ViewMessage::OpenSystemSettingsTab(_)
                    | ViewMessage::OpenSystemSettingsModelDetail(_, _)
            )
    );
    if leaving && (app.app_providers.leave.dirty || busy(app)) {
        if app.app_providers.leave.pending.is_none() {
            app.app_providers.leave.pending = Some(Box::new(message.clone()));
        }
        return true;
    }
    false
}

pub(crate) fn choose(app: &mut App, choice: Choice) -> Task<Message> {
    if app.app_providers.leave.pending.is_none() {
        return Task::none();
    }
    match choice {
        Choice::Cancel => {
            app.app_providers.leave.pending = None;
            app.app_providers.leave.saving = false;
            Task::none()
        }
        Choice::Discard if !busy(app) => {
            let pending = app.app_providers.leave.pending.take();
            app.app_providers.form = None;
            app.app_providers.editor = None;
            app.app_providers.leave = Default::default();
            pending.map_or_else(Task::none, |message| Task::done(*message))
        }
        Choice::Save if !busy(app) => {
            app.app_providers.leave.saving = true;
            let task = if app.app_providers.form.is_some() {
                edit::update(app, EditMessage::Save)
            } else {
                list::update(app, ListMessage::Save)
            };
            // Validation errors do not start a request; keep the dialog and draft.
            if !busy(app) {
                app.app_providers.leave.saving = false;
            }
            task
        }
        _ => Task::none(),
    }
}

pub(crate) fn resume(app: &mut App) -> Task<Message> {
    if app.app_providers.form.is_none() && app.app_providers.editor.is_none() {
        app.app_providers.leave.dirty = false;
        if app.app_providers.leave.saving {
            app.app_providers.leave.saving = false;
            if let Some(message) = app.app_providers.leave.pending.take() {
                return Task::done(*message);
            }
        }
    } else if !busy(app) {
        app.app_providers.leave.saving = false;
    }
    Task::none()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    use iced::widget::{button, column, row, text};
    if app.app_providers.leave.pending.is_none() {
        return base;
    }
    let message = |choice| list::message(ListMessage::Leave(choice));
    let cancel = message(Choice::Cancel);
    let mut discard = button(text("放弃并离开")).padding([9, 14]).style(rounded_action_btn_style);
    let mut save = button(text("保存后离开")).padding([9, 14]).style(primary_action_btn_style);
    if !busy(app) {
        discard = discard.on_press(message(Choice::Discard));
        save = save.on_press(message(Choice::Save));
    }
    let card = settings_modal_card(
        column![
            row![text("未保存的修改").size(20), settings_close_button(cancel.clone())].spacing(16),
            text(if busy(app) {
                "正在保存，请等待结果。"
            } else {
                "离开前是否保存当前修改？"
            }),
            row![
                button(text("取消"))
                    .padding([9, 14])
                    .style(rounded_action_btn_style)
                    .on_press(cancel.clone()),
                discard,
                save
            ]
            .spacing(10),
        ]
        .spacing(18),
    )
    .width(Length::Fixed(480.0));
    settings_modal_overlay(Some(base), cancel, card)
}
