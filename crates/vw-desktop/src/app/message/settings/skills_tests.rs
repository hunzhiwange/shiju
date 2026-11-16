use super::*;
use crate::app::message::settings::AgentsMessage;
fn app() -> App { App::new().0 }
async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[tokio::test(flavor = "multi_thread")]
async fn skills_project_selection_refreshes_current_and_opens_other_project() {
    let mut app = app();
    app.project_path = Some("/repo/current".to_string());

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SkillsProjectSelected("/repo/current".to_string()),
    ))
    .await;

    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::ProjectChanged(project_path)))
            if project_path == "/repo/current"
    )));
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Settings(SettingsMessage::SkillsRefresh)))
    );

    let _ = update(&mut app, SettingsMessage::SkillsProjectSelected("/repo/other".to_string()));

    assert_eq!(app.project_path.as_deref(), Some("/repo/other"));
    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/repo/other"));
    assert!(app.agents_settings.loading);
}
