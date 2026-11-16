// 宿主保留项目元数据保存及 Iced 延迟任务与截止时间回写契约；间隔边界在 desktop-task-board 独立测试。
// 验证任务看板设置持久化、期限计算与边界清理。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn save_settings_without_project_leaves_recent_projects_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = None;
    app.recent_projects_meta.clear();

    save_settings(&mut app);

    assert!(app.recent_projects_meta.is_empty());
}

#[test]
fn save_settings_updates_existing_project_meta() {
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/tmp/project-a".to_string());
    app.task_board_settings.max_concurrent = 7;
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/tmp/project-a".to_string(),
        name: "Project A".to_string(),
        task_board_settings: None,
        session_auto_refresh: false,
        session_refresh_interval_seconds: 10,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];

    save_settings(&mut app);

    let saved =
        app.recent_projects_meta[0].task_board_settings.as_ref().expect("settings should be saved");
    assert_eq!(saved.max_concurrent, 7);
    assert_eq!(app.recent_projects_meta[0].name, "Project A");
}

#[test]
fn save_settings_creates_project_meta_for_new_project_path() {
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/tmp/project-b".to_string());
    app.recent_projects_meta.clear();
    app.task_board_settings.max_concurrent = 3;

    save_settings(&mut app);

    assert_eq!(app.recent_projects_meta.len(), 1);
    let meta = &app.recent_projects_meta[0];
    assert_eq!(meta.path, "/tmp/project-b");
    assert_eq!(meta.name, "project-b");
    assert_eq!(
        meta.task_board_settings.as_ref().expect("settings should be saved").max_concurrent,
        3
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

#[test]
fn standalone_schedulers_produce_deferred_actions() {
    let (app, _task) = crate::app::App::new();

    for task in
        [schedule_scheduler_tick(&app), schedule_auto_review_tick(), schedule_auto_merge_tick()]
    {
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
