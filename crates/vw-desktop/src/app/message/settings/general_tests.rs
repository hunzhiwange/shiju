// 验证通用设置标签、主题画廊与时区持久化状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::state::GeneralSettingsTab;

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_selection_updates_general_settings_content() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TabSelected(GeneralSettingsTab::Editor)),
    );

    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Editor);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn theme_tab_selection_clears_error_and_refreshes_gallery() {
    let mut app = App::new().0;
    app.themes_settings.error = Some("stale".to_string());

    let task = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TabSelected(GeneralSettingsTab::Themes)),
    );

    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Themes);
    assert_eq!(app.themes_settings.error, None);
    assert!(task.units() > 0);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn timezone_selection_updates_state_and_persists_config() {
    let mut app = App::new().0;
    clear_last_persisted_timezone();

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(
            "America/New_York".to_string(),
        )),
    );

    assert_eq!(app.chat_timezone, "America/New_York");
    assert_eq!(last_persisted_timezone().as_deref(), Some("America/New_York"));
    clear_last_persisted_timezone();
    let original = app.chat_timezone.clone();

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(
            "Mars/Olympus_Mons".to_string(),
        )),
    );

    assert_eq!(app.chat_timezone, original);
    assert_eq!(last_persisted_timezone(), None);
}
