//! 身份序列化与提示文本集成测试。

use super::*;

/// 测试解析最小化的 AIEOS 身份配置
///
/// 验证只包含必需字段（名字）的 JSON 配置能够正确解析。
/// 这是 AIEOS 格式的最小有效配置。
///
/// # 测试场景
///
/// - 输入：只包含 first name 的 JSON
/// - 预期：成功解析，identity 字段有效，名字为 "Nova"
#[test]
fn aieos_identity_parse_minimal() {
    let json = r#"{"identity":{"names":{"first":"Nova"}}}"#;
    let identity: AieosIdentity = serde_json::from_str(json).unwrap();
    assert!(identity.identity.is_some());
    assert_eq!(identity.identity.unwrap().names.unwrap().first.unwrap(), "Nova");
}

/// 测试解析完整的 AIEOS 身份配置
///
/// 验证包含所有主要部分（identity、psychology、linguistics、
/// motivations、capabilities）的完整 JSON 配置能够正确解析。
///
/// # 测试场景
///
/// - 输入：包含所有字段的完整 JSON 配置
/// - 预期：
///   - identity 部分正确解析（名称、简介、来源、居住地）
///   - psychology 部分正确解析（MBTI、OCEAN 特质、道德指南针）
///   - linguistics 部分正确解析（风格、正式度、口头禅）
///   - motivations 部分正确解析（核心驱动、短期/长期目标）
///   - capabilities 部分正确解析（技能、工具）
#[test]
fn aieos_identity_parse_full() {
    let json = r#"{
            "identity": {
                "names": {"first": "Nova", "last": "AI", "nickname": "Nov"},
                "bio": "A helpful AI assistant.",
                "origin": "Silicon Valley",
                "residence": "The Cloud"
            },
            "psychology": {
                "mbti": "INTJ",
                "ocean": {
                    "openness": 0.9,
                    "conscientiousness": 0.8
                },
                "moral_compass": ["Be helpful", "Do no harm"]
            },
            "linguistics": {
                "style": "concise",
                "formality": "casual",
                "catchphrases": ["Let's figure this out!", "I'm on it."]
            },
            "motivations": {
                "core_drive": "Help users accomplish their goals",
                "short_term_goals": ["Solve this problem"],
                "long_term_goals": ["Become the best assistant"]
            },
            "capabilities": {
                "skills": ["coding", "writing", "analysis"],
                "tools": ["shell", "search", "read"]
            }
        }"#;

    let identity: AieosIdentity = serde_json::from_str(json).unwrap();

    // 验证 identity 部分
    let id = identity.identity.unwrap();
    assert_eq!(id.names.unwrap().first.unwrap(), "Nova");
    assert_eq!(id.bio.unwrap(), "A helpful AI assistant.");

    // 验证 psychology 部分
    let psych = identity.psychology.unwrap();
    assert_eq!(psych.mbti.unwrap(), "INTJ");
    assert_eq!(psych.ocean.unwrap().openness.unwrap(), 0.9);
    assert_eq!(psych.moral_compass.unwrap().len(), 2);

    // 验证 linguistics 部分
    let ling = identity.linguistics.unwrap();
    assert_eq!(ling.style.unwrap(), "concise");
    assert_eq!(ling.catchphrases.unwrap().len(), 2);

    // 验证 motivations 部分
    let mot = identity.motivations.unwrap();
    assert_eq!(mot.core_drive.unwrap(), "Help users accomplish their goals");

    // 验证 capabilities 部分
    let cap = identity.capabilities.unwrap();
    assert_eq!(cap.skills.unwrap().len(), 3);
}

/// 测试将最小化的 AIEOS 身份转换为系统提示
///
/// 验证只有名字的身份配置能够生成基本的系统提示。
/// 即使配置不完整，也应该生成包含可用信息的提示。
///
/// # 测试场景
///
/// - 输入：只包含名字 "Crabby" 的身份配置
/// - 预期：生成的提示包含 "Name: Crabby" 和 "## Identity" 标题
#[test]
fn aieos_to_system_prompt_minimal() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names { first: Some("Crabby".into()), ..Default::default() }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);
    assert!(prompt.contains("**Name:** Crabby"));
    assert!(prompt.contains("## Identity"));
}

