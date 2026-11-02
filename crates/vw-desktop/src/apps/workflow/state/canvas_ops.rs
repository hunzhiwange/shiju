//! # Workflow 画布状态操作
//!
//! 该模块处理画布平移缩放、拖拽、连线创建、删除以及撤销重做等交互状态变更。

use super::*;

impl WorkflowState {
    pub fn pan_by(&mut self, delta: Vector) {
        self.pan = Vector::new(self.pan.x + delta.x, self.pan.y + delta.y);
        self.context_menu = None;
        self.refresh_dirty_state();
    }

    pub fn zoom_by(&mut self, factor: f32, center_opt: Option<Point>, window_size: (f32, f32)) {
        let old_zoom = self.zoom.max(0.0001);
        let new_zoom = (old_zoom * factor).clamp(0.1, 4.0);
        let center = center_opt.unwrap_or(Point::new(window_size.0 / 2.0, window_size.1 / 2.0));
        let center_vector = Vector::new(center.x, center.y);

        self.pan = center_vector - (center_vector - self.pan) * (new_zoom / old_zoom);
        self.zoom = new_zoom;
        self.context_menu = None;
        self.zoom_menu_open = false;
        self.refresh_dirty_state();
    }

    pub fn zoom_set(&mut self, zoom: f32, window_size: (f32, f32)) {
        let current_zoom = self.zoom.max(0.0001);
        let target_zoom = zoom.clamp(0.1, 4.0);
        self.zoom_by(target_zoom / current_zoom, None, window_size);
        self.zoom_menu_open = false;
    }

    pub fn zoom_to_fit(&mut self, window_size: (f32, f32)) {
        let Some(bounds) = self.document.bounds() else {
            self.zoom = 1.0;
            self.pan = Vector::new(120.0, 120.0);
            return;
        };

        let usable_width = (window_size.0 - 72.0).max(320.0);
        let usable_height = (window_size.1 - 176.0).max(260.0);
        let scale_x = usable_width / bounds.width.max(1.0);
        let scale_y = usable_height / bounds.height.max(1.0);
        let zoom = (scale_x.min(scale_y) * 0.92).clamp(0.1, 4.0);

        let world_center =
            Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
        let screen_center = Vector::new(usable_width / 2.0 + 24.0, usable_height / 2.0 + 24.0);

        self.zoom = zoom;
        self.pan = screen_center - Vector::new(world_center.x * zoom, world_center.y * zoom);
        self.context_menu = None;
        self.zoom_menu_open = false;
        self.refresh_dirty_state();
    }

    pub fn start_node_drag(&mut self, id: &str) {
        self.select_node(id.to_string());
        self.action_menu_open = false;
        self.zoom_menu_open = false;

        if self.dragging_node_id.as_deref() != Some(id) {
            self.dragging_node_id = Some(id.to_string());
            self.drag_pending_snapshot = self.current_history_snapshot();
        }
    }

    fn commit_pending_node_drag_snapshot(&mut self) {
        let Some(snapshot) = self.drag_pending_snapshot.take() else {
            return;
        };

        self.push_history_snapshot(snapshot);
    }

    pub fn finish_node_drag(&mut self) {
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;
    }

    pub fn move_node(&mut self, id: &str, delta: Vector) {
        if delta.x.abs() < f32::EPSILON && delta.y.abs() < f32::EPSILON {
            return;
        }

        if self.dragging_node_id.as_deref() == Some(id) {
            self.commit_pending_node_drag_snapshot();
        }

        let descendants = self.document.descendant_ids(id);

        if let Some(node) = self.document.node_mut(id) {
            node.position.x += delta.x;
            node.position.y += delta.y;
        }

        for child_id in descendants {
            if let Some(child) = self.document.node_mut(&child_id) {
                child.position.x += delta.x;
                child.position.y += delta.y;
            }
        }

        self.context_menu = None;
        self.set_status(crate::app::i18n::format::Message::new("已更新节点布局", vec![]));
        self.refresh_dirty_state();
    }

    pub fn start_connection(&mut self, from: WorkflowConnectionEndpoint, cursor_world: Point) {
        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.connection_draft = Some(WorkflowConnectionDraft { from: from.clone(), cursor_world });
        self.selected_node_id = Some(from.node_id);
        self.selected_edge_id = None;
        self.set_status(crate::app::i18n::format::Message::new("拖到目标句柄以创建连线", vec![]));
        self.sync_selection_flags();
    }

