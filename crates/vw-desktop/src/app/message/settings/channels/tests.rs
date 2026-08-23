// 验证频道顶层更新仅将匹配消息委派给专用模块。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::message::settings::ChannelsMessage;

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn channels_top_level_update_delegates_to_updates_module() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TabSelected(
            crate::app::state::ChannelsSettingsTab::Telegram,
        )),
    );

    assert_eq!(app.channels_settings.active_tab, crate::app::state::ChannelsSettingsTab::Telegram);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn channels_top_level_update_ignores_non_channels_messages() {
    let mut app = App::new().0;
    app.channels_settings.cli = true;
    app.channels_settings.save_error = Some("existing error".to_string());
    let active_tab = app.channels_settings.active_tab;

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.channels_settings.cli);
    assert_eq!(app.channels_settings.save_error.as_deref(), Some("existing error"));
    assert_eq!(app.channels_settings.active_tab, active_tab);
}
