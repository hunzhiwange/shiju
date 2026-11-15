use super::*;

fn app() -> App { App::new().0 }

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn valid_runtime_settings_schedule_persistence_task() {
    let mut app = app();
    app.runtime_settings.docker_memory_limit_mb_input = String::new();
    app.runtime_settings.docker_cpu_limit_input = "1.25".to_string();
    app.runtime_settings.reasoning_enabled_input = "auto".to_string();
    app.runtime_settings.reasoning_level_input = "high".to_string();

    let task = update(&mut app, SettingsMessage::Runtime(RuntimeMessage::DockerCpuLimitChanged("1.25".to_string())));

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}
