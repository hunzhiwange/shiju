//! Node validation and document submission.

use super::*;

impl WorkflowState {
    pub fn submit_node_editor(&mut self) -> Result<(), desktop_i18n::workflow::Feedback> {
        if let Some(editor) = self.node_editor.as_mut() {
            refresh_node_editor_validation(editor);
            if editor.validation.has_errors() {
                return Err(desktop_i18n::workflow::Feedback::new(
                    "请先修正节点表单中的错误字段，再保存。",
                    vec![],
                ));
            }
        }

        let Some(editor) = self.node_editor.as_ref() else {
            return Ok(());
        };

        let mode = editor.mode.clone();
        let block_type = editor.block_type.clone();
        let title = if editor.title.trim().is_empty() {
            pretty_block_type(&block_type)
        } else {
            editor.title.trim().to_string()
        };
        let description = editor.description.trim().to_string();
        let position = editor.position;
        let raw_data_yaml = if editor.visual_draft.is_some() {
            apply_visual_draft_to_yaml(
                &editor.block_type,
                &editor.raw_data_editor.text(),
                editor.visual_draft.as_ref(),
            )?
        } else {
            editor.raw_data_editor.text()
        };

        match mode {
            WorkflowNodeEditorMode::Create => {
                self.ensure_start_node_available(&block_type)?;
                let node_id = generate_node_id(&block_type);
                let next_z =
                    self.document.nodes.iter().map(|node| node.z_index).fold(0.0_f32, f32::max)
                        + 1.0;
                let base_node =
                    create_node_from_type(&block_type, node_id.clone(), position, next_z)?;
                let new_node =
                    rebuild_node_from_parts(&base_node, &title, &description, &raw_data_yaml)?;

                self.push_undo_snapshot();
                self.document.nodes.push(new_node);
                self.selected_node_id = Some(node_id);
                self.selected_edge_id = None;
                self.node_editor = None;
                self.refresh_dirty_state();
                self.set_status(desktop_i18n::format::Message::new(
                    "已新增 %s 节点",
                    vec![pretty_block_type(&block_type).to_string().into()],
                ));
                self.sync_selection_flags();
            }
            WorkflowNodeEditorMode::Edit(node_id) => {
                let index =
                    self.document.nodes.iter().position(|node| node.id == node_id).ok_or_else(
                        || desktop_i18n::workflow::Feedback::new("目标节点不存在", vec![]),
                    )?;
                let existing = self.document.nodes[index].clone();
                let updated =
                    rebuild_node_from_parts(&existing, &title, &description, &raw_data_yaml)?;

                self.push_undo_snapshot();
                self.document.nodes[index] = updated;
                let removed_edges = self.prune_invalid_edges_for_node_handles(&node_id);

                self.selected_node_id = Some(node_id.clone());
                if removed_edges > 0 {
                    self.set_status(desktop_i18n::format::Message::new(
                        "已更新 %s 节点，并移除 %s 条失效连线",
                        vec![
                            pretty_block_type(&block_type).to_string().into(),
                            removed_edges.to_string().into(),
                        ],
                    ));
                } else {
                    self.set_status(desktop_i18n::format::Message::new(
                        "已更新 %s 节点",
                        vec![pretty_block_type(&block_type).to_string().into()],
                    ));
                }
                self.node_editor = None;
                self.refresh_dirty_state();
                self.sync_selection_flags();
            }
        }

        Ok(())
    }
}
