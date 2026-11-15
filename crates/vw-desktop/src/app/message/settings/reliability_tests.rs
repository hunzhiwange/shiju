use super::*;

fn app() -> App { App::new().0 }

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn reliability_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ReliabilityProviderRetriesChanged(3),
        SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(5),
        SettingsMessage::ReliabilitySave,
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
