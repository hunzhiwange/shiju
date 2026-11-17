pub use desktop_navigation::components::top_bar::types::{WorkspaceMode, AppTab, TopBarGatewayTab, ExternalOpenApp, RuntimePlatform};

pub use desktop_navigation::presentation::*;

pub use desktop_navigation::components::model_hover_tooltip::ModelPopoverHover;


#[cfg(test)]
#[path = "presentation_tests.rs"]
mod presentation_tests;

#[path = "mobile_layout.rs"]
mod mobile_layout;
pub use mobile_layout::{LayoutMode, LayoutPreference, MobileLayoutState};

#[cfg(test)]
pub use desktop_git_ui::git_panel::types::ConventionalCommitType;
