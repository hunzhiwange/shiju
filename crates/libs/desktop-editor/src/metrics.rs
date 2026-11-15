//! 编辑器字号预览与提交计算。
use iced_code_editor::CodeEditor;

#[derive(Default)]
pub struct MetricsPreview {
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub generation: u64,
}

impl MetricsPreview {
    /// Advance the debounce token; the host schedules its delivery after 200ms.
    pub fn schedule_commit(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Metrics {
    pub font_size: f32,
    pub line_height: f32,
    pub auto_line_height: bool,
}

impl Metrics {
    pub fn normalized(font_size: f32, line_height: f32, auto_line_height: bool) -> Self {
        let (font_size, line_height) = desktop_config::config_system_settings::normalize_editor_metrics(
            font_size, line_height, auto_line_height,
        );
        Self { font_size: font_size.max(1.0), line_height: line_height.max(1.0), auto_line_height }
    }
}

pub fn displayed(applied: Metrics, preview: &MetricsPreview) -> Metrics {
    let mut displayed = applied;
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

pub struct MetricsCommit {
    pub next: Metrics,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
}

pub fn commit(previous: Metrics, preview: &mut MetricsPreview, auto: Option<bool>) -> Option<MetricsCommit> {
    let mut next = displayed(previous, preview);
    let manual_height = preview.line_height.take();
    preview.font_size = None;

    if let Some(auto) = auto {
        next.auto_line_height = auto;
        if auto {
            next.line_height = next.font_size * 1.4;
        }
    }
    if next == previous {
        return None;
    }

    let font_size = (next.font_size != previous.font_size).then_some(next.font_size);
    // A font-only change in manual mode preserves each editor's existing line height.
    let line_height = (next.auto_line_height
        || manual_height.is_some()
        || next.line_height != previous.line_height)
        .then_some(next.line_height.clamp(10.0, 60.0));

    Some(MetricsCommit { next, font_size, line_height })
}

pub fn apply(editor: &mut CodeEditor, font_size: Option<f32>, line_height: Option<f32>) {
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
