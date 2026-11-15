use super::{I18n, workflow::Feedback};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_13_raw_error_never_becomes_a_template() {
    let raw = Feedback::from("应用名称不能为空 %n %s".to_string());
    assert_eq!(raw.render(I18n::new(DesktopLocale::English)), "应用名称不能为空 %n %s");
}

#[test]
fn desktop_gettext_area_13_local_error_preserves_raw_argument() {
    let message = Feedback::new("节点 data YAML 解析失败: %s", vec!["用户/%n/%s".into()]);
    assert_eq!(
        message.render(I18n::new(DesktopLocale::English)),
        "Failed to parse node data YAML: 用户/%n/%s"
    );
    assert_eq!(
        message.render(I18n::new(DesktopLocale::TraditionalChinese)),
        "節點 data YAML 解析失敗: 用户/%n/%s"
    );
}
