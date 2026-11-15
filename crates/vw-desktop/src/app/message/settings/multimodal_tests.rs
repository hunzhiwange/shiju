// 验证多模态模型选择、OCR 前置条件与图像限制边界。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.multimodal_settings.max_images = 7;
    app.multimodal_settings.max_image_size_mb = 12;
    app.multimodal_settings.allow_remote_fetch = true;
    app.multimodal_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::SchedulerHelpOpen);

    assert_eq!(app.multimodal_settings.max_images, 7);
    assert_eq!(app.multimodal_settings.max_image_size_mb, 12);
    assert!(app.multimodal_settings.allow_remote_fetch);
    assert_eq!(app.multimodal_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn multimodal_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        MultimodalMessage::MaxImagesChanged(4),
        MultimodalMessage::MaxImageSizeMbChanged(8),
        MultimodalMessage::AllowRemoteFetchToggled(true),
        MultimodalMessage::ModelSupportsImagesToggled(true),
        MultimodalMessage::OcrModelChanged("provider/model".to_string()),
        MultimodalMessage::OcrEnabledToggled(true),
    ] {
        let task = update(&mut app, SettingsMessage::Multimodal(message));
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
