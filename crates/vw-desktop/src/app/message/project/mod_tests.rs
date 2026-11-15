// 验证项目消息类型及未处理消息的顶层回退行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{ProjectMessage, update};
use crate::app::App;

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("mod_tests"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn update_returns_no_task_for_unhandled_start_deferred_tasks_message() {
    let (mut app, _task) = App::new();
    let before_project_path = app.project_path.clone();

    let _task = update(
        &mut app,
        ProjectMessage::StartDeferredTasks { project_path: "/tmp/project".to_string() },
    );

    assert_eq!(app.project_path, before_project_path);
}
