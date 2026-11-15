/// 轨迹详情检查器的稳定标签集合。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TrajectoryDetailsTab {
    SystemPrompt,
    Tools,
    #[default]
    Summary,
    Preview,
    Raw,
    Source,
    Options,
    Usage,
    Timing,
    Payload,
    Result,
    Schema,
}

impl TrajectoryDetailsTab {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SystemPrompt => "System Prompt",
            Self::Tools => "Tools",
            Self::Summary => "Summary",
            Self::Preview => "Preview",
            Self::Raw => "Raw",
            Self::Source => "Source",
            Self::Options => "Options",
            Self::Usage => "Usage",
            Self::Timing => "Timing",
            Self::Payload => "Payload",
            Self::Result => "Result",
            Self::Schema => "Schema",
        }
    }
}
