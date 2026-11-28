//! Session-local history for pages in the main application shell.

use super::{
    App, Message, Screen,
    message::{ViewMessage, view},
};
use desktop_settings_shell::{SystemTab, search::SystemSettingsSearchTarget as SettingsTarget};
use iced::{Element, Task, window};

const HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy)]
pub enum HistoryDirection {
    Back,
    Forward,
}

#[derive(Clone, PartialEq, Eq)]
enum Page {
    Home,
    Tab(Screen, String),
    Settings(SettingsTarget),
}

impl Page {
    fn current(app: &App) -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        if app.startup_state != super::state::NativeStartupState::Ready {
            return None;
        }

        if app.navigation_panels.show_system_settings {
            let target = match app.navigation_panels.system_settings_tab {
                SystemTab::General => {
                    SettingsTarget::General(app.general_settings.general_settings_tab)
                }
                SystemTab::Models => SettingsTarget::Models(app.model_settings.active_tab),
                SystemTab::Agents => SettingsTarget::Agents(app.agents_settings.active_config_tab),
                SystemTab::Memory => SettingsTarget::Memory(app.memory_settings.active_tab),
                SystemTab::Autonomy => SettingsTarget::Autonomy(app.autonomy_settings.active_tab),
                SystemTab::Security => SettingsTarget::Security(app.security_settings.active_tab),
                SystemTab::Gateway => SettingsTarget::Gateway(app.gateway_settings.active_tab),
                tab => SettingsTarget::TopLevel(tab),
            };
            return Some(Self::Settings(target));
        }

        let tabs = &app.navigation_tabs;
        if tabs.screen == Screen::Home {
            Some(Self::Home)
        } else {
            let id = tabs.active_tab_id.as_deref()?;
            tabs.open_tabs
                .iter()
                .any(|tab| tab.id == id && tab.screen == tabs.screen)
                .then(|| Self::Tab(tabs.screen, id.to_owned()))
        }
    }

    fn message(&self, app: &App) -> Option<ViewMessage> {
        Some(match self {
            Self::Home => ViewMessage::GoHome,
            Self::Settings(target) => ViewMessage::OpenSystemSettingsTarget(*target),
            Self::Tab(screen, id) => {
                if app
                    .navigation_tabs
                    .open_tabs
                    .iter()
                    .any(|tab| tab.id == *id && tab.screen == *screen)
                {
                    ViewMessage::TabSelected(id.clone())
                } else if let Screen::AgentManagement(page) = *screen {
                    ViewMessage::OpenAgentManagement(Some(page))
                } else if *screen == Screen::Apps {
                    // Opening a project automatically removes the Apps tab.
                    ViewMessage::OpenApps
                } else {
                    return None;
                }
            }
        })
    }
}

#[derive(Default)]
pub(crate) struct State {
    pages: Vec<Page>,
    index: usize,
}

impl State {
    fn destination(&self, app: &App, direction: HistoryDirection) -> Option<(usize, ViewMessage)> {
        let visit = |index: usize| {
            let page = &self.pages[index];
            if self.pages.get(self.index) == Some(page) {
                return None;
            }
            page.message(app).map(|message| (index, message))
        };
        match direction {
            HistoryDirection::Back => (0..self.index).rev().find_map(visit),
            HistoryDirection::Forward => (self.index + 1..self.pages.len()).find_map(visit),
        }
    }
}

pub(crate) fn initialize(app: &mut App) {
    if app.page_history.pages.is_empty() {
        record(app);
    }
}

pub(crate) fn record(app: &mut App) {
    let Some(page) = Page::current(app) else {
        return;
    };
    let history = &mut app.page_history;
    if history.pages.get(history.index) == Some(&page) {
        return;
    }
    history.pages.truncate(history.index + 1);
    if history.pages.len() == HISTORY_LIMIT {
        history.pages.remove(0);
    }
    history.pages.push(page);
    history.index = history.pages.len() - 1;
}

