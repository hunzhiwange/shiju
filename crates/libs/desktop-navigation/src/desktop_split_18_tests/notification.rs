use crate::components::notification::{Input, Message, Notification, view};
#[derive(Default)]
struct Fixture {
    notifications: Vec<Notification>,
    notification_editors: std::collections::HashMap<usize, iced::widget::text_editor::Content>,
    copied_notification_id: Option<usize>,
}
impl Fixture {
    fn add(&mut self, message: String) {
        let id = self.notifications.len();
        self.notification_editors
            .insert(id, iced::widget::text_editor::Content::with_text(&message));
        self.notifications.push(Notification {
            id,
            message,
            localized_message: None,
            created_at: web_time::SystemTime::now(),
        });
    }
    fn input(&self) -> Input<'_> {
        Input {
            desktop_locale: Default::default(),
            notifications: &self.notifications,
            notification_editors: &self.notification_editors,
            copied_notification_id: self.copied_notification_id,
            notifications_scroll_id: iced::widget::Id::unique(),
        }
    }
}
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}
#[test]
fn view_builds_empty_notification_panel() {
    let app = Fixture::default();

    keep_element(view(app.input()));
}

#[test]
fn view_builds_notification_items_with_editors_and_copy_feedback() {
    let mut app = Fixture::default();
    app.add("first".to_string());
    app.add("second".to_string());
    app.copied_notification_id = Some(app.notifications[1].id);

    keep_element(view(app.input()));
}

#[test]
fn view_builds_notification_items_without_editor_content() {
    let mut app = Fixture::default();
    app.add("plain".to_string());
    app.notification_editors.clear();

    keep_element(view(app.input()));
}
