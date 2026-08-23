//! 原始用量数据弹窗的测试，覆盖关闭时不渲染和打开时构建内容。
//! 该组固定弹窗显隐契约，详细载荷状态由步骤面板测试覆盖。
use crate::app::App;
use crate::app::state::UsageStepRawModal;

#[test]
fn view_returns_none_when_modal_closed() {
    let (app, _task) = App::new();
    assert!(super::view(&app).is_none());
}

#[test]
fn view_builds_when_modal_open() {
    let (mut app, _task) = App::new();
    app.usage_step_raw_modal =
        Some(UsageStepRawModal { step_index: 3, content: r#"{"messages":[]}"#.to_string() });

    assert!(super::view(&app).is_some());
}
