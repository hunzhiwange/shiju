//! Scout 模块单元测试
//!
//! 本模块包含 Scout 技能发现子系统的各种测试用例，验证以下功能：
//! - 技能来源（ScoutSource）的字符串解析
//! - 搜索结果去重功能
//! - GitHub API 响应解析
//! - URL 编码工具函数

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 ScoutSource 枚举从字符串解析的功能
    ///
    /// 验证各种来源标识符能够正确解析为对应的枚举值：
    /// - "github" 和 "GitHub" 应解析为 GitHub
    /// - "clawhub" 应解析为 ClawHub
    /// - "huggingface" 和 "hf" 应解析为 HuggingFace
    /// - 未知来源应回退到 GitHub
    #[test]
    fn scout_source_from_str() {
        // 小写 github 应正确解析
        assert_eq!("github".parse::<ScoutSource>().unwrap(), ScoutSource::GitHub);
        // 大小写混合应正确解析
        assert_eq!("GitHub".parse::<ScoutSource>().unwrap(), ScoutSource::GitHub);
        // ClawHub 来源
        assert_eq!("clawhub".parse::<ScoutSource>().unwrap(), ScoutSource::ClawHub);
        // HuggingFace 完整名称
        assert_eq!("huggingface".parse::<ScoutSource>().unwrap(), ScoutSource::HuggingFace);
        // HuggingFace 缩写
        assert_eq!("hf".parse::<ScoutSource>().unwrap(), ScoutSource::HuggingFace);
        // 未知来源回退到 GitHub
        assert_eq!("unknown".parse::<ScoutSource>().unwrap(), ScoutSource::GitHub);
    }

    /// 测试 dedup 函数去除重复搜索结果的功能
    ///
    /// 验证当多个搜索结果具有相同 URL 时，只保留第一个出现的条目。
    /// 测试场景：
    /// - 第一个和第二个条目具有相同 URL（应去重）
    /// - 第三个条目 URL 不同（应保留）
    /// - 最终应只剩 2 个结果
    #[test]
    fn dedup_removes_duplicates() {
        // 构造测试数据：包含重复 URL 的搜索结果列表
        let mut results = vec![
            // 第一个结果：名称 "a"
            ScoutResult {
                name: "a".into(),
                url: "https://github.com/x/a".into(),
                description: String::new(),
                stars: 10,
                language: None,
                updated_at: None,
                source: ScoutSource::GitHub,
                owner: "x".into(),
                has_license: true,
            },
            // 第二个结果：名称不同但 URL 与第一个相同（应被去重）
            ScoutResult {
                name: "a-dup".into(),
                url: "https://github.com/x/a".into(),
                description: String::new(),
                stars: 10,
                language: None,
                updated_at: None,
                source: ScoutSource::GitHub,
                owner: "x".into(),
                has_license: true,
            },
            // 第三个结果：URL 不同（应保留）
            ScoutResult {
                name: "b".into(),
                url: "https://github.com/x/b".into(),
                description: String::new(),
                stars: 5,
                language: None,
                updated_at: None,
                source: ScoutSource::GitHub,
                owner: "x".into(),
                has_license: false,
            },
        ];

        // 执行去重操作
        dedup(&mut results);

        // 验证结果数量：应从 3 个减少到 2 个
        assert_eq!(results.len(), 2);
        // 验证第一个结果保留（名称 "a"）
        assert_eq!(results[0].name, "a");
        // 验证第三个结果保留（名称 "b"）
        assert_eq!(results[1].name, "b");
    }

    /// 测试 GitHub API 响应的 JSON 解析功能
    ///
    /// 验证 GitHubScout::parse_items 能够正确解析 GitHub 搜索 API 返回的 JSON 格式：
    /// - 解析仓库名称、描述、星标数
    /// - 解析编程语言和更新时间
    /// - 解析所有者信息和许可证状态
    #[test]
    fn parse_github_items() {
        // 构造模拟的 GitHub API 响应 JSON
        let json = serde_json::json!({
            "total_count": 1,
            "items": [
                {
                    "name": "cool-skill",                          // 仓库名称
                    "html_url": "https://github.com/user/cool-skill",  // 仓库 URL
                    "description": "A cool skill",                  // 描述
                    "stargazers_count": 42,                         // 星标数
                    "language": "Rust",                             // 编程语言
                    "updated_at": "2026-01-15T10:00:00Z",          // 更新时间
                    "owner": { "login": "user" },                   // 所有者
                    "license": { "spdx_id": "MIT" }                 // 许可证
                }
            ]
        });

        // 调用解析函数
        let items = GitHubScout::parse_items(&json);

        // 验证解析结果数量
        assert_eq!(items.len(), 1);
        // 验证仓库名称
        assert_eq!(items[0].name, "cool-skill");
        // 验证星标数
        assert_eq!(items[0].stars, 42);
        // 验证许可证标记
        assert!(items[0].has_license);
        // 验证所有者
        assert_eq!(items[0].owner, "user");
    }

    /// 测试 URL 编码函数的正确性
    ///
    /// 验证 urlencoding 函数能够正确处理：
    /// - 空格字符转换为加号
    /// - 特殊字符（&、# 等）转换为百分号编码
    #[test]
    fn urlencoding_works() {
        // 空格应转换为加号
        assert_eq!(urlencoding("hello world"), "hello+world");
        // 特殊字符 & 和 # 应转换为百分号编码
        assert_eq!(urlencoding("a&b#c"), "a%26b%23c");
    }
}

#[test]
fn parse_items_uses_defaults_and_skips_incomplete_entries() {
    let json = serde_json::json!({
        "items": [
            {
                "name": "minimal",
                "html_url": "https://github.com/user/minimal",
                "description": null,
                "stargazers_count": null,
                "language": null,
                "updated_at": "not-a-date",
                "owner": {},
                "license": null
            },
            {
                "name": "missing-url"
            }
        ]
    });

    let items = GitHubScout::parse_items(&json);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "minimal");
    assert_eq!(items[0].description, "");
    assert_eq!(items[0].stars, 0);
    assert!(items[0].language.is_none());
    assert!(items[0].updated_at.is_none());
    assert_eq!(items[0].owner, "unknown");
    assert!(!items[0].has_license);
}

#[test]
fn parse_items_returns_empty_without_items_array() {
    assert!(GitHubScout::parse_items(&serde_json::json!({})).is_empty());
    assert!(GitHubScout::parse_items(&serde_json::json!({"items": "nope"})).is_empty());
}

#[test]
fn github_scout_uses_default_queries() {
    let scout = GitHubScout::new(None);

    assert_eq!(scout.queries, vec!["vibewindow skill", "ai agent skill"]);
}

#[test]
fn github_scout_accepts_valid_token_without_changing_queries() {
    let scout = GitHubScout::new(Some("test-token".to_string()));

    assert_eq!(scout.queries, vec!["vibewindow skill", "ai agent skill"]);
}

#[test]
fn github_scout_rejects_invalid_header_value_without_panicking() {
    let scout = GitHubScout::new(Some("line\nbreak".to_string()));

    assert_eq!(scout.queries, vec!["vibewindow skill", "ai agent skill"]);
}