    pub fn update_connection_cursor(&mut self, cursor_world: Point) {
        if let Some(draft) = self.connection_draft.as_mut() {
            draft.cursor_world = cursor_world;
        }
    }

    pub fn cancel_connection(&mut self) {
        if self.connection_draft.take().is_some() {
            self.set_status(crate::app::i18n::format::Message::new("已取消连线", vec![]));
        }
    }

    pub fn cancel_interaction(&mut self) {
        if self.action_menu_open || self.zoom_menu_open || self.quick_insert_panel_open {
            self.close_floating_panels();
            self.set_status(crate::app::i18n::format::Message::new("已关闭浮层菜单", vec![]));
        } else if self.context_menu.take().is_some() {
            self.set_status(crate::app::i18n::format::Message::new("已关闭右键菜单", vec![]));
        } else if self.connection_draft.is_some() {
            self.cancel_connection();
        } else if self.variable_editor.is_some() {
            self.variable_editor = None;
            self.set_status(crate::app::i18n::format::Message::new("已关闭变量编辑器", vec![]));
        } else if self.variable_panel.is_some() {
            self.variable_panel = None;
            self.set_status(crate::app::i18n::format::Message::new("已关闭变量面板", vec![]));
        } else if self.selected_edge_id.is_some() || self.selected_node_id.is_some() {
            self.clear_selection();
            self.set_status(crate::app::i18n::format::Message::new("已清除选择", vec![]));
        }
    }

    pub fn finish_connection(&mut self, to: WorkflowConnectionEndpoint) {
        let Some(draft) = self.connection_draft.take() else {
            return;
        };

        let Some((source, target)) = normalize_connection_endpoints(&draft.from, &to) else {
            self.set_status(crate::app::i18n::format::Message::new(
                "连线需要从输出句柄连接到输入句柄",
                vec![],
            ));
            return;
        };

        if source.node_id == target.node_id {
            self.set_status(crate::app::i18n::format::Message::new(
                "暂不支持节点自身回环连线",
                vec![],
            ));
            return;
        }

        if let Some(existing) = self.document.edges.iter().find(|edge| {
            edge.source == source.node_id
                && edge.target == target.node_id
                && edge.source_handle.as_deref() == Some(source.handle_id.as_str())
                && edge.target_handle.as_deref() == Some(target.handle_id.as_str())
        }) {
            self.select_edge(existing.id.clone());
            self.set_status(crate::app::i18n::format::Message::new("这条连线已经存在", vec![]));
            return;
        }

        let Some(source_node) = self.document.node(&source.node_id) else {
            self.set_status(crate::app::i18n::format::Message::new(
                "源节点不存在，无法创建连线",
                vec![],
            ));
            return;
        };
        let source_type = source_node.block_type.clone();
        let source_title = source_node.title.clone();

        let Some(target_node) = self.document.node(&target.node_id) else {
            self.set_status(crate::app::i18n::format::Message::new(
                "目标节点不存在，无法创建连线",
                vec![],
            ));
            return;
        };
        let target_type = target_node.block_type.clone();
        let target_title = target_node.title.clone();

        let edge_id = generate_edge_id(&source, &target);
        let next_z =
            self.document.edges.iter().map(|edge| edge.z_index).fold(0.0_f32, f32::max) + 1.0;

        self.push_undo_snapshot();
        self.document.edges.push(WorkflowEdge {
            id: edge_id.clone(),
            source: source.node_id.clone(),
            target: target.node_id.clone(),
            source_handle: Some(source.handle_id.clone()),
            target_handle: Some(target.handle_id.clone()),
            source_type,
            target_type,
            selected: true,
            z_index: next_z,
            raw_edge: Value::Null,
        });

        self.selected_node_id = None;
        self.selected_edge_id = Some(edge_id);
        self.context_menu = None;
        self.set_status(crate::app::i18n::format::Message::new(
            "已连接 %s -> %s",
            vec![source_title.to_string().into(), target_title.to_string().into()],
        ));
        self.sync_selection_flags();
        self.refresh_dirty_state();
    }

