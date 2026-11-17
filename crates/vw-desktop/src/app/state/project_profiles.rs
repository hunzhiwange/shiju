//! Project profile state is owned by the workspace domain; host identity stays opaque.
pub type State = desktop_workspace::project_profiles::State<crate::app::message::project_profiles::Identity>;
#[cfg(test)]
pub type Application = desktop_workspace::project_profiles::Application<crate::app::message::project_profiles::Identity>;

#[cfg(test)]
pub use desktop_workspace::project_profiles::Action;
#[cfg(test)]
pub use desktop_workspace::project_profiles::Management;
#[cfg(test)]
pub use desktop_workspace::project_profiles::Preview;
