pub mod list;
pub mod clients;
pub mod publish;
pub mod desktop;
pub mod omo;
pub mod omp;
pub mod pi;
pub use list::{Form, Selection, supported};

pub enum Notice { Error(String), Info(String), Success(String), Warning(String) }
fn error<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
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