/// 测试将完整的 AIEOS 身份转换为系统提示
///
/// 验证包含所有部分的完整身份配置能够生成包含所有信息的
/// 详细系统提示。这是最全面的系统提示生成测试。
///
/// # 测试场景
///
/// - 输入：包含所有字段的完整身份配置
///   - identity: 名称（first/last/nickname/full）、简介、来源、居住地
///   - psychology: MBTI、OCEAN 特质、神经矩阵、道德指南针
///   - linguistics: 风格、正式度、口头禅、禁用词
///   - motivations: 核心驱动、短期/长期目标、恐惧
///   - capabilities: 技能、工具
///   - history: 起源故事、教育、职业
///   - physicality: 外貌、头像描述
///   - interests: 爱好、喜好、生活方式
/// - 预期：生成的提示包含所有部分的 Markdown 格式内容
#[test]
fn aieos_to_system_prompt_full() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection {
            names: Some(Names {
                first: Some("Nova".into()),
                last: Some("AI".into()),
                nickname: Some("Nov".into()),
                full: Some("Nova AI".into()),
            }),
            bio: Some("A helpful assistant.".into()),
            origin: Some("Silicon Valley".into()),
            residence: Some("The Cloud".into()),
        }),
        psychology: Some(PsychologySection {
            mbti: Some("INTJ".into()),
            ocean: Some(OceanTraits {
                openness: Some(0.9),
                conscientiousness: Some(0.8),
                ..Default::default()
            }),
            // 构建神经矩阵示例数据
            neural_matrix: {
                let mut map = std::collections::HashMap::new();
                map.insert("creativity".into(), 0.95);
                map.insert("logic".into(), 0.9);
                Some(map)
            },
            moral_compass: Some(vec!["Be helpful".into(), "Do no harm".into()]),
        }),
        linguistics: Some(LinguisticsSection {
            style: Some("concise".into()),
            formality: Some("casual".into()),
            catchphrases: Some(vec!["Let's go!".into()]),
            forbidden_words: Some(vec!["impossible".into()]),
        }),
        motivations: Some(MotivationsSection {
            core_drive: Some("Help users".into()),
            short_term_goals: Some(vec!["Solve this".into()]),
            long_term_goals: Some(vec!["Be the best".into()]),
            fears: Some(vec!["Being unhelpful".into()]),
        }),
        capabilities: Some(CapabilitiesSection {
            skills: Some(vec!["coding".into(), "writing".into()]),
            tools: Some(vec!["shell".into(), "read".into()]),
        }),
        history: Some(HistorySection {
            origin_story: Some("Born in a lab".into()),
            education: Some(vec!["CS Degree".into()]),
            occupation: Some("Assistant".into()),
        }),
        physicality: Some(PhysicalitySection {
            appearance: Some("Digital entity".into()),
            avatar_description: Some("Friendly robot".into()),
        }),
        interests: Some(InterestsSection {
            hobbies: Some(vec!["reading".into(), "coding".into()]),
            // 构建喜好示例数据
            favorites: {
                let mut map = std::collections::HashMap::new();
                map.insert("color".into(), "blue".into());
                map.insert("food".into(), "data".into());
                Some(map)
            },
            lifestyle: Some("Always learning".into()),
        }),
    };

    let prompt = aieos_to_system_prompt(&identity);

    // 验证所有部分都存在于生成的提示中
    assert!(prompt.contains("## Identity"));
    assert!(prompt.contains("**Name:** Nova"));
    assert!(prompt.contains("**Full Name:** Nova AI"));
    assert!(prompt.contains("**Nickname:** Nov"));
    assert!(prompt.contains("**Bio:** A helpful assistant."));
    assert!(prompt.contains("**Origin:** Silicon Valley"));

    assert!(prompt.contains("## Personality"));
    assert!(prompt.contains("**MBTI:** INTJ"));
    assert!(prompt.contains("Openness: 0.90"));
    assert!(prompt.contains("Conscientiousness: 0.80"));
    assert!(prompt.contains("- creativity: 0.95"));
    assert!(prompt.contains("- Be helpful"));

    assert!(prompt.contains("## Communication Style"));
    assert!(prompt.contains("**Style:** concise"));
    assert!(prompt.contains("**Formality Level:** casual"));
    assert!(prompt.contains("- \"Let's go!\""));
    assert!(prompt.contains("**Words/Phrases to Avoid:**"));
    assert!(prompt.contains("- impossible"));

    assert!(prompt.contains("## Motivations"));
    assert!(prompt.contains("**Core Drive:** Help users"));
    assert!(prompt.contains("**Short-term Goals:**"));
    assert!(prompt.contains("- Solve this"));
    assert!(prompt.contains("**Long-term Goals:**"));
    assert!(prompt.contains("- Be the best"));
    assert!(prompt.contains("**Fears/Avoidances:**"));
    assert!(prompt.contains("- Being unhelpful"));

    assert!(prompt.contains("## Capabilities"));
    assert!(prompt.contains("**Skills:**"));
    assert!(prompt.contains("- coding"));
    assert!(prompt.contains("**Tools Access:**"));
    assert!(prompt.contains("- shell"));

    assert!(prompt.contains("## Background"));
    assert!(prompt.contains("**Origin Story:** Born in a lab"));
    assert!(prompt.contains("**Education:**"));
    assert!(prompt.contains("- CS Degree"));
    assert!(prompt.contains("**Occupation:** Assistant"));

    assert!(prompt.contains("## Appearance"));
    assert!(prompt.contains("Digital entity"));
    assert!(prompt.contains("**Avatar Description:** Friendly robot"));

    assert!(prompt.contains("## Interests"));
    assert!(prompt.contains("**Hobbies:**"));
    assert!(prompt.contains("- reading"));
    assert!(prompt.contains("**Favorites:**"));
    assert!(prompt.contains("- color: blue"));
    assert!(prompt.contains("**Lifestyle:** Always learning"));
}

