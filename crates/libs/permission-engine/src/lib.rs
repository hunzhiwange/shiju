//! 无状态的权限配置解析、规则合并、求值与工具禁用判定。

pub use vw_shared::permission::{Action, Rule, Ruleset};

use glob::Pattern;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;

/// 通配符模式匹配
///
/// 使用 glob 模式进行文本匹配。
///
/// # 参数
///
/// * `text` - 待匹配的文本
/// * `pat` - glob 模式字符串
///
/// # 返回值
///
/// 如果模式有效且匹配成功，返回 `true`；否则返回 `false`。
///
/// # 示例
///
/// ```ignore
/// assert!(wildcard_match("src/main.rs", "src/**/*.rs"));
/// assert!(wildcard_match("file.txt", "*"));
/// assert!(!wildcard_match("file.rs", "*.txt"));
/// ```
pub fn wildcard_match(text: &str, pat: &str) -> bool {
    Pattern::new(pat).map(|p| p.matches(text)).unwrap_or(false)
}

/// 从配置值解析权限规则集
///
/// 将 JSON 格式的权限配置转换为 `Ruleset`。
///
/// # 参数
///
/// * `permission` - 权限配置的 JSON 值
/// * `expand` - 应用提供的路径模式展开函数
///
/// # 返回值
///
/// 返回解析后的规则集合。
///
/// # 配置格式
///
/// 支持两种配置格式：
///
/// 1. 简单格式：对所有模式应用相同动作
/// ```json
/// {
///   "read": "allow",
///   "bash": "ask"
/// }
/// ```
///
/// 2. 详细格式：对不同模式应用不同动作
/// ```json
/// {
///   "edit": {
///     "src/**": "allow",
///     "~/.ssh/*": "deny"
///   }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let config = json!({
///     "edit": { "src/**": "allow" },
///     "read": "allow",
/// });
/// let rules = from_config(&config, str::to_string);
/// assert_eq!(rules.len(), 2);
/// ```
pub fn from_config(permission: &Value, expand: impl Fn(&str) -> String) -> Ruleset {
    let mut out = Vec::new();
    let Some(obj) = permission.as_object() else {
        return out;
    };

    for (key, value) in obj {
        match value {
            // 简单格式：对所有模式应用相同动作
            Value::String(s) => {
                let action = parse_action(s);
                out.push(Rule { permission: key.to_string(), action, pattern: "*".to_string() });
            }
            // 详细格式：对不同模式应用不同动作
            Value::Object(map) => {
                for (pattern, action) in map {
                    if let Some(action) = action.as_str().map(parse_action) {
                        out.push(Rule {
                            permission: key.to_string(),
                            pattern: expand(pattern),
                            action,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// 解析动作字符串
///
/// 将字符串转换为 `Action` 枚举值。
///
/// # 参数
///
/// * `s` - 动作字符串（"allow"、"deny" 或其他）
///
/// # 返回值
///
/// - "allow" → `Action::Allow`
/// - "deny" → `Action::Deny`
/// - 其他 → `Action::Ask`（默认值）
fn parse_action(s: &str) -> Action {
    match s.trim() {
        "allow" => Action::Allow,
        "deny" => Action::Deny,
        _ => Action::Ask,
    }
}

/// 合并多个规则集
///
/// 将多个规则集合并为一个扁平的规则列表。
///
/// # 参数
///
/// * `rulesets` - 规则集切片
///
/// # 返回值
///
/// 返回合并后的规则集，保持原有顺序。
///
/// # 示例
///
/// ```ignore
/// let rules1 = vec![Rule { permission: "read".into(), pattern: "*".into(), action: Action::Allow }];
/// let rules2 = vec![Rule { permission: "edit".into(), pattern: "*".into(), action: Action::Ask }];
/// let merged = merge(&[rules1, rules2]);
/// assert_eq!(merged.len(), 2);
/// ```
pub fn merge(rulesets: &[Ruleset]) -> Ruleset {
    rulesets.iter().flat_map(|r| r.iter().cloned()).collect()
}

/// 评估权限
///
/// 根据给定的权限标识符和目标模式，在规则集中查找匹配的规则。
///
/// # 参数
///
/// * `permission` - 权限标识符（如 "edit"、"read"）
/// * `pattern` - 目标模式（如 "src/main.rs"）
/// * `rulesets` - 规则集数组
///
/// # 返回值
///
/// 返回匹配的规则。如果没有匹配，返回默认的 Ask 规则。
///
/// # 评估逻辑
///
/// 1. 合并所有规则集
/// 2. 从后向前查找匹配的规则（后定义的优先级更高）
/// 3. 规则匹配条件：权限和模式都必须匹配
/// 4. 没有匹配时返回默认 Ask 规则
///
/// # 示例
///
/// ```ignore
/// let rules = vec![
///     Rule { permission: "edit".into(), pattern: "*".into(), action: Action::Ask },
///     Rule { permission: "edit".into(), pattern: "src/**".into(), action: Action::Allow },
/// ];
///
/// let rule = evaluate("edit", "src/main.rs", &[rules]);
/// assert_eq!(rule.action, Action::Allow);  // 后定义的规则优先
/// ```
pub fn evaluate(permission: &str, pattern: &str, rulesets: &[Ruleset]) -> Rule {
    let merged = merge(rulesets);

    // 从后向前查找第一个匹配的规则
    if let Some(rule) = merged
        .iter()
        .rev()
        .find(|r| wildcard_match(permission, &r.permission) && wildcard_match(pattern, &r.pattern))
    {
        return rule.clone();
    }

    // 没有匹配时返回默认的 Ask 规则
    Rule { action: Action::Ask, permission: permission.to_string(), pattern: "*".to_string() }
}

/// 编辑类工具名称集合
///
/// 这些工具名称在权限检查时会被映射到 "edit" 权限。
static EDIT_TOOLS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["edit", "write", "patch"].into_iter().collect());

/// 获取被禁用的工具列表
///
/// 根据规则集判断哪些工具应该被禁用。
///
/// # 参数
///
/// * `tools` - 工具名称列表
/// * `ruleset` - 权限规则集
///
/// # 返回值
///
/// 返回被禁用的工具名称集合。
///
/// # 禁用条件
///
/// 工具被禁用的条件是：
/// 1. 存在匹配该工具的规则
/// 2. 规则的 pattern 为 "*"（全局规则）
/// 3. 规则的 action 为 Deny
///
/// # 特殊处理
///
/// 编辑类工具（edit、write、patch）统一映射到 "edit" 权限。
///
/// # 示例
///
/// ```ignore
/// let tools = vec!["edit".into(), "read".into(), "bash".into()];
/// let ruleset = vec![
///     Rule { permission: "edit".into(), pattern: "*".into(), action: Action::Deny },
/// ];
/// let disabled = disabled(&tools, &ruleset);
/// assert!(disabled.contains("edit"));
/// assert!(!disabled.contains("read"));
/// ```
pub fn disabled(tools: &[String], ruleset: &Ruleset) -> HashSet<String> {
    let mut result = HashSet::new();

    for tool in tools {
        // 编辑类工具统一映射到 "edit" 权限
        let permission = if EDIT_TOOLS.contains(tool.as_str()) { "edit" } else { tool.as_str() };

        // 从后向前查找匹配的规则
        let rule = ruleset.iter().rev().find(|r| wildcard_match(permission, &r.permission));

        if let Some(rule) = rule {
            // 只有全局拒绝规则才会禁用工具
            if rule.pattern == "*" && rule.action == Action::Deny {
                result.insert(tool.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests;
