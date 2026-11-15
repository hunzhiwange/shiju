//! Effects requiring desktop toast or cross-domain project navigation.
pub enum Effect {
    Toast { success: bool, message: &'static str },
    Open { project: vw_api_project::ProjectDto, directory: String },
}
