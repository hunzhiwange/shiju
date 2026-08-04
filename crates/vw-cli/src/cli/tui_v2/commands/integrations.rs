use super::super::model::integration::IntegrationKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationCommand {
    Mcp,
    Memory,
    Skills,
    Plugins,
    Hooks,
    Doctor,
    ReloadSkills,
    ReloadPlugins,
}

impl IntegrationCommand {
    pub(crate) fn kind(self) -> IntegrationKind {
        match self {
            Self::Mcp => IntegrationKind::Mcp,
            Self::Memory => IntegrationKind::Memory,
            Self::Skills | Self::ReloadSkills => IntegrationKind::Skill,
            Self::Plugins | Self::ReloadPlugins => IntegrationKind::Plugin,
            Self::Hooks => IntegrationKind::Hook,
            Self::Doctor => IntegrationKind::Doctor,
        }
    }

    pub(crate) fn refresh(self) -> bool {
        matches!(self, Self::ReloadSkills | Self::ReloadPlugins)
    }
}
