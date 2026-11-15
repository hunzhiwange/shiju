pub mod list;
pub mod clients;
pub mod publish;
pub mod desktop;
pub mod omo;
pub mod omp;
pub mod pi;
pub use list::{Form, Selection, supported};

pub use desktop_apps_ui::aggregation::Notice;

pub(crate) fn error<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Error(text.into()));
    iced::Task::none()
}
fn info<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Info(text.into()));
    iced::Task::none()
}
fn success<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Success(text.into()));
    iced::Task::none()
}
fn warning<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Warning(text.into()));
    iced::Task::none()
}

pub mod subagent;

#[cfg(test)]
mod aggregation_subagent_tests;
#[cfg(test)]
mod aggregation_review_tests;
