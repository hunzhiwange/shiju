//! 延迟提交滑块预览，合并字号与行高的布局更新。

use iced::Task;

use super::{EditorSettingsPatch, persistence};
use crate::app::{App, Message};

pub(crate) use desktop_editor::metrics::{Metrics, MetricsPreview};
use desktop_editor::metrics::apply;
fn applied(app: &App) -> Metrics {
    Metrics {
        font_size: app.current_font_size,
        line_height: app.current_line_height,
        auto_line_height: app.auto_adjust_line_height,
    }
}
pub(crate) fn displayed(app: &App) -> Metrics {
    desktop_editor::metrics::displayed(applied(app), &app.editor_metrics_preview)
}

pub(crate) fn commit(app: &mut App, auto: Option<bool>) -> Task<Message> {
    let previous = applied(app);
    let Some(update) = desktop_editor::metrics::commit(previous, &mut app.editor_metrics_preview, auto) else {
        return Task::none();
    };
    let desktop_editor::metrics::MetricsCommit { next, font_size, line_height } = update;
    apply_to_open_editors(app, font_size, line_height);
    app.current_font_size = next.font_size;
    app.current_line_height = next.line_height;
    app.auto_adjust_line_height = next.auto_line_height;
    let patch = EditorSettingsPatch::metrics(
        app.current_font_size,
        app.current_line_height,
        app.auto_adjust_line_height,
    );
    persistence::enqueue(app, patch)
}

pub(crate) fn apply_snapshot(app: &mut App, size: f32, height: f32, auto: bool) {
    let metrics = Metrics::normalized(size, height, auto);
    app.current_font_size = metrics.font_size;
    app.current_line_height = metrics.line_height;
    app.auto_adjust_line_height = auto;
    apply_to_open_editors(app, Some(app.current_font_size), Some(app.current_line_height));
}

fn apply_to_open_editors(app: &mut App, font_size: Option<f32>, line_height: Option<f32>) {
    for tab in &mut app.preview_tabs {
        apply(&mut tab.editor.inner, font_size, line_height);
    }
    apply(
        &mut app.git.git_copy_modal_code_editor,
        font_size.map(|size| size.clamp(10.0, 30.0)),
        line_height,
    );
}
