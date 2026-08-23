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
fn normalize_drops_sections_without_useful_values() {
    let payload = json!({
        "identity": {
            "names": {"first": " ", "last": [], "nickname": {}, "full": null},
            "bio": null,
            "origin": [],
            "residence": {}
        },
        "psychology": {
            "neural_matrix": {"@meta": 1, "logic": "not-a-number", "flag": true},
            "ocean": {"openness": false},
            "moral_compass": {"@note": "skip"}
        },
        "linguistics": {
            "catchphrases": [],
            "forbidden_words": [""],
            "text_style": {"style_descriptors": [], "formality_level": null}
        },
        "motivations": {
            "short_term_goals": [],
            "long_term_goals": [],
            "fears": []
        },
        "capabilities": {"skills": [], "tools": []},
        "physicality": {"face": {}, "body": {}, "style": {}, "image_prompts": {}},
        "history": {"education": []},
        "interests": {
            "hobbies": [],
            "favorites": {"@meta": "ignored"},
            "lifestyle": null
        }
    });

    let normalized = normalize_aieos_identity(&payload);

    assert!(normalized.identity.is_none());
    assert!(normalized.psychology.is_none());
    assert!(normalized.linguistics.is_none());
    assert!(normalized.motivations.is_none());
    assert!(normalized.capabilities.is_none());
    assert!(normalized.physicality.is_none());
    assert!(normalized.history.is_none());
    assert!(normalized.interests.is_none());
}

#[test]
fn normalize_direct_values_trim_coerce_and_dedupe() {
    let payload = json!({
        "identity": {
            "names": {"first": " Ada ", "last": " Lovelace ", "nickname": true},
            "bio": ["mathematician", 42, true],
            "origin": {
                "@meta": "ignored",
                "city": "London",
                "birthplace": {"country": "UK"}
            },
            "residence": false
        },
        "psychology": {
            "neural_matrix": {
                "@meta": 1,
                "logic": "0.75",
                "creativity": 1,
                "bad": "not-a-number"
            },
            "mbti": " INTJ ",
            "ocean": {
                "openness": "0.90",
                "conscientiousness": 0.8,
                "extraversion": "0.70",
                "agreeableness": 0.6,
                "neuroticism": "0.50"
            },
            "moral_compass": [
                "Care",
                "care",
                {"title": "Justice"},
                {"name": "Wisdom"},
                {"detail": ["Nested"]}
            ]
        },
        "linguistics": {
            "style": {"tone": "Warm", "@meta": "ignored"},
            "formality": true,
            "catchphrases": [" hello ", "HELLO", {"name": "cheers"}],
            "forbidden_words": "never"
        },
        "motivations": {
            "core_drive": {"why": "help"},
            "short_term_goals": "ship",
            "long_term_goals": {"title": "learn"},
            "fears": {
                "rational": ["bugs"],
                "irrational": ["void", "BUGS"]
            }
        },
        "capabilities": {
            "skills": [{"name": "coding"}, {"title": "writing"}, {"detail": "analysis"}],
            "tools": "shell"
        },
        "physicality": {
            "appearance": ["tall", {"hair": "blue"}],
            "avatar_description": true
        },
        "history": {
            "origin_story": 123,
            "education": [{"name": "MIT"}, ["Lab"]],
            "occupation": {"title": "Engineer", "industry": "AI"}
        },
        "interests": {
            "hobbies": ["rust", ["music"], {"title": "tea"}],
            "favorites": {
                "@meta": "ignored",
                "color": "blue",
                "foods": ["noodles", "tea"],
                "profile": {"name": "favorite place"}
            },
            "lifestyle": {"sleep_schedule": "late"}
        }
    });

    let normalized = normalize_aieos_identity(&payload);
    let identity = normalized.identity.unwrap();
    let names = identity.names.unwrap();
    assert_eq!(names.first.as_deref(), Some("Ada"));
    assert_eq!(names.last.as_deref(), Some("Lovelace"));
    assert_eq!(names.nickname.as_deref(), Some("true"));
    assert_eq!(names.full.as_deref(), Some("Ada Lovelace"));
    assert_eq!(identity.bio.as_deref(), Some("mathematician, 42, true"));
    assert!(identity.origin.unwrap().contains("city: London"));
    assert_eq!(identity.residence.as_deref(), Some("false"));

    let psychology = normalized.psychology.unwrap();
    let matrix = psychology.neural_matrix.unwrap();
    assert_eq!(matrix.get("logic"), Some(&0.75));
    assert_eq!(matrix.get("creativity"), Some(&1.0));
    assert!(!matrix.contains_key("@meta"));
    assert_eq!(psychology.mbti.as_deref(), Some("INTJ"));
    let ocean = psychology.ocean.unwrap();
    assert_eq!(ocean.openness, Some(0.9));
    assert_eq!(ocean.conscientiousness, Some(0.8));
    assert_eq!(ocean.extraversion, Some(0.7));
    assert_eq!(ocean.agreeableness, Some(0.6));
    assert_eq!(ocean.neuroticism, Some(0.5));
    assert_eq!(
        psychology.moral_compass.unwrap(),
        vec!["Care", "Justice", "Wisdom", "detail: Nested"]
    );

    let linguistics = normalized.linguistics.unwrap();
    assert_eq!(linguistics.style.as_deref(), Some("tone: Warm"));
    assert_eq!(linguistics.formality.as_deref(), Some("true"));
    assert_eq!(linguistics.catchphrases.unwrap(), vec!["hello", "cheers"]);
    assert_eq!(linguistics.forbidden_words.unwrap(), vec!["never"]);

    let motivations = normalized.motivations.unwrap();
    assert_eq!(motivations.core_drive.as_deref(), Some("why: help"));
    assert_eq!(motivations.short_term_goals.unwrap(), vec!["ship"]);
    assert_eq!(motivations.long_term_goals.unwrap(), vec!["learn"]);
    assert_eq!(motivations.fears.unwrap(), vec!["bugs", "void"]);

    let capabilities = normalized.capabilities.unwrap();
    assert_eq!(capabilities.skills.unwrap(), vec!["coding", "writing", "detail: analysis"]);
    assert_eq!(capabilities.tools.unwrap(), vec!["shell"]);

    let physicality = normalized.physicality.unwrap();
    assert_eq!(physicality.appearance.as_deref(), Some("tall, hair: blue"));
    assert_eq!(physicality.avatar_description.as_deref(), Some("true"));

    let history = normalized.history.unwrap();
    assert_eq!(history.origin_story.as_deref(), Some("123"));
    assert_eq!(history.education.unwrap(), vec!["MIT", "Lab"]);
    assert!(history.occupation.unwrap().contains("Engineer"));

    let interests = normalized.interests.unwrap();
    assert_eq!(interests.hobbies.unwrap(), vec!["rust", "music", "tea"]);
    let favorites = interests.favorites.unwrap();
    assert_eq!(favorites.get("color").map(String::as_str), Some("blue"));
    assert_eq!(favorites.get("foods").map(String::as_str), Some("noodles, tea"));
    assert_eq!(favorites.get("profile").map(String::as_str), Some("name: favorite place"));
    assert!(!favorites.contains_key("@meta"));
    assert_eq!(interests.lifestyle.as_deref(), Some("sleep schedule: late"));
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
