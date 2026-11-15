//! AIEOS 身份配置测试模块
//!
//! 本模块提供了对 AIEOS 身份配置格式的全面测试，包括：
//! - 配置验证测试
//! - 官方生成器格式兼容性测试
//!
//! AIEOS 是一种结构化的身份配置格式，用于定义 AI 代理的名称、性格、
//! 沟通风格、动机和能力等属性。

use super::*;
use vw_config_types::security::IdentityConfig;

/// 测试模块内部实现
///
/// 包含所有针对 AIEOS 身份功能的单元测试
#[allow(dead_code)]
mod tests {
    use super::*;

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
