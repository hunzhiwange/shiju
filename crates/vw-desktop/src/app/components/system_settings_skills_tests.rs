//! 验证技能设置标签、系统配置、错误提示和帮助浮层的组合行为。
//!
//! 测试遍历全部技能设置标签，并确认错误横幅跟随当前子页而不是全局常驻。

// 宿主契约：多标签页面组合浏览器与配置页，帮助浮层再组合仓库、安装等弹窗；
// 错误横幅及 appearance 测试验证 SettingsMessage 经 App::update 的状态投影。
use super::*;
use crate::app::{App, Message};

fn test_app() -> App {
    App::new().0
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_skills() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::message::settings::SettingsMessage;
    use crate::app::state::SkillsDirectoryScope;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_skills"),
        test_app,
        |scene, app| {
            app.skills_settings.active_tab = SkillsSettingsTab::Skills;
            app.skills_settings.save_error = Some("fixture skills error".into());
            let locale = app.desktop_locale;

            let scope = scene.click(view(app), tr(locale, "全局目录"));
            assert!(matches!(
                scope,
                SettingsMessage::SkillsDirectoryScopeChanged(SkillsDirectoryScope::Global)
            ));
            let _ = app.update(Message::Settings(scope));

            let tab = scene.check(
                app,
                view(app),
                &["技能", "顺序", "系统配置"].map(|label| tr(locale, label)),
                1,
            );
            assert!(matches!(
                tab,
                SettingsMessage::SkillsTabChanged(SkillsSettingsTab::DiscoveryOrder)
            ));
            let _ = app.update(Message::Settings(tab));
            assert_eq!(app.skills_settings.directory_scope, SkillsDirectoryScope::Global);
            let labels = scene.labels(view(app));
            assert!(labels.contains(tr(locale, "发现顺序")));
            assert!(labels.contains("fixture skills error"));
        },
    )
    .await;
}
