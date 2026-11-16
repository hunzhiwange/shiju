//! Destination and filename prompts for on-demand transcript exports.
//! 本模块负责对话记录导出的目标与文件名提示。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;
use crate::app_event::TranscriptExportDestination;

impl ChatWidget {
    // 处理 `copy_transcript_to_clipboard` 对应的界面事件并同步相关状态。
    pub(crate) fn copy_transcript_to_clipboard(&mut self, markdown: &str) {
        match crate::clipboard_copy::copy_to_clipboard(markdown) {
            Ok(lease) => {
                self.clipboard_lease = lease;
                self.add_info_message(
                    "Copied conversation to clipboard".to_string(),
                    /*hint*/ None,
                );
            }
            Err(error) => self.add_error_message(format!("Copy failed: {error}")),
        }
    }

    // 构造 `show_transcript_export_popup` 对应的展示内容并交给现有界面层。
    pub(super) fn show_transcript_export_popup(&mut self) {
        self.show_selection_view(SelectionViewParams {
            title: Some("Export conversation".to_string()),
            subtitle: Some("Save the complete conversation as Markdown".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items: vec![
                SelectionItem {
                    name: "Copy to clipboard".to_string(),
                    description: Some("Copy the complete Markdown transcript".to_string()),
                    is_disabled: cfg!(target_os = "android"),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::ExportTranscript {
                            destination: TranscriptExportDestination::Clipboard,
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "Save to file".to_string(),
                    description: Some("Choose a Markdown filename".to_string()),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::OpenTranscriptExportFilePrompt);
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        self.defer_input_until_settings_applied();
        self.request_redraw();
    }

    pub(crate) fn show_transcript_export_file_prompt(&mut self) {
        let tx = self.app_event_tx.clone();
        let filename = self.thread_id().map_or_else(
            || "codex-session.md".to_string(),
            |thread_id| format!("codex-session-{thread_id}.md"),
        );
        let view = CustomPromptView::new(
            "Save conversation".to_string(),
            "Type a filename and press Enter".to_string(),
            filename,
            /*context_label*/ None,
            Box::new(move |filename| {
                tx.send(AppEvent::ExportTranscript {
                    destination: TranscriptExportDestination::File(PathBuf::from(filename)),
                });
            }),
        );
        self.bottom_pane.show_text_prompt(view);
        self.request_redraw();
    }
}
