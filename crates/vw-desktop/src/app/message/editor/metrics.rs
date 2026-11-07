//! 延迟提交滑块预览，合并字号与行高的布局更新。

use std::time::Duration;

use iced::Task;
use iced_code_editor::CodeEditor;

use super::{EditorMessage, EditorSettingsPatch, persistence};
use crate::app::{App, Message};

#[derive(Default)]
pub(crate) struct MetricsPreview {
    pub(super) font_size: Option<f32>,
    pub(super) line_height: Option<f32>,
    pub(super) generation: u64,
}

impl MetricsPreview {
    pub(super) fn schedule_commit(&mut self) -> Task<Message> {
        self.generation = self.generation.wrapping_add(1);
        crate::app::message::after(
            Duration::from_millis(200),
            Message::Editor(EditorMessage::CommitMetricsDue(self.generation)),
        )
    }
}

#[derive(PartialEq)]
pub(crate) struct Metrics {
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) auto_line_height: bool,
}

impl Metrics {
    fn applied(app: &App) -> Self {
        Self {
            font_size: app.current_font_size,
            line_height: app.current_line_height,
            auto_line_height: app.auto_adjust_line_height,
        }
    }
}

pub(crate) fn displayed(app: &App) -> Metrics {
    let mut displayed = Metrics::applied(app);
    let preview = &app.editor_metrics_preview;
    if let Some(size) = preview.font_size {
        displayed.font_size = size;
        if displayed.auto_line_height {
            displayed.line_height = size * 1.4;
        }
    }
    if let Some(height) = preview.line_height {
        displayed.line_height = height;
        displayed.auto_line_height = false;
    }
    displayed
}

pub(crate) fn commit(app: &mut App, auto: Option<bool>) -> Task<Message> {
    let previous = Metrics::applied(app);
    let mut next = displayed(app);
    let preview = &mut app.editor_metrics_preview;
    let manual_height = preview.line_height.take();
    preview.font_size = None;

    if let Some(auto) = auto {
        next.auto_line_height = auto;
        if auto {
            next.line_height = next.font_size * 1.4;
        }
    }
    if next == previous {
        return Task::none();
    }

    let font_size = (next.font_size != previous.font_size).then_some(next.font_size);
    // A font-only change in manual mode preserves each editor's existing line height.
    let line_height = (next.auto_line_height
        || manual_height.is_some()
        || next.line_height != previous.line_height)
        .then_some(next.line_height.clamp(10.0, 60.0));

    for tab in &mut app.preview_tabs {
        apply(&mut tab.editor.inner, font_size, line_height);
    }
    apply(
        &mut app.git_copy_modal_code_editor,
        font_size.map(|size| size.clamp(10.0, 30.0)),
        line_height,
    );

    app.current_font_size = next.font_size;
    app.current_line_height = next.line_height;
    app.auto_adjust_line_height = next.auto_line_height;
    persistence::enqueue(app, EditorSettingsPatch::metrics(app))
}

fn apply(editor: &mut CodeEditor, font_size: Option<f32>, line_height: Option<f32>) {
    if let Some(size) = font_size
        && size != editor.font_size()
    {
        // The library's proportional height differs from the desktop's 1.4 ratio.
        editor.set_font_size(size, false);
    }
    if let Some(height) = line_height
        && height != editor.line_height()
    {
        editor.set_line_height(height);
    }
}
