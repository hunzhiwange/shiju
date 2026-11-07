//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

pub const SETTINGS_LABEL_WIDTH: f32 = 232.0;
/// `SETTINGS_CONTROL_TEXT_SIZE` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub const SETTINGS_CONTROL_TEXT_SIZE: f32 = 14.0;
/// `SETTINGS_CONTROL_PADDING` 常量，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub const SETTINGS_CONTROL_PADDING: [u16; 2] = [12, 14];

/// 设置行使用默认字体，支持中英文混排及换行。
pub fn settings_field_label(label: &str) -> iced::widget::Text<'_> {
    iced::widget::text(label).size(15)
}

/// 与常规设置行使用相同尺寸和主题的开关。
pub fn settings_toggle<'a, Message: 'a>(checked: bool) -> iced::widget::Toggler<'a, Message> {
    iced::widget::toggler(checked).size(22).style(super::styles::settings_toggle_style)
}

/// 构建或处理 `url_encode` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b as char {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(*b as char),
            _ => result.push_str(&format!("%{:02X}", *b)),
        }
    }
    result
}

/// 构建或处理 `bool_support_label` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn bool_support_label(v: bool) -> &'static str {
    if v { "支持" } else { "不支持" }
}

/// 构建或处理 `format_context_limit` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn format_context_limit(v: u64) -> String {
    if v >= 1024 && v.is_multiple_of(1024) {
        return format!("{}K", v / 1024);
    }
    if v >= 1000 && v.is_multiple_of(1000) {
        return format!("{}K", v / 1000);
    }
    v.to_string()
}