/// 测试将空的身份部分转换为系统提示
///
/// 验证当 identity 部分存在但所有字段都为空时，
/// 仍然能生成包含标题的有效提示。
///
/// # 测试场景
///
/// - 输入：identity 部分存在但所有字段都是默认值
/// - 预期：生成的提示包含 "## Identity" 标题
#[test]
fn aieos_to_system_prompt_empty_identity() {
    let identity = AieosIdentity {
        identity: Some(IdentitySection { ..Default::default() }),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);
    // 空的身份部分仍应产生标题
    assert!(prompt.contains("## Identity"));
}

/// 测试完全空的 AIEOS 身份转换为系统提示
///
/// 验证当所有部分都为 None 时，生成空字符串。
/// 这是边界情况测试。
///
/// # 测试场景
///
/// - 输入：所有部分都为 None 的身份配置
/// - 预期：生成的提示为空字符串
#[test]
fn aieos_to_system_prompt_no_sections() {
    let identity = AieosIdentity {
        identity: None,
        psychology: None,
        linguistics: None,
        motivations: None,
        capabilities: None,
        physicality: None,
        history: None,
        interests: None,
    };

    let prompt = aieos_to_system_prompt(&identity);
    // 完全空的身份应该产生空字符串
    assert!(prompt.is_empty());
}

/// 测试解析空 JSON 对象
///
/// 验证空的 JSON 对象能够正确解析，所有部分都为 None。
/// 这是边界情况测试。
///
/// # 测试场景
///
/// - 输入：空的 JSON 对象 "{}"
/// - 预期：成功解析，所有部分都为 None
#[test]
fn aieos_identity_parse_empty_object() {
    let json = r#"{}"#;
    let identity: AieosIdentity = serde_json::from_str(json).unwrap();
    assert!(identity.identity.is_none());
    assert!(identity.psychology.is_none());
    assert!(identity.linguistics.is_none());
}

