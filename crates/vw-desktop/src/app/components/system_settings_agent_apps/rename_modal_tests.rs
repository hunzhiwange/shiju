use super::*;

#[test]
fn cc_base_icon_desktop_rename_modal_chinese_input() {
    let form = crate::app::state::IconRenameState {
        id: "icon".into(),
        name: "中文图标".into(),
        saving: false,
        error: Some("名称过长".into()),
        context: crate::app::state::AgentAppsContext {
            gateway: "http://localhost".into(),
            tenant: vw_gateway_client::vw_api_types::id::TenantScope::new("p", "c").unwrap(),
            project: None,
        },
        generation: 1,
    };
    let _: iced::Element<'_, Message> = rename_modal::view(
        &form,
        I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &Theme::Light,
        iced::widget::text("base").into(),
    );
}
