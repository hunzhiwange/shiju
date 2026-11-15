use crate::{archived_chats, state::{ArchivedChatsState, ArchivedChatsSort}};
use crate::archived_chats::ArchivedChatsMessage as Archive;
use super::fixture::fixture;
use iced::Theme;
const CASES: [&str; 15] = [
    "list",
    "sort-closed",
    "project-closed",
    "actions",
    "filtered",
    "created",
    "title",
    "delete",
    "deleting",
    "loading",
    "error",
    "empty",
    "restoring",
    "partial",
    "reloaded",
];
pub struct Scenarios {
    pub state: ArchivedChatsState,
    pub case: usize,
    pub error: ArchivedChatsState,
    pub partial: ArchivedChatsState,
    pub reloaded: ArchivedChatsState,
    pub identity: archived_chats::Identity,
}
impl Scenarios {
    pub fn setup(&mut self) {
        self.state = ArchivedChatsState {
            sessions: fixture(),
            identity: Some(self.identity.clone()),
            project_names: [
                ("batch".into(), "同名项目".into()),
                ("duplicate".into(), "同名项目".into()),
            ]
            .into(),
            ..Default::default()
        };
        let identity = self.state.identity.clone().unwrap();
        let state = &mut self.state;
        let message = match self.case % CASES.len() {
            3 => Some(Archive::ProjectActionsToggled("batch".into())),
            4 => Some(Archive::ProjectSelected(Some("duplicate".into()))),
            5 => Some(Archive::SortSelected(ArchivedChatsSort::Created)),
            6 => Some(Archive::SortSelected(ArchivedChatsSort::Title)),
            7 | 8 => Some(Archive::DeleteRequested("chat-0000".into())),
            9 => Some(Archive::Refresh),
            10 => {
                *state = self.error.clone();
                None
            }
            11 => {
                state.sessions.clear();
                None
            }
            12 => Some(Archive::Restore("chat-0000".into())),
            13 => {
                *state = self.partial.clone();
                None
            }
            14 => {
                *state = self.reloaded.clone();
                None
            }
            _ => None,
        };
        if let Some(message) = message {
            archived_chats::apply(state, message, &identity);
        }
        if self.case % CASES.len() == 8 {
            archived_chats::apply(state, Archive::DeleteConfirmed, &identity);
        }
    }
    pub fn theme(&self) -> Theme {
        if self.case / CASES.len() % 2 == 0 { Theme::Light } else { Theme::Dark }
    }
    pub fn width(&self) -> f32 {
        if self.case < CASES.len() * 2 { 860.0 } else { 360.0 }
    }
    pub fn finished(&self) -> bool { self.case == CASES.len() * 4 }
    pub fn capture_name(&self) -> String {
        format!("{}-{}-{}.png", if self.case / CASES.len() % 2 == 0 { "light" } else { "dark" }, self.width(), CASES[self.case % CASES.len()])
    }
    pub fn view(&self) -> iced::Element<'_, Archive> {
        iced::widget::container(crate::view::page(&self.state, config_ui::DesktopLocale::SimplifiedChinese))
            .padding(16).width(iced::Length::Fill).height(iced::Length::Fill).into()
    }
}
