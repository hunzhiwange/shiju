// 验证记忆后端、保留策略与嵌入参数的规范化和钳制。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::MemorySettingsTab;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

fn apply(app: &mut App, message: MemoryMessage) {
    let _ = update(app, SettingsMessage::Memory(message));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn update_ignores_unrelated_messages_and_refresh_clears_only_the_error() {
    let mut app = app();
    app.memory_settings.backend = "postgres".to_string();
    app.memory_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert_eq!(app.memory_settings.backend, "postgres");
    assert_eq!(app.memory_settings.save_error.as_deref(), Some("old"));

    apply(&mut app, MemoryMessage::Refresh);
    assert_eq!(app.memory_settings.backend, "postgres");
    assert!(app.memory_settings.save_error.is_none());
}

#[test]
fn tab_selection_updates_only_visible_memory_section() {
    let mut app = app();
    app.memory_settings.save_error = Some("keep".to_string());

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Embedding));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Embedding);
    assert_eq!(app.memory_settings.save_error.as_deref(), Some("keep"));

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::EmbeddingRoutes);

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Storage));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Storage);

    app.embedding_routes_settings.save_error = Some(("route error".to_string()).into());
    app.embedding_routes_settings.save_success = true;
    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes));
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::EmbeddingRoutes);

    apply(&mut app, MemoryMessage::BackendChanged("qdrant".to_string()));
    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Qdrant);

    apply(&mut app, MemoryMessage::BackendChanged("sqlite".to_string()));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::General);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。






// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。




// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn memory_edits_schedule_persistence_while_refresh_and_unrelated_messages_do_not() {
    let mut app = app();

    let edit_task = update(
        &mut app,
        SettingsMessage::Memory(MemoryMessage::BackendChanged("qdrant".to_string())),
    );
    assert!(has_scheduled_action(edit_task));

    let refresh_task = update(&mut app, SettingsMessage::Memory(MemoryMessage::Refresh));
    assert!(!has_scheduled_action(refresh_task));

    let unrelated_task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert!(!has_scheduled_action(unrelated_task));
}
