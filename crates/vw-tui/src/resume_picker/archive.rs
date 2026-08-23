//! 管理会话选择器中的归档、恢复请求及其异步结果。

use codex_protocol::ThreadId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::LoadTrigger;
use super::PickerLoadRequest;
use super::PickerState;
use super::Row;
use super::SearchState;
use super::SessionPickerAction;
use super::SessionPickerLaunchContext;
use super::SessionSelection;
use super::SessionStatus;
use super::SessionTarget;
use crate::keymap::KeymapContext;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ArchiveState {
    // 每次只允许一个归档或恢复操作，thread_id 用于丢弃过期异步结果。
    #[default]
    Idle,
    Pending {
        thread_id: ThreadId,
    },
    Restoring {
        thread_id: ThreadId,
    },
}

impl PickerState {
    pub(super) fn archive_shortcut_available(&self) -> bool {
        // Ctrl+A 仅在恢复视图且未被单键或列表组合键前缀占用时开放。
        if !matches!(self.action, SessionPickerAction::Resume)
            || self.status == SessionStatus::Archived
        {
            return false;
        }

        let archive_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        self.list_keymap.action_for(archive_key).is_none()
            && !self.chord_keymap.bindings.iter().any(|binding| {
                binding.action.context == KeymapContext::List
                    && binding.chord.prefix.is_press(archive_key)
            })
    }

    pub(super) fn request_archive_for_selected_session(&mut self) {
        // 当前会话不能从选择器内归档，因为该操作还需要退出当前运行会话。
        if !matches!(self.archive_state, ArchiveState::Idle) {
            return;
        }

        let Some(row) = self.filtered_rows.get(self.selected) else {
            return;
        };
        let Some(thread_id) = row.thread_id else {
            self.inline_error = Some(String::from("Selected session does not have a thread ID."));
            self.request_frame();
            return;
        };

        if matches!(
            self.launch_context,
            SessionPickerLaunchContext::ExistingSession {
                current_thread_id: Some(current_thread_id)
            } if current_thread_id == thread_id
        ) {
            self.inline_error = Some(String::from(
                "Use /archive to archive the current session and exit.",
            ));
            self.request_frame();
            return;
        }

        self.archive_state = ArchiveState::Pending { thread_id };
        self.request_frame();
        (self.picker_loader)(PickerLoadRequest::Archive { thread_id });
    }

    pub(super) fn handle_archive_result(
        &mut self,
        thread_id: ThreadId,
        result: std::io::Result<()>,
    ) {
        // 只接受与当前 Pending 状态匹配的结果，旧请求不得覆盖新操作状态。
        if self.archive_state != (ArchiveState::Pending { thread_id }) {
            return;
        }
        self.archive_state = ArchiveState::Idle;

        if let Err(error) = result {
            self.inline_error = Some(format!("Failed to archive session: {error}"));
            self.request_frame();
            return;
        }

        let selected_key = self
            .filtered_rows
            .get(self.selected)
            .filter(|row| row.thread_id != Some(thread_id))
            .and_then(Row::seen_key);

        self.all_rows.retain(|row| row.thread_id != Some(thread_id));
        // Keep the seen-row key as a tombstone so an older page that is already
        // in flight cannot put the archived session back into the picker.
        // 保留已见行键作为墓碑，防止已经在途的旧分页把刚归档的会话重新放回选择器。
        self.transcript_previews.remove(&thread_id);
        self.transcript_cells.remove(&thread_id);
        if self.expanded_thread_id == Some(thread_id) {
            self.expanded_thread_id = None;
        }
        if self.pending_transcript_open == Some(thread_id) {
            self.pending_transcript_open = None;
            if let Some(cancellation) = self.pending_transcript_cancellation.take() {
                let _ = cancellation.send(());
            }
            self.transcript_loading_frame_shown = false;
        }
        self.inline_error = None;
        self.apply_filter();

        if let Some(selected_key) = selected_key
            && let Some(selected) = self
                .filtered_rows
                .iter()
                .position(|row| row.seen_key().as_ref() == Some(&selected_key))
        {
            self.selected = selected;
            self.ensure_selected_visible();
        }
        if self.filtered_rows.is_empty()
            && !self.query.is_empty()
            && self.pagination.next_cursor.is_some()
            && !self.pagination.reached_scan_cap
            && !self.search_state.is_active()
        {
            // 归档导致当前搜索页变空时，若仍有游标则继续扫描以寻找后续匹配项。
            let token = self.allocate_search_token();
            self.search_state = SearchState::Active { token };
            self.load_more_if_needed(LoadTrigger::Search { token });
        }
    }

    pub(super) fn request_unarchive(&mut self, thread_id: ThreadId) {
        // 恢复沿用同一互斥状态，避免与归档请求并发改变选择器数据源。
        if !matches!(self.archive_state, ArchiveState::Idle) {
            return;
        }

        self.archive_state = ArchiveState::Restoring { thread_id };
        self.request_frame();
        (self.picker_loader)(PickerLoadRequest::Unarchive { thread_id });
    }

    pub(super) fn handle_unarchive_result(
        &mut self,
        thread_id: ThreadId,
        result: std::io::Result<SessionTarget>,
    ) -> Option<SessionSelection> {
        // 成功结果直接转为恢复选择；失败留在选择器中显示可重试错误。
        if self.archive_state != (ArchiveState::Restoring { thread_id }) {
            return None;
        }
        self.archive_state = ArchiveState::Idle;

        match result {
            Ok(target) => Some(SessionSelection::Resume(target)),
            Err(error) => {
                self.inline_error = Some(format!("Failed to restore archived session: {error}"));
                self.request_frame();
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
