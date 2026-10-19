//! # Workflow 节点操作
//!
//! 该模块处理节点编辑器开关、节点插入复制、下游创建和节点定位等操作逻辑。

use super::*;
use crate::apps::workflow::model::WorkflowHandle;

impl WorkflowState {
    pub fn open_create_node_editor(
        &mut self,
        block_type: &str,
        position: Point,
    ) -> Result<(), String> {
        self.ensure_start_node_available(block_type)?;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = None;
        self.variable_editor = None;
        self.node_editor_dsl_context_menu_open = false;
        self.node_editor_dsl_context_menu_pos = None;
        self.node_editor_dsl_undo_stack.clear();
        self.node_editor_dsl_redo_stack.clear();
        let yaml = default_node_data_yaml(block_type)?;
        let title = pretty_block_type(block_type);
        let visual_draft = build_node_visual_draft(block_type, &yaml)?;
        let raw_data_editor = text_editor::Content::with_text(&yaml);
        let active_tab = if visual_draft.is_some() {
            WorkflowNodeEditorTab::Visual
        } else {
            WorkflowNodeEditorTab::Description
        };
        let start_variable_focus_index = initial_start_variable_focus(visual_draft.as_ref());
        self.app_editor = None;
        self.node_editor = Some(WorkflowNodeEditorDraft {
            mode: WorkflowNodeEditorMode::Create,
            active_tab,
            block_type: block_type.to_string(),
            title,
            description: String::new(),
            description_editor: text_editor::Content::with_text(""),
            position,
            visual_draft,
            hovered_start_variable_index: None,
            start_variable_focus_index,
            start_variable_editor: None,
            validation: WorkflowNodeEditorValidation::default(),
            show_raw_data_editor: false,
            raw_data_editor,
        });
        Ok(())
    }

    pub fn open_edit_node_editor(&mut self, id: Option<&str>) -> Result<(), String> {
        let target_id = id
            .map(|value| value.to_string())
            .or_else(|| self.selected_node_id.clone())
            .ok_or_else(|| "请先选择一个节点".to_string())?;
        let node =
            self.document.node(&target_id).cloned().ok_or_else(|| "目标节点不存在".to_string())?;
        let yaml = node_data_yaml(&node)?;
        let visual_draft = build_node_visual_draft(&node.block_type, &yaml)?;
        let raw_data_editor = text_editor::Content::with_text(&yaml);
        let active_tab = if visual_draft.is_some() {
            WorkflowNodeEditorTab::Visual
        } else {
            WorkflowNodeEditorTab::Description
        };
        let start_variable_focus_index = initial_start_variable_focus(visual_draft.as_ref());
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = None;
        self.variable_editor = None;
        self.node_editor_dsl_context_menu_open = false;
        self.node_editor_dsl_context_menu_pos = None;
        self.node_editor_dsl_undo_stack.clear();
        self.node_editor_dsl_redo_stack.clear();
        self.app_editor = None;
        self.node_editor = Some(WorkflowNodeEditorDraft {
            mode: WorkflowNodeEditorMode::Edit(target_id),
            active_tab,
            block_type: node.block_type.clone(),
            title: node.title.clone(),
            description: node.description.clone(),
            description_editor: text_editor::Content::with_text(&node.description),
            position: node.position,
            visual_draft,
            hovered_start_variable_index: None,
            start_variable_focus_index,
            start_variable_editor: None,
            validation: WorkflowNodeEditorValidation::default(),
            show_raw_data_editor: false,
            raw_data_editor,
        });
        Ok(())
    }

    pub fn close_node_editor(&mut self) {
        self.node_editor = None;
        self.node_editor_dsl_context_menu_open = false;
        self.node_editor_dsl_context_menu_pos = None;
        self.node_editor_dsl_undo_stack.clear();
        self.node_editor_dsl_redo_stack.clear();
    }

