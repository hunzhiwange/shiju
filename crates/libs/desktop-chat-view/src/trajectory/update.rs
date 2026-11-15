//! 轨迹视图的局部交互更新；仅返回复制文本效果。
use super::model::{TrajectoryControlsState, TrajectoryDetailsState, TrajectoryDetailsTab, TrajectoryDetailsAction, TrajectoryProjection};
use super::details::trajectory_detail_values;
use iced::widget::text_editor;

pub enum Message {
    TrajectorySearchChanged(String),

    /// 在真实耗时与等宽时间线间切换。
    TrajectoryDurationToggled,

    /// 按全局规则展开或折叠所有轮次。
    TrajectoryTurnsToggled,

    /// 按全局规则展开或折叠所有工具调用步骤。
    TrajectoryCallsToggled,

    /// 独立展开或折叠一个轮次。
    TrajectoryTurnToggled(u32),

    /// 独立展开或折叠一个工具调用步骤。
    TrajectoryToolStepToggled(u32),

    /// 选中一条稳定轨迹记录。
    TrajectoryRecordSelected(String),

    /// 切换轨迹详情标签。
    TrajectoryDetailsTabSelected(TrajectoryDetailsTab),

    /// 关闭轨迹详情。
    TrajectoryDetailsClosed,

    /// 在只读轨迹详情中移动光标或更新选区。
    TrajectoryDetailsEditorAction(usize, text_editor::Action),

    /// 在指定详情字段上打开右键复制菜单。
    TrajectoryDetailsContextMenuOpened {
        index: usize,
        x: f32,
        y: f32,
    },

    /// 关闭轨迹详情的右键菜单。
    TrajectoryDetailsContextMenuClosed,

    /// 复制轨迹详情的当前选区，未选中时复制整个字段。
    TrajectoryDetailsContextMenuCopy,

    /// 展开或折叠当前详情标签中的 JSON 路径。
    TrajectoryDetailsJsonToggled(String),

    /// 复制 JSON 树节点或完整树。
    TrajectoryDetailsJsonCopied(serde_json::Value),
}

pub fn update(controls: &mut TrajectoryControlsState, details: &mut TrajectoryDetailsState, search_text: &mut String, session_id: Option<&str>, projection: &TrajectoryProjection, message: Message) -> Option<String> {
    match message {
        Message::TrajectorySearchChanged(search) => {
            controls.sync_session(session_id);
            *search_text = search;
            None
        }
        Message::TrajectoryDurationToggled => {
            controls.sync_session(session_id);
            controls.toggle_duration_mode();
            None
        }
        Message::TrajectoryTurnsToggled => {
            controls.sync_session(session_id);
            controls.toggle_all_turns(projection);
            None
        }
        Message::TrajectoryCallsToggled => {
            controls.sync_session(session_id);
            controls.toggle_all_calls(projection);
            None
        }
        Message::TrajectoryTurnToggled(turn_index) => {
            controls.sync_session(session_id);
            controls.toggle_turn(turn_index);
            None
        }
        Message::TrajectoryToolStepToggled(step_index) => {
            controls.sync_session(session_id);
            controls.toggle_tool_step(step_index);
            None
        }
        Message::TrajectoryRecordSelected(record_id) => {
            controls.sync_session(session_id);
            let changed = if let Some(record) = projection.record(&record_id) {
                controls.reveal_record(record);
                details.select_record(record)
            } else {
                details.select(record_id)
            };
            if changed {
                sync_details_values(details, projection);
            }
            None
        }
        Message::TrajectoryDetailsTabSelected(tab) => {
            if details.apply(TrajectoryDetailsAction::SelectTab(tab)) {
                sync_details_values(details, projection);
            }
            None
        }
        Message::TrajectoryDetailsClosed => {
            details.apply(TrajectoryDetailsAction::Close);
            None
        }
        Message::TrajectoryDetailsEditorAction(index, action) => {
            details.perform_editor_action(index, action);
            None
        }
        Message::TrajectoryDetailsContextMenuOpened { index, x, y } => {
            details.open_context_menu(index, x, y);
            None
        }
        Message::TrajectoryDetailsContextMenuClosed => {
            details.close_context_menu();
            None
        }
        Message::TrajectoryDetailsContextMenuCopy => {
            let text = details.context_menu_copy_text();
            details.close_context_menu();
            text
        }
        Message::TrajectoryDetailsJsonToggled(path) => {
            details.toggle_json_path(path);
            None
        }
        Message::TrajectoryDetailsJsonCopied(value) => {
            let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            Some(text)
        }
    }
}

pub fn sync_details_values(details: &mut TrajectoryDetailsState, projection: &TrajectoryProjection) {
    let values = details.selected_record_id()
        .and_then(|id| projection.record(id))
        .map(|record| trajectory_detail_values(record, details.active_tab()))
        .unwrap_or_default();
    details.sync_values(values);
}