    pub fn delete_selected_edge(&mut self) -> bool {
        let Some(edge_id) = self.selected_edge_id.clone() else {
            return false;
        };

        self.push_undo_snapshot();

        let Some(removed) = self.document.remove_edge(&edge_id) else {
            return false;
        };

        self.selected_edge_id = None;
        self.clear_context_menu_if_target_edge(&edge_id);
        self.sync_selection_flags();
        self.set_status(crate::app::i18n::format::Message::new(
            "已删除连线 %s -> %s",
            vec![removed.source.to_string().into(), removed.target.to_string().into()],
        ));
        self.refresh_dirty_state();
        true
    }

    pub fn delete_selected_node(&mut self) -> bool {
        let Some(node_id) = self.selected_node_id.clone() else {
            return false;
        };

        let Some(node) = self.document.node(&node_id).cloned() else {
            self.selected_node_id = None;
            self.sync_selection_flags();
            return false;
        };

        let descendant_ids = self.document.descendant_ids(&node_id);
        let removed_child_count = descendant_ids.len();
        let removed_ids =
            std::iter::once(node_id.clone()).chain(descendant_ids).collect::<HashSet<_>>();
        let removed_edge_count = self
            .document
            .edges
            .iter()
            .filter(|edge| {
                removed_ids.contains(edge.source.as_str())
                    || removed_ids.contains(edge.target.as_str())
            })
            .count();

        self.push_undo_snapshot();

        self.document.nodes.retain(|candidate| !removed_ids.contains(candidate.id.as_str()));
        self.document.edges.retain(|edge| {
            !removed_ids.contains(edge.source.as_str())
                && !removed_ids.contains(edge.target.as_str())
        });

        self.selected_node_id = None;
        self.selected_edge_id = None;

        if self
            .connection_draft
            .as_ref()
            .is_some_and(|draft| removed_ids.contains(draft.from.node_id.as_str()))
        {
            self.connection_draft = None;
        }

        if self.node_editor.as_ref().is_some_and(|editor| match &editor.mode {
            WorkflowNodeEditorMode::Create => false,
            WorkflowNodeEditorMode::Edit(editing_id) => removed_ids.contains(editing_id.as_str()),
        }) {
            self.node_editor = None;
        }

        self.clear_context_menu_if_target_node_ids(&removed_ids);

        self.sync_selection_flags();
        self.refresh_dirty_state();

        let (source, arguments) = match (removed_child_count > 0, removed_edge_count > 0) {
            (true, true) => (
                "已删除节点 %s，包含 %s 个子节点，并移除 %s 条连线",
                vec![
                    node.title.into(),
                    removed_child_count.to_string().into(),
                    removed_edge_count.to_string().into(),
                ],
            ),
            (true, false) => (
                "已删除节点 %s，包含 %s 个子节点",
                vec![node.title.into(), removed_child_count.to_string().into()],
            ),
            (false, true) => (
                "已删除节点 %s，并移除 %s 条连线",
                vec![node.title.into(), removed_edge_count.to_string().into()],
            ),
            (false, false) => ("已删除节点 %s", vec![node.title.into()]),
        };
        self.set_status(crate::app::i18n::format::Message::new(source, arguments));
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        let Some(current) = self.current_history_snapshot() else {
            return false;
        };

        self.redo_stack.push(current);
        if self.redo_stack.len() > WORKFLOW_HISTORY_LIMIT {
            self.redo_stack.remove(0);
        }

        self.restore_history_snapshot(previous);
        self.set_status(crate::app::i18n::format::Message::new("已撤销上一步", vec![]));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        let Some(current) = self.current_history_snapshot() else {
            return false;
        };

        self.undo_stack.push(current);
        if self.undo_stack.len() > WORKFLOW_HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }

        self.restore_history_snapshot(next);
        self.set_status(crate::app::i18n::format::Message::new("已重做上一步", vec![]));
        true
    }

    fn clear_context_menu_if_target_edge(&mut self, edge_id: &str) {
        if self.context_menu.as_ref().is_some_and(|menu| {
            matches!(&menu.target, WorkflowCanvasContextMenuTarget::Edge(current_id) if current_id == edge_id)
        }) {
            self.context_menu = None;
        }
    }

    fn clear_context_menu_if_target_node_ids(&mut self, removed_ids: &HashSet<String>) {
        if self.context_menu.as_ref().is_some_and(|menu| {
            matches!(
                &menu.target,
                WorkflowCanvasContextMenuTarget::Node(node_id)
                    | WorkflowCanvasContextMenuTarget::NodeInsert(node_id)
                    if removed_ids.contains(node_id)
            )
        }) {
            self.context_menu = None;
        }
    }
}
