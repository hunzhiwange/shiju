#[derive(Clone, Copy)]
pub enum Target { Mcp, Skills, Plugins, General, #[cfg(not(target_arch = "wasm32"))] Auth }
//! Domain shortcuts reuse the settings navigation dispatcher and its refresh behavior.
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::Element;

pub fn view<'a, M: Clone + 'a>(external_sessions: Element<'a, M>, event: impl Fn(Target) -> M + Copy + 'a) -> Element<'a, M> {
    let mut shortcuts = row![].spacing(8);
    for (label, target) in [
        ("MCP", Target::Mcp),
        ("技能", Target::Skills),
        ("插件", Target::Plugins),
        ("通用设置", Target::General),
        #[cfg(not(target_arch = "wasm32"))]
        ("授权账号", Target::Auth),
    ] {
        shortcuts = shortcuts.push(
            button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(event(target)),
        );
    }
    settings_group_card(
        column![
            settings_group_heading("领域导航", "选择领域，或在下方应用卡片中打开对应应用的能力。"),
            shortcuts.wrap(),
            text("供应商与用量：点击应用的“供应商”，在供应商或绑定账号卡片查看用量。")
                .size(13).style(settings_muted_text_style),
            text("提示词、项目组合与工作区：展开应用详情；入口按应用的领域声明提供。OpenClaw 提供工作区与应用设置，Hermes 提供记忆与面板。")
                .size(13).style(settings_muted_text_style),
            external_sessions,
            text("配置备份与恢复、WebDAV/S3 设备同步：尚未接入对应桌面页面，等待领域接口接线。")
                .size(13).style(settings_muted_text_style),
            text("页面可达不代表所有应用支持全部操作；安装状态待确认也不代表不支持。权限与具体能力以领域返回结果为准。")
                .size(13).style(settings_muted_text_style),
        ].spacing(10),
    ).into()
}