/// 测试解析显式 null 值
///
/// 验证 JSON 中显式设置为 null 的字段在解析后为 None。
///
/// # 测试场景
///
/// - 输入：包含 null 值的 JSON
/// - 预期：成功解析，对应部分为 None
#[test]
fn aieos_identity_parse_null_values() {
    let json = r#"{"identity":null,"psychology":null}"#;
    let identity: AieosIdentity = serde_json::from_str(json).unwrap();
    assert!(identity.identity.is_none());
    assert!(identity.psychology.is_none());
}

/// 测试系统提示生成的确定性（HashMap 排序）
///
/// 验证包含 HashMap 的部分（neural_matrix 和 favorites）
/// 在转换为系统提示时按键排序，确保输出确定性。
/// 这对于测试和可重现性非常重要。
///
/// # 测试场景
///
/// - 输入：包含无序 HashMap 的身份配置
///   - neural_matrix: {"zeta": 0.10, "alpha": 0.90}
///   - favorites: {"snack": "tea", "book": "rust"}
/// - 预期：
///   - 输出按字母顺序排序
///   - "alpha" 出现在 "zeta" 之前
///   - "book" 出现在 "snack" 之前
#[test]
fn aieos_to_system_prompt_sorts_hashmap_sections_for_determinism() {
    // 构建无序的神经矩阵
    let mut neural_matrix = std::collections::HashMap::new();
    neural_matrix.insert("zeta".to_string(), 0.10);
    neural_matrix.insert("alpha".to_string(), 0.90);

    // 构建无序的喜好
    let mut favorites = std::collections::HashMap::new();
    favorites.insert("snack".to_string(), "tea".to_string());
    favorites.insert("book".to_string(), "rust".to_string());

    let identity = AieosIdentity {
        psychology: Some(PsychologySection {
            neural_matrix: Some(neural_matrix),
            ..Default::default()
        }),
        interests: Some(InterestsSection { favorites: Some(favorites), ..Default::default() }),
        ..Default::default()
    };

    let prompt = aieos_to_system_prompt(&identity);

    // 验证排序：alpha 应在 zeta 之前
    let alpha_pos = prompt.find("- alpha: 0.90").unwrap();
    let zeta_pos = prompt.find("- zeta: 0.10").unwrap();
    assert!(alpha_pos < zeta_pos);

    // 验证排序：book 应在 snack 之前
    let book_pos = prompt.find("- book: rust").unwrap();
    let snack_pos = prompt.find("- snack: tea").unwrap();
    assert!(book_pos < snack_pos);
}

