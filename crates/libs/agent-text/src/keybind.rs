//! 解析、比较并格式化键盘快捷键描述。
//! 模块保持键位字符串和结构化表示之间的转换集中，便于交互层复用。

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// name 字段保存该结构体对外暴露的同名状态。
    pub name: String,
    /// ctrl 字段保存该结构体对外暴露的同名状态。
    pub ctrl: bool,
    /// meta 字段保存该结构体对外暴露的同名状态。
    pub meta: bool,
    /// shift 字段保存该结构体对外暴露的同名状态。
    pub shift: bool,
    /// super_key 字段保存该结构体对外暴露的同名状态。
    pub super_key: bool,
    /// leader 字段保存该结构体对外暴露的同名状态。
    pub leader: bool,
}

/// 执行 matches 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn matches(a: Option<&Info>, b: &Info) -> bool {
    a.is_some_and(|a| a == b)
}

/// 执行 to_string 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn to_string(info: Option<&Info>) -> String {
    let Some(info) = info else { return String::new() };
    let mut parts: Vec<&str> = Vec::new();
    if info.ctrl {
        parts.push("ctrl");
    }
    if info.meta {
        parts.push("alt");
    }
    if info.super_key {
        parts.push("super");
    }
    if info.shift {
        parts.push("shift");
    }
    if !info.name.is_empty() {
        if info.name == "delete" {
            parts.push("del");
        } else {
            parts.push(info.name.as_str());
        }
    }

    let mut result = parts.join("+");
    if info.leader {
        result =
            if result.is_empty() { "<leader>".to_string() } else { format!("<leader> {}", result) };
    }
    result
}

/// 执行 parse 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn parse(key: &str) -> Vec<Info> {
    if key == "none" {
        return Vec::new();
    }
    key.split(',')
        .map(|combo| {
            let normalized = combo.replace("<leader>", "leader+");
            let parts = normalized.to_lowercase();
            let parts = parts.split('+');
            let mut info = Info {
                ctrl: false,
                meta: false,
                shift: false,
                super_key: false,
                leader: false,
                name: String::new(),
            };
            for part in parts {
                match part {
                    "ctrl" => info.ctrl = true,
                    "alt" | "meta" | "option" => info.meta = true,
                    "super" => info.super_key = true,
                    "shift" => info.shift = true,
                    "leader" => info.leader = true,
                    "esc" => info.name = "escape".to_string(),
                    other => info.name = other.to_string(),
                }
            }
            info
        })
        .collect()
}
