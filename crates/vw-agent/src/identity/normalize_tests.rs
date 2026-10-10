// 验证身份输入的空白处理、字段规范化与长度边界。

// 规范化会裁剪、去重并兼容嵌套生成器形状，但不凭空保留空章节。
use super::*;
use serde_json::json;

#[test]
fn normalize_builds_full_name_from_first_and_last() {
    let payload = json!({"identity": {"names": {"first": "Ada", "last": "Lovelace"}}});
    let normalized = normalize_aieos_identity(&payload);
    let names = normalized.identity.unwrap().names.unwrap();
    assert_eq!(names.full.as_deref(), Some("Ada Lovelace"));
}

#[test]
fn normalize_ignores_empty_sections_and_dedupes_lists() {
    let payload = json!({"linguistics": {}, "psychology": {"moral_compass": ["care", "", "care"]}});
    let normalized = normalize_aieos_identity(&payload);
    assert!(normalized.linguistics.is_none());
    assert_eq!(normalized.psychology.unwrap().moral_compass.unwrap(), vec!["care"]);
}



#[test]
// 兼容生成器的嵌套形状是输入适配边界，不能影响规范化后的统一模型。
fn normalize_supports_nested_generator_fallback_shape() {
    let payload = json!({
        "identity": {"names": {"full": "Marta Jankowska"}},
        "psychology": {
            "traits": {
                "mbti": "ISFJ",
                "ocean": {"openness": 0.4}
            },
            "moral_compass": {
                "alignment": "Lawful Good",
                "core_values": ["Loyalty", "Helpfulness"],
                "conflict_resolution_style": "Seeks compromise"
            }
        },
        "linguistics": {
            "text_style": {
                "style_descriptors": ["Sincere", "Grounded"],
                "formality_level": 0.6
            },
            "idiolect": {
                "catchphrases": ["Stay calm"],
                "forbidden_words": ["severe profanity"]
            }
        },
        "motivations": {
            "goals": {
                "short_term": ["Expand greenhouse"],
                "long_term": ["Support community"]
            },
            "fears": {"name": "Deep water"}
        },
        "physicality": {
            "face": {"shape": "round"},
            "body": {"build_description": "compact"},
            "style": {"aesthetic_archetype": "minimal"},
            "image_prompts": {"portrait": "A friendly portrait"}
        }
    });

    let normalized = normalize_aieos_identity(&payload);

    let names = normalized.identity.unwrap().names.unwrap();
    assert_eq!(names.full.as_deref(), Some("Marta Jankowska"));
    assert!(names.first.is_none());

    let psychology = normalized.psychology.unwrap();
    assert_eq!(psychology.mbti.as_deref(), Some("ISFJ"));
    assert_eq!(psychology.ocean.unwrap().openness, Some(0.4));
    assert_eq!(
        psychology.moral_compass.unwrap(),
        vec![
            "Alignment: Lawful Good",
            "Loyalty",
            "Helpfulness",
            "Conflict style: Seeks compromise"
        ]
    );

    let linguistics = normalized.linguistics.unwrap();
    assert_eq!(linguistics.style.as_deref(), Some("Sincere, Grounded"));
    assert_eq!(linguistics.formality.as_deref(), Some("0.60"));
    assert_eq!(linguistics.catchphrases.unwrap(), vec!["Stay calm"]);
    assert_eq!(linguistics.forbidden_words.unwrap(), vec!["severe profanity"]);

    let motivations = normalized.motivations.unwrap();
    assert_eq!(motivations.short_term_goals.unwrap(), vec!["Expand greenhouse"]);
    assert_eq!(motivations.long_term_goals.unwrap(), vec!["Support community"]);
    assert_eq!(motivations.fears.unwrap(), vec!["Deep water"]);

    let physicality = normalized.physicality.unwrap();
    assert_eq!(
        physicality.appearance.as_deref(),
        Some("脸型: round; 体型: compact; 审美: minimal")
    );
    assert_eq!(physicality.avatar_description.as_deref(), Some("A friendly portrait"));
}

#[test]
// 无法识别的复合值保守提取可用字段，标量恐惧项也应正常接收。
fn normalize_uses_object_fallbacks_for_unrecognized_moral_compass_and_fears() {
    let payload = json!({
        "psychology": {"moral_compass": {"name": "Fallback principle"}},
        "motivations": {"fears": {"title": "Unknown future"}},
        "linguistics": {"text_style": {"formality_level": "formal"}}
    });

    let normalized = normalize_aieos_identity(&payload);

    assert_eq!(normalized.psychology.unwrap().moral_compass.unwrap(), vec!["Fallback principle"]);
    assert_eq!(normalized.motivations.unwrap().fears.unwrap(), vec!["Unknown future"]);
    assert_eq!(normalized.linguistics.unwrap().formality.as_deref(), Some("formal"));
}

#[test]
fn normalize_accepts_scalar_fears() {
    let payload = json!({"motivations": {"fears": "losing context"}});

    let normalized = normalize_aieos_identity(&payload);

    assert_eq!(normalized.motivations.unwrap().fears.unwrap(), vec!["losing context"]);
}
