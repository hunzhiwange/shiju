pub mod charts;
pub mod display;
pub mod groups;
pub mod logs;
pub mod overview;
pub mod pricing;
pub mod scan;
#[derive(Clone, Debug)]
pub enum Outcome<E> { Event(E), Refresh, Error(String), Success(String) }

pub mod state;
pub mod event;
pub mod view;
