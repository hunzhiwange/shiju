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
fn prompt_sorts_neural_matrix_keys() {
    let mut matrix = HashMap::new();
    matrix.insert("zeta".to_string(), 0.2);
    matrix.insert("alpha".to_string(), 0.8);
    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names { full: Some("Nova".into()), ..Default::default() }),
            ..Default::default()
        }),
        psychology: Some(PsychologySection { neural_matrix: Some(matrix), ..Default::default() }),
        ..Default::default()
    };
    let prompt = aieos_to_system_prompt(&identity);
    assert!(prompt.contains("**姓名:** Nova"));
    assert!(prompt.find("alpha").unwrap() < prompt.find("zeta").unwrap());
}

#[test]
fn prompt_includes_every_populated_section() {
    let mut matrix = HashMap::new();
    matrix.insert("logic".to_string(), 0.91);
    matrix.insert("creativity".to_string(), 0.82);

    let mut favorites = HashMap::new();
    favorites.insert("snack".to_string(), "tea".to_string());
    favorites.insert("book".to_string(), "rust".to_string());

    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names {
                first: Some("Ada".to_string()),
                last: Some("Lovelace".to_string()),
                nickname: Some("Enchantress".to_string()),
                full: None,
            }),
            bio: Some("Mathematician".to_string()),
            origin: Some("London".to_string()),
            residence: Some("Analytical Engine".to_string()),
        }),
        psychology: Some(PsychologySection {
            neural_matrix: Some(matrix),
            mbti: Some("INTJ".to_string()),
            ocean: Some(OceanTraits {
                openness: Some(0.95),
                conscientiousness: Some(0.88),
                extraversion: Some(0.31),
                agreeableness: Some(0.72),
                neuroticism: Some(0.12),
            }),
            moral_compass: Some(vec!["Curiosity".to_string(), "Care".to_string()]),
        }),
        linguistics: Some(LinguisticsSection {
            style: Some("precise".to_string()),
            formality: Some("formal".to_string()),
            catchphrases: Some(vec!["Numbers sing".to_string()]),
            forbidden_words: Some(vec!["impossible".to_string()]),
        }),
        motivations: Some(MotivationsSection {
            core_drive: Some("Reveal patterns".to_string()),
            short_term_goals: Some(vec!["Draft notes".to_string()]),
            long_term_goals: Some(vec!["Build engines".to_string()]),
            fears: Some(vec!["Wasted potential".to_string()]),
        }),
        capabilities: Some(CapabilitiesSection {
            skills: Some(vec!["analysis".to_string()]),
            tools: Some(vec!["notebook".to_string()]),
        }),
        history: Some(HistorySection {
            origin_story: Some("Raised among equations".to_string()),
            education: Some(vec!["Self study".to_string()]),
            occupation: Some("Computer pioneer".to_string()),
        }),
        physicality: Some(PhysicalitySection {
            appearance: Some("Victorian scholar".to_string()),
            avatar_description: Some("Portrait with brass gears".to_string()),
        }),
        interests: Some(InterestsSection {
            hobbies: Some(vec!["poetry".to_string()]),
            favorites: Some(favorites),
            lifestyle: Some("Nocturnal study".to_string()),
        }),
    };

    let prompt = aieos_to_system_prompt(&identity);

    assert!(prompt.contains("## 身份信息"));
    assert!(prompt.contains("**Name:** Ada"));
    assert!(prompt.contains("**Full Name:** Ada Lovelace"));
    assert!(prompt.contains("**Nickname:** Enchantress"));
    assert!(prompt.contains("**Bio:** Mathematician"));
    assert!(prompt.contains("**Origin:** London"));
    assert!(prompt.contains("**居住地:** Analytical Engine"));
    assert!(prompt.contains("**MBTI:** INTJ"));
    assert!(prompt.contains("- Openness: 0.95"));
    assert!(prompt.contains("- 尽责性: 0.88"));
    assert!(prompt.contains("- 外向性: 0.31"));
    assert!(prompt.contains("- 宜人性: 0.72"));
    assert!(prompt.contains("- 神经质: 0.12"));
    assert!(prompt.find("- creativity: 0.82").unwrap() < prompt.find("- logic: 0.91").unwrap());
    assert!(prompt.contains("- Curiosity"));
    assert!(prompt.contains("**Style:** precise"));
    assert!(prompt.contains("**Formality Level:** formal"));
    assert!(prompt.contains("- \"Numbers sing\""));
    assert!(prompt.contains("**避免使用的词语:**"));
    assert!(prompt.contains("- impossible"));
    assert!(prompt.contains("**Core Drive:** Reveal patterns"));
    assert!(prompt.contains("**Short-term Goals:**"));
    assert!(prompt.contains("- Draft notes"));
    assert!(prompt.contains("**Long-term Goals:**"));
    assert!(prompt.contains("- Build engines"));
    assert!(prompt.contains("**Fears/Avoidances:**"));
    assert!(prompt.contains("- Wasted potential"));
    assert!(prompt.contains("**Skills:**"));
    assert!(prompt.contains("- analysis"));
    assert!(prompt.contains("**Tools Access:**"));
    assert!(prompt.contains("- notebook"));
    assert!(prompt.contains("**Origin Story:** Raised among equations"));
    assert!(prompt.contains("**Education:**"));
    assert!(prompt.contains("- Self study"));
    assert!(prompt.contains("**Occupation:** Computer pioneer"));
    assert!(prompt.contains("Victorian scholar"));
    assert!(prompt.contains("**Avatar Description:** Portrait with brass gears"));
    assert!(prompt.contains("**Hobbies:**"));
    assert!(prompt.contains("- poetry"));
    assert!(prompt.find("- book: rust").unwrap() < prompt.find("- snack: tea").unwrap());
    assert!(prompt.contains("**Lifestyle:** Nocturnal study"));
    assert_eq!(prompt, prompt.trim());
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
fn prompt_keeps_empty_optional_collections_quiet_but_emits_section_headings() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names { first: Some("Ada".to_string()), ..Default::default() }),
            ..Default::default()
        }),
        psychology: Some(PsychologySection {
            neural_matrix: Some(HashMap::new()),
            moral_compass: Some(Vec::new()),
            ..Default::default()
        }),
        linguistics: Some(LinguisticsSection {
            catchphrases: Some(Vec::new()),
            forbidden_words: Some(Vec::new()),
            ..Default::default()
        }),
        motivations: Some(MotivationsSection {
            short_term_goals: Some(Vec::new()),
            long_term_goals: Some(Vec::new()),
            fears: Some(Vec::new()),
            ..Default::default()
        }),
        capabilities: Some(CapabilitiesSection {
            skills: Some(Vec::new()),
            tools: Some(Vec::new()),
        }),
        history: Some(HistorySection { education: Some(Vec::new()), ..Default::default() }),
        interests: Some(InterestsSection {
            hobbies: Some(Vec::new()),
            favorites: Some(HashMap::new()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);

    assert!(prompt.contains("## Personality"));
    assert!(prompt.contains("## Communication Style"));
    assert!(prompt.contains("## Motivations"));
    assert!(prompt.contains("## Capabilities"));
    assert!(prompt.contains("## Background"));
    assert!(prompt.contains("## Interests"));
    assert!(!prompt.contains("**Full Name:**"));
    assert!(!prompt.contains("**Short-term Goals:**"));
    assert!(!prompt.contains("**Skills:**"));
    assert!(!prompt.contains("**Favorites:**"));
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
