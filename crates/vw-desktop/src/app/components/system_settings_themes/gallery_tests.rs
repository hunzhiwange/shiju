// 宿主导航契约：主题页从独立侧栏移入 General，使用真实 SystemTab 枚举。
use crate::app::components::system_settings::SystemTab;
#[test]
fn themes_are_nested_under_general_navigation() {
    assert!(
        !SystemTab::all().any(|tab| tab == SystemTab::Themes),
        "Themes should be nested under General"
    );
    assert_eq!(SystemTab::Themes.to_string(), "主题");
}