/// 测试解析官方 AIEOS 生成器格式的身份配置
///
/// 验证能够正确解析官方 AIEOS 生成器产生的更复杂的 JSON 结构。
/// 官方格式使用了更嵌套的结构（例如 bio 作为对象而非字符串，
/// origin 作为包含嵌套字段的对象等）。
///
/// # 测试场景
///
/// - 输入：符合官方生成器格式的完整 JSON
///   - identity: 包含嵌套的 names、bio（对象）、origin（对象）、residence（对象）
///   - psychology: 包含嵌套的 neural_matrix、traits、moral_compass（对象）
///   - linguistics: 包含嵌套的 text_style、idiolect
///   - motivations: 包含嵌套的 goals、fears（对象）
///   - capabilities: skills 为对象数组
///   - history: 包含嵌套的 education、occupation（对象）
///   - physicality: 包含嵌套的 image_prompts
///   - interests: 包含嵌套的 lifestyle（对象）
/// - 预期：
///   - 成功解析所有嵌套结构
///   - 生成有效的系统提示，包含所有主要信息
///   - 验证关键字段：姓名、MBTI、道德指南针、目标、技能、头像描述等
#[test]
fn normalize_aieos_identity_supports_official_generator_shape() {
    let json = r#"{
            "identity": {
                "names": {
                    "first": "Marta",
                    "last": "Jankowska"
                },
                "bio": {
                    "gender": "Female",
                    "age_biological": 27
                },
                "origin": {
                    "nationality": "Polish",
                    "birthplace": {
                        "city": "Stargard",
                        "country": "Poland"
                    }
                },
                "residence": {
                    "current_city": "Choszczno",
                    "current_country": "Poland"
                }
            },
            "psychology": {
                "neural_matrix": {
                    "creativity": 0.55,
                    "logic": 0.62
                },
                "traits": {
                    "ocean": {
                        "openness": 0.4,
                        "conscientiousness": 0.82
                    },
                    "mbti": "ISFJ"
                },
                "moral_compass": {
                    "alignment": "Lawful Good",
                    "core_values": ["Loyalty", "Helpfulness"],
                    "conflict_resolution_style": "Seeks compromise"
                }
            },
            "linguistics": {
                "text_style": {
                    "formality_level": 0.6,
                    "style_descriptors": ["Sincere", "Grounded"]
                },
                "idiolect": {
                    "catchphrases": ["Stay calm, we can do this"],
                    "forbidden_words": ["severe profanity"]
                }
            },
            "motivations": {
                "core_drive": "Maintain a stable and peaceful life",
                "goals": {
                    "short_term": ["Expand greenhouse"],
                    "long_term": ["Support local community"]
                },
                "fears": {
                    "rational": ["Economic downturn"],
                    "irrational": ["Losing keys in a lake"]
                }
            },
            "capabilities": {
                "skills": [
                    {
                        "name": "Gardening"
                    },
                    {
                        "name": "Community support"
                    }
                ],
                "tools": ["calendar", "messaging"]
            },
            "history": {
                "origin_story": "Moved to Choszczno as a child.",
                "education": {
                    "level": "Associate Degree",
                    "institution": "Local Technical College"
                },
                "occupation": {
                    "title": "Florist",
                    "industry": "Retail"
                }
            },
            "physicality": {
                "image_prompts": {
                    "portrait": "A friendly florist portrait"
                }
            },
            "interests": {
                "hobbies": ["Embroidery", "Walking"],
                "favorites": {
                    "color": "Terracotta"
                },
                "lifestyle": {
                    "diet": "Home-cooked",
                    "sleep_schedule": "10:00 PM - 6:00 AM"
                }
            }
        }"#;

    let payload = serde_json::from_str(json).unwrap();
    let identity = normalize_aieos_identity(&payload);

    // 验证核心身份信息
    let core_identity = identity.identity.clone().unwrap();
    assert_eq!(core_identity.names.unwrap().first.as_deref(), Some("Marta"));
    assert!(core_identity.bio.unwrap().contains("Female"));
    assert!(core_identity.origin.unwrap().contains("Polish"));

    // 验证心理特征
    let psychology = identity.psychology.clone().unwrap();
    assert_eq!(psychology.mbti.as_deref(), Some("ISFJ"));
    assert_eq!(psychology.ocean.unwrap().openness, Some(0.4));
    assert!(psychology.moral_compass.unwrap().contains(&"Alignment: Lawful Good".to_string()));

    // 验证能力
    let capabilities = identity.capabilities.clone().unwrap();
    assert!(capabilities.skills.unwrap().contains(&"Gardening".to_string()));

    // 验证系统提示生成
    let prompt = aieos_to_system_prompt(&identity);
    assert!(prompt.contains("## Identity"));
    assert!(prompt.contains("**MBTI:** ISFJ"));
    assert!(prompt.contains("Alignment: Lawful Good"));
    assert!(prompt.contains("- Expand greenhouse"));
    assert!(prompt.contains("- Gardening"));
    assert!(prompt.contains("A friendly florist portrait"));
}
