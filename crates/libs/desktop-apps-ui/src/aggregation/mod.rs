pub mod pi;
pub mod pi_view;
pub mod omp;
pub mod omp_view;
pub mod omo;
pub mod omo_view;
pub mod omp_core;
pub mod omo_state;

pub enum Notice {
    Error(desktop_i18n::format::Message), Info(desktop_i18n::format::Message),
    Success(desktop_i18n::format::Message), Warning(desktop_i18n::format::Message),
}
pub(crate) fn error<E>(notice: &mut Option<Notice>, text: impl Into<crate::Feedback>) -> iced::Task<E> { *notice = Some(Notice::Error(text.into().into_message())); iced::Task::none() }
fn info<E>(notice: &mut Option<Notice>, text: impl Into<crate::Feedback>) -> iced::Task<E> { *notice = Some(Notice::Info(text.into().into_message())); iced::Task::none() }
