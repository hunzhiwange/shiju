//! 系统设置中技能管理页面的浏览、目录或帮助视图。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::{App, Message, message};
use iced::Element;

pub(super) use desktop_settings_skills::help::help_text;

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view_overlays<'a>(
    app: &'a App,
    dialog: Element<'a, Message>,
) -> Element<'a, Message> {
    let s = &app.skills_settings;
    if !s.show_help_modal {
        return dialog;
    }

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        crate::app::i18n::I18n::new(app.desktop_locale).__("Skills 配置帮助"),
        help_text(app.desktop_locale),
        Message::Settings(message::SettingsMessage::SkillsHelpClose),
    )
}
