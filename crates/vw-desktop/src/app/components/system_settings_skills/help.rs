//! 系统设置中技能管理页面的浏览、目录或帮助视图。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::{App, Message, message};
use iced::Element;

pub(super) fn help_text() -> &'static str {
    r#"技能配置说明

一、作用
- skills 用于控制技能系统加载来源与系统提示词注入方式。
- 当前页面上半部分保留配置项，下半部分通过 gateway service 展示技能目录与来源。
- 该配置影响上下文长度、技能可见性和本地 open-skills 仓库行为。

二、页面结构
1) 技能
- 技能页支持“项目目录 / 全部目录”切换，并按项目目录、父级目录、全局目录、内置技能分层显示。
- 点击左侧技能卡片后，会弹出详情窗口展示对应 `SKILL.md`，若没有则回退显示 `SKILL.toml`。
- 对本地技能可直接执行启用、禁用、删除；对内置技能可在打开项目后安装到当前项目目录。

2) 插件
- 当前为占位页，暂不提供具体内容。

3) 系统配置
- 项目技能默认加载、社区技能、仓库目录、注入模式配置集中在这里。
- 这些配置只影响社区仓库同步与提示注入，不会移除内置技能列表。

三、字段含义
1) auto_load_project_skills
- 类型：布尔（true / false）
- true（默认）：委托代理自动加载项目及父级 `skills`、`.agents/skills`。
- false：委托代理仅加载自身 `allowed_skills` 明确选择的技能。

2) open_skills_enabled
- 类型：布尔（true / false）
- true：启用并同步社区 open-skills 仓库（默认目录见下文）。
- false：不启用社区仓库，仅使用内置技能。

3) open_skills_dir
- 类型：字符串或 null
- 含义：本地 open-skills 仓库路径。
- 为空时，启用状态下默认使用 `$HOME/open-skills`。

4) prompt_injection_mode
- 类型："compact" | "full"
- compact（默认）：仅注入技能元信息，按需加载，节省上下文。
- full：注入完整技能文本，兼容旧行为，但会增加上下文占用。

四、示例
{
  "skills": {
    "auto_load_project_skills": true,
    "open_skills_enabled": false,
    "open_skills_dir": null,
    "prompt_injection_mode": "compact"
  }
}

五、启用/禁用说明
- 本地技能通过技能目录下的 `SKILL.disabled` 标记文件控制启用状态。
- 禁用后仍会在目录页显示，但运行时会跳过加载，便于后续重新启用。
"#
}

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
        "Skills 配置帮助",
        help_text(),
        Message::Settings(message::SettingsMessage::SkillsHelpClose),
    )
}
