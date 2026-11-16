// 验证可观测性后端、追踪模式及相关文本字段更新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}



#[test]
fn observability_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ObservabilityBackendChanged("log".to_string()),
        SettingsMessage::ObservabilityOtelEndpointChanged(String::new()),
        SettingsMessage::ObservabilityRuntimeTracePathChanged(String::new()),
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
