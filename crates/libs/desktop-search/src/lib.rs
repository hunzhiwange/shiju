pub mod components {
    pub mod search_panel;
    #[cfg(test)]
    mod search_panel_tests;
}
pub mod message;
pub use message::{State, SearchMessage, Effect, update};

#[cfg(test)]
mod desktop_split_09_tests;
