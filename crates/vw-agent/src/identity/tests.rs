//! AIEOS 身份配置测试模块
//!
//! 本模块提供了对 AIEOS 身份配置格式的全面测试，包括：
//! - 配置验证测试
//! - 官方生成器格式兼容性测试
//!
//! AIEOS 是一种结构化的身份配置格式，用于定义 AI 代理的名称、性格、
//! 沟通风格、动机和能力等属性。

use super::*;
use crate::app::agent::config::IdentityConfig;
use std::path::PathBuf;

/// 测试模块内部实现
///
/// 包含所有针对 AIEOS 身份功能的单元测试
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::app::agent::config::IdentityConfig;
    use std::path::PathBuf;

    /// 获取测试工作空间目录
    ///
    /// 返回一个临时目录路径，用于测试过程中创建临时文件。
    /// 目录名包含 "vibewindow-test-identity" 以便于识别和清理。
    ///
    /// # 返回值
    ///
    /// 指向临时测试目录的 PathBuf
    fn test_workspace_dir() -> PathBuf {
        std::env::temp_dir().join("vibewindow-test-identity")
    }

    /// 测试 AIEOS 配置验证 - 有路径配置时返回 true
    ///
    /// 验证当配置格式为 "aieos" 且提供了文件路径时，
    /// is_aieos_configured 函数返回 true。
    ///
    /// # 测试场景
    ///
    /// - 输入：format="aieos"，aieos_path=Some("identity.json")
    /// - 预期：is_aieos_configured 返回 true
    #[test]
    fn is_aieos_configured_true_with_path() {
        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: Some("identity.json".into()),
            aieos_inline: None,
        };
        assert!(is_aieos_configured(&config));
    }

    /// 测试 AIEOS 配置验证 - 有内联配置时返回 true
    ///
    /// 验证当配置格式为 "aieos" 且提供了内联 JSON 时，
    /// is_aieos_configured 函数返回 true。
    ///
    /// # 测试场景
    ///
    /// - 输入：format="aieos"，aieos_inline=Some("{\"identity\":{}}")
    /// - 预期：is_aieos_configured 返回 true
    #[test]
    fn is_aieos_configured_true_with_inline() {
        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: None,
            aieos_inline: Some("{\"identity\":{}}".into()),
        };
        assert!(is_aieos_configured(&config));
    }

    /// 测试 AIEOS 配置验证 - 非 AIEOS 格式返回 false
    ///
    /// 验证当配置格式不是 "aieos" 时，
    /// 即使提供了路径，is_aieos_configured 函数也返回 false。
    ///
    /// # 测试场景
    ///
    /// - 输入：format="openclaw"，aieos_path=Some("identity.json")
    /// - 预期：is_aieos_configured 返回 false
    #[test]
    fn is_aieos_configured_false_openclaw_format() {
        let config = IdentityConfig {
            format: "openclaw".into(),
            aieos_path: Some("identity.json".into()),
            aieos_inline: None,
        };
        assert!(!is_aieos_configured(&config));
    }

    /// 测试 AIEOS 配置验证 - 无配置时返回 false
    ///
    /// 验证当配置格式为 "aieos" 但既没有路径也没有内联配置时，
    /// is_aieos_configured 函数返回 false。
    ///
    /// # 测试场景
    ///
    /// - 输入：format="aieos"，aieos_path=None，aieos_inline=None
    /// - 预期：is_aieos_configured 返回 false
    #[test]
    fn is_aieos_configured_false_no_config() {
        let config =
            IdentityConfig { format: "aieos".into(), aieos_path: None, aieos_inline: None };
        assert!(!is_aieos_configured(&config));
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
    fn parse_aieos_identity_supports_official_generator_shape() {
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

        let identity = parse_aieos_identity(json).unwrap();

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

    /// 测试从文件加载 AIEOS 身份配置（支持生成器格式）
    ///
    /// 验证 load_aieos_identity 函数能够从文件系统读取
    /// 符合官方生成器格式的 JSON 配置。
    ///
    /// # 测试场景
    ///
    /// - 创建临时目录和身份配置文件
    /// - 文件内容：符合生成器格式的 JSON（包含嵌套的 bio 和 traits）
    /// - 预期：成功加载并解析文件，字段值正确
    #[test]
    fn load_aieos_identity_from_file_supports_generator_shape() {
        let json = r#"{
                "identity": {
                    "names": { "first": "Nova" },
                    "bio": { "gender": "Non-binary" }
                },
                "psychology": {
                    "traits": { "mbti": "ENTP" },
                    "moral_compass": { "alignment": "Chaotic Good" }
                }
            }"#;

        // 创建临时目录和文件
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("identity.json");
        std::fs::write(&path, json).unwrap();

        let config = IdentityConfig {
            format: "aieos".into(),
            aieos_path: Some("identity.json".into()),
            aieos_inline: None,
        };

        let identity = load_aieos_identity(&config, temp.path()).unwrap().unwrap();
        assert_eq!(identity.identity.unwrap().names.unwrap().first.as_deref(), Some("Nova"));
        assert_eq!(identity.psychology.unwrap().mbti.as_deref(), Some("ENTP"));
    }

}
