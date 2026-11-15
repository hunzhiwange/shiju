//! AIEOS 身份纯模型、规范化与系统提示转换。

mod model;
mod normalize;
mod prompt;

pub use model::{
    AieosIdentity, CapabilitiesSection, HistorySection, IdentitySection, InterestsSection,
    LinguisticsSection, MotivationsSection, Names, OceanTraits, PhysicalitySection,
    PsychologySection,
};
pub use normalize::normalize_aieos_identity;
pub use prompt::aieos_to_system_prompt;

#[cfg(test)]
mod tests;
