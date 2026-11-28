pub mod omo;
pub mod omp;
pub mod pi;

pub use desktop_apps_ui::aggregation::Notice;

pub(crate) fn error<E>(notice: &mut Option<Notice>, text: impl Into<desktop_apps_ui::Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Error(text.into().into_message()));
    iced::Task::none()
}
fn info<E>(notice: &mut Option<Notice>, text: impl Into<desktop_apps_ui::Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Info(text.into().into_message()));
    iced::Task::none()
}
fn success<E>(notice: &mut Option<Notice>, text: impl Into<desktop_apps_ui::Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Success(text.into().into_message()));
    iced::Task::none()
}
fn warning<E>(notice: &mut Option<Notice>, text: impl Into<desktop_apps_ui::Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Warning(text.into().into_message()));
    iced::Task::none()
}
