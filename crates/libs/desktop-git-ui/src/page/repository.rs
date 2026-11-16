use crate::git_panel::messages::UiMessage;
use crate::git_panel::repository as domain;
use crate::page::Message;
use crate::page::ViewContext;
fn input<'a>(app: &ViewContext<'a>) -> domain::Input<'a, Message> {
    domain::Input {
        git_repository: &app.git.git_repository,
        repo_available: super::git_repo_path_for_app(app).is_some(),
        context_path: app.context_path.clone(),
        git_commit_in_progress: app.git.git_commit_in_progress,
        history_file: {
            let app = (*app).clone();
            Box::new(move |index, file| {
                super::diff_view::view_history_diff(&app, index, file).map(UiMessage::External)
            })
        },
    }
}
pub(super) fn toolbar<'a>(app: &ViewContext<'a>) -> iced::Element<'a, Message> {
    domain::toolbar(&input(app)).map(super::view_messages::map)
}
pub(super) fn view<'a>(app: &ViewContext<'a>) -> iced::Element<'a, Message> {
    domain::view(input(app)).map(super::view_messages::map)
}
