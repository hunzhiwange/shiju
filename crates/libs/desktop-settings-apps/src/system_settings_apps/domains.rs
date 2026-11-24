//! Domain shortcuts reuse the settings navigation dispatcher and its refresh behavior.
#[derive(Clone, Copy)]
pub enum Target { Mcp, Skills, Plugins, General, #[cfg(not(target_arch = "wasm32"))] Auth }
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::Element;

pub fn view<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n, external_sessions: Element<'a, M>, event: impl Fn(Target) -> M + Copy + 'static) -> Element<'a, M> {
    let mut shortcuts = row![].spacing(8);
    for (label, target) in [
        ("MCP", Target::Mcp),
        (i18n.__("技能"), Target::Skills),
        (i18n.__("插件"), Target::Plugins),
        (i18n.__("通用设置"), Target::General),
        #[cfg(not(target_arch = "wasm32"))]
        (i18n.__("授权账号"), Target::Auth),
    ] {
        shortcuts = shortcuts.push(
            button(text(i18n.__(label)).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(event(target)),
        );
    }
    settings_group_card(
        column![
            settings_group_heading(i18n.__("领域导航"), i18n.__("选择领域，或在下方应用卡片中打开对应应用的能力。")),
            shortcuts.wrap(),
            text(i18n.__("供应商与用量：点击应用的“供应商”，在供应商或绑定账号卡片查看用量。"))
                .size(13).style(settings_muted_text_style),
            text(i18n.__("提示词、项目组合与工作区：展开应用详情；入口按应用的领域声明提供。OpenClaw 提供工作区与应用设置，Hermes 提供记忆与面板。"))
                .size(13).style(settings_muted_text_style),
            external_sessions,
            text(i18n.__("页面可达不代表所有应用支持全部操作；安装状态待确认也不代表不支持。权限与具体能力以领域返回结果为准。"))
                .size(13).style(settings_muted_text_style),
        ].spacing(10),
    ).into()
}
