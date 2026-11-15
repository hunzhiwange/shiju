//! Project profile state is owned by the workspace domain; host identity stays opaque.
pub use desktop_workspace::project_profiles::{Action, Management, Preview};
pub type State = desktop_workspace::project_profiles::State<crate::app::message::project_profiles::Identity>;
pub type Application = desktop_workspace::project_profiles::Application<crate::app::message::project_profiles::Identity>;