    pub fn set_node_editor_active_tab(&mut self, tab: WorkflowNodeEditorTab) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.active_tab = tab;
        }
        self.node_editor_dsl_context_menu_open = false;
        self.node_editor_dsl_context_menu_pos = None;
    }

    pub fn insert_node_immediately(
        &mut self,
        block_type: &str,
        position: Point,
    ) -> Result<(), String> {
        self.insert_node_immediately_with_id(block_type, position).map(|_| ())
    }

    pub fn create_context_node(&mut self, block_type: &str) -> Result<(), String> {
        let position = self.context_menu_new_node_position();
        let upstream_node_id = self.context_menu_auto_connect_source_node_id();
        let node_id = self.insert_node_immediately_with_id(block_type, position)?;
        let undo_len_after_insert = self.undo_stack.len();

        if let Some(source_node_id) = upstream_node_id {
            let target_title = self
                .document
                .node(&node_id)
                .map(|node| node.title.clone())
                .unwrap_or_else(|| pretty_block_type(block_type));

            match self.connect_nodes_by_default_handles(&source_node_id, &node_id) {
                Ok(()) => {
                    if self.undo_stack.len() > undo_len_after_insert {
                        self.undo_stack.pop();
                    }
                    self.selected_node_id = Some(node_id);
                    self.selected_edge_id = None;
                    self.sync_selection_flags();
                    self.status_message =
                        Some(format!("已新增下游 {} 节点并自动关联", target_title));
                }
                Err(error) => {
                    self.selected_node_id = Some(node_id);
                    self.selected_edge_id = None;
                    self.sync_selection_flags();
                    self.status_message =
                        Some(format!("已新增 {} 节点，但自动关联失败：{}", target_title, error));
                }
            }
        }

        Ok(())
    }

    pub fn duplicate_selected_node(&mut self) -> Result<(), String> {
        let source_id =
            self.selected_node_id.clone().ok_or_else(|| "请先选择一个节点".to_string())?;
        let source_node =
            self.document.node(&source_id).cloned().ok_or_else(|| "目标节点不存在".to_string())?;

        if source_node.block_type == "start" {
            return Err("开始节点只能有一个，不能复制开始节点".to_string());
        }

        let next_z =
            self.document.nodes.iter().map(|node| node.z_index).fold(0.0_f32, f32::max) + 1.0;
        let node_id = generate_node_id(&source_node.block_type);
        let mut duplicated = source_node.clone();
        duplicated.id = node_id.clone();
        duplicated.position =
            Point::new(source_node.position.x + 36.0, source_node.position.y + 36.0);
        duplicated.selected = true;
        duplicated.z_index = next_z;

        self.push_undo_snapshot();
        self.document.nodes.push(duplicated);
        self.selected_node_id = Some(node_id.clone());
        self.selected_edge_id = None;
        self.connection_draft = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.refresh_dirty_state();
        self.status_message = Some(format!("已复制节点 {}", source_node.title));
        self.sync_selection_flags();
        Ok(())
    }

    pub fn open_downstream_node_picker(&mut self, node_id: &str) -> Result<(), String> {
        let anchor = self
            .context_menu
            .as_ref()
            .map(|menu| menu.anchor)
            .ok_or_else(|| "右键菜单已关闭".to_string())?;
        let node = self.document.node(node_id).ok_or_else(|| "目标节点不存在".to_string())?;
        let world = Point::new(node.position.x + node.size.width + 120.0, node.position.y + 18.0);

        self.selected_node_id = Some(node_id.to_string());
        self.selected_edge_id = None;
        self.context_menu = Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::NodeInsert(node_id.to_string()),
            anchor,
            world,
        });
        self.sync_selection_flags();
        Ok(())
    }
    pub fn insert_downstream_node(
        &mut self,
        source_node_id: &str,
        block_type: &str,
    ) -> Result<(), String> {
        self.insert_downstream_node_with_handle(source_node_id, None, block_type)
    }

    pub fn insert_downstream_node_from_handle(
        &mut self,
        source_node_id: &str,
        source_handle_id: &str,
        block_type: &str,
    ) -> Result<(), String> {
        self.insert_downstream_node_with_handle(source_node_id, Some(source_handle_id), block_type)
    }

    fn insert_downstream_node_with_handle(
        &mut self,
        source_node_id: &str,
        source_handle_id: Option<&str>,
        block_type: &str,
    ) -> Result<(), String> {
        self.ensure_start_node_available(block_type)?;

        let source_node = self
            .document
            .node(source_node_id)
            .cloned()
            .ok_or_else(|| "目标节点不存在".to_string())?;
        let position = Point::new(
            source_node.position.x + source_node.size.width + 120.0,
            source_node.position.y + 18.0,
        );

        let node_id = generate_node_id(block_type);
        let next_z =
            self.document.nodes.iter().map(|node| node.z_index).fold(0.0_f32, f32::max) + 1.0;
        let title = pretty_block_type(block_type);
        let raw_data_yaml = default_node_data_yaml(block_type)?;
        let base_node = create_node_from_type(block_type, node_id.clone(), position, next_z)?;
        let new_node = rebuild_node_from_parts(&base_node, &title, "", &raw_data_yaml)?;

        self.push_undo_snapshot();
        self.document.nodes.push(new_node);

        let status_message =
            match self.connect_nodes_with_source_handle(source_node_id, source_handle_id, &node_id)
            {
                Ok(()) => format!("已在 {} 后新增 {} 节点", source_node.title, title),
                Err(error) => format!("已新增 {} 节点，但自动关联失败：{}", title, error),
            };

        self.selected_node_id = Some(source_node_id.to_string());
        self.selected_edge_id = None;
        self.connection_draft = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.refresh_dirty_state();
        self.status_message = Some(status_message);
        self.sync_selection_flags();
        Ok(())
    }

    fn insert_node_immediately_with_id(
        &mut self,
        block_type: &str,
        position: Point,
    ) -> Result<String, String> {
        self.ensure_start_node_available(block_type)?;
        let node_id = generate_node_id(block_type);
        let next_z =
            self.document.nodes.iter().map(|node| node.z_index).fold(0.0_f32, f32::max) + 1.0;
        let title = pretty_block_type(block_type);
        let raw_data_yaml = default_node_data_yaml(block_type)?;
        let base_node = create_node_from_type(block_type, node_id.clone(), position, next_z)?;
        let new_node = rebuild_node_from_parts(&base_node, &title, "", &raw_data_yaml)?;

        self.push_undo_snapshot();
        self.document.nodes.push(new_node);
        self.selected_node_id = Some(node_id.clone());
        self.selected_edge_id = None;
        self.connection_draft = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.refresh_dirty_state();
        self.status_message = Some(format!("已插入 {} 节点", title));
        self.sync_selection_flags();
        Ok(node_id)
    }

    pub(super) fn ensure_start_node_available(&self, block_type: &str) -> Result<(), String> {
        if block_type == "start" && self.has_start_node() {
            return Err("开始节点只能有一个".to_string());
        }

        Ok(())
    }

    pub(super) fn connect_nodes_by_default_handles(
        &mut self,
        source_node_id: &str,
        target_node_id: &str,
    ) -> Result<(), String> {
        self.connect_nodes_with_source_handle(source_node_id, None, target_node_id)
    }

    pub(super) fn connect_nodes_with_source_handle(
        &mut self,
        source_node_id: &str,
        source_handle_id: Option<&str>,
        target_node_id: &str,
    ) -> Result<(), String> {
        if source_node_id == target_node_id {
            return Err("暂不支持节点自身回环连线".to_string());
        }

        let source_node = self
            .document
            .node(source_node_id)
            .cloned()
            .ok_or_else(|| "源节点不存在，无法自动连线".to_string())?;
        let target_node = self
            .document
            .node(target_node_id)
            .cloned()
            .ok_or_else(|| "目标节点不存在，无法自动连线".to_string())?;

        let source_handle = match source_handle_id {
            Some(handle_id) => source_node
                .source_handles
                .iter()
                .find(|handle| handle.id == handle_id)
                .cloned()
                .or_else(|| self.synthetic_source_handle_for_editor(&source_node, handle_id))
                .ok_or_else(|| format!("源节点不存在输出句柄 {}", handle_id))?,
            None => source_node
                .source_handles
                .first()
                .cloned()
                .ok_or_else(|| "源节点没有可用输出句柄".to_string())?,
        };
        let target_handle = target_node
            .target_handles
            .first()
            .ok_or_else(|| "目标节点没有可用输入句柄".to_string())?;

        if self.document.edges.iter().any(|edge| {
            edge.source == source_node.id
                && edge.target == target_node.id
                && edge.source_handle.as_deref() == Some(source_handle.id.as_str())
                && edge.target_handle.as_deref() == Some(target_handle.id.as_str())
        }) {
            return Err("这条连线已经存在".to_string());
        }

        let source = WorkflowConnectionEndpoint {
            node_id: source_node.id.clone(),
            handle_id: source_handle.id.clone(),
            kind: WorkflowHandleKind::Source,
        };
        let target = WorkflowConnectionEndpoint {
            node_id: target_node.id.clone(),
            handle_id: target_handle.id.clone(),
            kind: WorkflowHandleKind::Target,
        };
        let edge_id = generate_edge_id(&source, &target);
        let next_z =
            self.document.edges.iter().map(|edge| edge.z_index).fold(0.0_f32, f32::max) + 1.0;

        self.push_undo_snapshot();
        self.document.edges.push(WorkflowEdge {
            id: edge_id,
            source: source.node_id,
            target: target.node_id,
            source_handle: Some(source.handle_id),
            target_handle: Some(target.handle_id),
            source_type: source_node.block_type,
            target_type: target_node.block_type,
            selected: false,
            z_index: next_z,
            raw_edge: Value::Null,
        });

        Ok(())
    }

    fn synthetic_source_handle_for_editor(
        &self,
        source_node: &WorkflowNode,
        handle_id: &str,
    ) -> Option<WorkflowHandle> {
        if handle_id != "fail-branch" {
            return None;
        }

        let editor = self.node_editor.as_ref()?;
        match (&editor.mode, editor.block_type.as_str(), editor.visual_draft.as_ref()) {
            (
                WorkflowNodeEditorMode::Edit(editor_node_id),
                "code",
                Some(WorkflowNodeVisualDraft::Code { error_strategy, .. }),
            ) if editor_node_id == &source_node.id && error_strategy == "fail-branch" => {
                Some(WorkflowHandle {
                    id: handle_id.to_string(),
                    label: "异常".to_string(),
                    kind: WorkflowHandleKind::Source,
                })
            }
            _ => None,
        }
    }

    pub(super) fn prune_invalid_edges_for_node_handles(&mut self, node_id: &str) -> usize {
        let Some(node) = self.document.node(node_id).cloned() else {
            return 0;
        };

        let valid_source_handles = node
            .source_handles
            .iter()
            .map(|handle| handle.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let valid_target_handles = node
            .target_handles
            .iter()
            .map(|handle| handle.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut removed_edge_ids = Vec::new();

        self.document.edges.retain(|edge| {
            let invalid_source = edge.source == node_id
                && edge
                    .source_handle
                    .as_deref()
                    .map(|handle_id| !valid_source_handles.contains(handle_id))
                    .unwrap_or(true);
            let invalid_target = edge.target == node_id
                && edge
                    .target_handle
                    .as_deref()
                    .map(|handle_id| !valid_target_handles.contains(handle_id))
                    .unwrap_or(true);
            let keep = !(invalid_source || invalid_target);
            if !keep {
                removed_edge_ids.push(edge.id.clone());
            }
            keep
        });

        if removed_edge_ids.is_empty() {
            return 0;
        }

        if self.selected_edge_id.as_ref().is_some_and(|selected_id| {
            removed_edge_ids.iter().any(|edge_id| edge_id == selected_id)
        }) {
            self.selected_edge_id = None;
        }

        if self.context_menu.as_ref().is_some_and(|menu| {
            matches!(
                &menu.target,
                WorkflowCanvasContextMenuTarget::Edge(edge_id)
                    if removed_edge_ids.iter().any(|removed_id| removed_id == edge_id)
            )
        }) {
            self.context_menu = None;
        }

        removed_edge_ids.len()
    }
}

#[cfg(test)]
#[path = "node_ops_tests.rs"]
mod node_ops_tests;
