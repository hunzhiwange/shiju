// 验证身份信息到系统提示词的稳定渲染与可选字段处理。

// 提示词测试固定键排序，确保相同身份数据在多次运行中生成稳定文本。
use super::*;
use crate::identity::model::{
    AieosIdentity, CapabilitiesSection, HistorySection, IdentitySection, InterestsSection,
    LinguisticsSection, MotivationsSection, Names, OceanTraits, PhysicalitySection,
    PsychologySection,
};
use std::collections::HashMap;

#[test]
fn prompt_is_empty_for_empty_identity() {
    assert!(aieos_to_system_prompt(&AieosIdentity::default()).is_empty());
}

#[test]
fn prompt_uses_full_name_when_first_name_is_absent() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names { full: Some("Single Name".to_string()), ..Default::default() }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);

    assert!(prompt.contains("**Name:** Single Name"));
    assert!(prompt.contains("**姓名:** Single Name"));
    assert!(!prompt.contains("**Full Name:**"));
}

#[test]
fn prompt_keeps_absent_optional_collections_quiet_but_emits_section_headings() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection::default()),
        psychology: Some(PsychologySection::default()),
        linguistics: Some(LinguisticsSection::default()),
        motivations: Some(MotivationsSection::default()),
        capabilities: Some(CapabilitiesSection::default()),
        history: Some(HistorySection::default()),
        interests: Some(InterestsSection::default()),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);

    assert!(prompt.contains("## Identity"));
    assert!(prompt.contains("## Personality"));
    assert!(prompt.contains("## Communication Style"));
    assert!(prompt.contains("## Motivations"));
    assert!(prompt.contains("## Capabilities"));
    assert!(prompt.contains("## Background"));
    assert!(prompt.contains("## Interests"));
    assert!(!prompt.contains("**Name:**"));
    assert!(!prompt.contains("**神经矩阵"));
    assert!(!prompt.contains("**口头禅:**"));
    assert!(!prompt.contains("**Short-term Goals:**"));
    assert!(!prompt.contains("**Skills:**"));
    assert!(!prompt.contains("**Education:**"));
    assert!(!prompt.contains("**Favorites:**"));
}