pub(crate) fn can_navigate(
    app: &App,
    direction: HistoryDirection,
    window: Option<window::Id>,
) -> bool {
    window.is_none_or(|id| app.main_window_id == Some(id))
        && !app.shutdown_requested
        && !app.mobile_layout.drawer_open
        && !app.mobile_layout.mine_open
        && !app.navigation_panels.show_about_modal
        && !app.update_panel.show_cli_install_modal
        && app.app_providers.leave.pending.is_none()
        && app.session_state.question_modal_request.is_none()
        && app.session_state.permission_modal_request.is_none()
        && app.workspace_state.new_session_picker_project.is_none()
        && app.session_state.chat_fork_dialog.is_none()
        && app.session_state.session_rename_id.is_none()
        && !app.cron_settings.add_draft.submitting
        && !app.cron_settings.edit_draft.submitting
        && (!app.navigation_panels.show_system_settings
            || app.navigation_panels.system_settings_help_tab.is_none())
        && !app.search.show_search_overlay
        && app.page_history.destination(app, direction).is_some()
}

pub(crate) fn navigate(app: &mut App, direction: HistoryDirection) -> Task<Message> {
    let Some((index, message)) = app.page_history.destination(app, direction) else {
        return Task::none();
    };
    app.navigation_panels.close_menu();
    let leaving_settings = app.navigation_panels.show_system_settings
        && !matches!(app.page_history.pages[index], Page::Settings(_));
    let close_settings = if leaving_settings {
        view::update(app, ViewMessage::ToggleSystemSettings)
    } else {
        Task::none()
    };
    let select_page = if Page::current(app).as_ref() == app.page_history.pages.get(index) {
        Task::none()
    } else {
        view::update(app, message)
    };
    if Page::current(app).as_ref() == app.page_history.pages.get(index) {
        app.page_history.index = index;
    }
    Task::batch([close_settings, select_page])
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn shortcut(
    event: &iced::Event,
    status: iced::event::Status,
) -> Option<HistoryDirection> {
    use iced::{event, keyboard};

    if status != event::Status::Ignored {
        return None;
    }
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Character(key),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };
    if *modifiers != keyboard::Modifiers::COMMAND {
        return None;
    }
    match key.as_str() {
        "[" => Some(HistoryDirection::Back),
        "]" => Some(HistoryDirection::Forward),
        _ => None,
    }
}

pub(crate) fn controls(app: &App) -> Element<'static, Message> {
    use desktop_assets::assets::Icon;
    use iced::widget::row;

    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let (back, forward) =
        if cfg!(target_os = "macos") { ("⌘[", "⌘]") } else { ("Ctrl+[", "Ctrl+]") };
    row![
        history_button(app, HistoryDirection::Back, Icon::ChevronLeft, i18n.__("返回"), back),
        history_button(
            app,
            HistoryDirection::Forward,
            Icon::ChevronRight,
            i18n.__("前进"),
            forward
        ),
    ]
    .spacing(4)
    .into()
}

fn history_button(
    app: &App,
    direction: HistoryDirection,
    icon: desktop_assets::assets::Icon,
    label: &str,
    shortcut: &'static str,
) -> Element<'static, Message> {
    use desktop_navigation::components::top_bar::widgets;
    use iced::widget::{button, container, row, svg, text, tooltip};

    let enabled = can_navigate(app, direction, None);
    let icon_alpha = if enabled { 1.0 } else { 0.22 };
    let icon = widgets::icon_svg(icon).style(move |theme, _| svg::Style {
        color: Some(theme.palette().text.scale_alpha(icon_alpha)),
    });
    let control = button(container(icon).center(24))
        .padding(0)
        .style(move |theme, status| {
            let mut style = widgets::icon_toggle_button_style(theme, status, enabled, false);
            style.border.radius = 12.0.into();
            style
        })
        .on_press_maybe(
            enabled.then_some(Message::View(ViewMessage::NavigateHistory(direction, None))),
        );
    let shortcut =
        container(text(shortcut).size(12)).padding([2, 6]).style(|theme: &iced::Theme| {
            container::Style {
                background: Some(theme.palette().text.scale_alpha(0.14).into()),
                border: iced::border::rounded(8),
                ..Default::default()
            }
        });
    let hint =
        container(row![text(label.to_owned()).size(13), shortcut].spacing(8).align_y(iced::Center))
            .padding([4, 8])
            .style(|theme| {
                let mut style = widgets::icon_tooltip_style(theme);
                style.border.radius = 14.0.into();
                style
            });
    tooltip(control, hint, tooltip::Position::Bottom).gap(4).into()
}

#[cfg(test)]
#[path = "page_history_tests.rs"]
mod tests;
