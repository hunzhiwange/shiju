//! # Workflow 应用界面状态
//!
//! 该模块处理应用级 UI 状态，包括右键菜单、应用信息编辑和当前应用快照持久化。

use super::*;

impl WorkflowState {
    pub fn open_context_menu(
        &mut self,
        target: WorkflowCanvasContextMenuTarget,
        anchor: Point,
        world: Point,
    ) {
        self.connection_draft = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        match &target {
            WorkflowCanvasContextMenuTarget::Canvas => {
                self.selected_node_id = None;
                self.selected_edge_id = None;
            }
            WorkflowCanvasContextMenuTarget::Node(node_id)
            | WorkflowCanvasContextMenuTarget::NodeInsert(node_id) => {
                self.selected_node_id = Some(node_id.clone());
                self.selected_edge_id = None;
            }
            WorkflowCanvasContextMenuTarget::Edge(edge_id) => {
                self.selected_edge_id = Some(edge_id.clone());
                self.selected_node_id = None;
            }
        }
        self.context_menu = Some(WorkflowCanvasContextMenu { target, anchor, world });
        self.sync_selection_flags();
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub fn context_menu_new_node_position(&self) -> Point {
        let Some(menu) = self.context_menu.as_ref() else {
            return Point::new(80.0, 80.0);
        };

        match &menu.target {
            WorkflowCanvasContextMenuTarget::Canvas | WorkflowCanvasContextMenuTarget::Edge(_) => {
                menu.world
            }
            WorkflowCanvasContextMenuTarget::Node(node_id)
            | WorkflowCanvasContextMenuTarget::NodeInsert(node_id) => self
                .document
                .node(node_id)
                .map(|node| {
                    Point::new(node.position.x + node.size.width + 120.0, node.position.y + 18.0)
                })
                .unwrap_or(menu.world),
        }
    }

    pub(super) fn context_menu_auto_connect_source_node_id(&self) -> Option<String> {
        self.context_menu.as_ref().and_then(|menu| match &menu.target {
            WorkflowCanvasContextMenuTarget::NodeInsert(node_id) => Some(node_id.clone()),
            _ => None,
        })
    }

    pub fn set_editor_name(&mut self, value: String) {
        if let Some(editor) = self.app_editor.as_mut() {
            editor.name = value;
        }
    }

    pub fn set_editor_description(&mut self, value: String) {
        if let Some(editor) = self.app_editor.as_mut() {
            editor.description = value;
        }
    }

    pub fn set_editor_icon(&mut self, value: String) {
        if let Some(editor) = self.app_editor.as_mut() {
            editor.icon = value;
        }
    }

    pub fn set_editor_use_icon_as_answer_icon(&mut self, value: bool) {
        if let Some(editor) = self.app_editor.as_mut() {
            editor.use_icon_as_answer_icon = value;
        }
    }

    pub fn set_editor_max_active_requests_input(&mut self, value: String) {
        if let Some(editor) = self.app_editor.as_mut() {
            editor.max_active_requests_input = value;
        }
    }

    pub fn submit_editor(
        &mut self,
        window_size: (f32, f32),
        loaded: LoadedWorkflow,
    ) -> Result<(), String> {
        let Some(editor) = self.app_editor.clone() else {
            return Ok(());
        };

        let max_active_requests = editor
            .max_active_requests_input
            .trim()
            .parse::<u32>()
            .map_err(|_| "最大活跃请求数必须是非负整数".to_string())?;

        let meta = WorkflowAppMeta {
            name: editor.name.trim().to_string(),
            description: editor.description.trim().to_string(),
            icon: if editor.icon.trim().is_empty() {
                "🤖".to_string()
            } else {
                editor.icon.trim().to_string()
            },
            icon_background: "#FFEAD5".to_string(),
            mode: "advanced-chat".to_string(),
            use_icon_as_answer_icon: editor.use_icon_as_answer_icon,
            max_active_requests,
        };

        if meta.name.is_empty() {
            return Err("应用名称不能为空".to_string());
        }

        match editor.mode {
            WorkflowAppEditorMode::Create => {
                let mut loaded = loaded;
                loaded.app_meta = meta.clone();
                loaded.source_name = meta.name.clone();
                loaded.document.name = meta.name.clone();
                self.apply_loaded(loaded, window_size);
            }
            WorkflowAppEditorMode::Edit(target_id) => {
                let is_active = self.active_app_id.as_deref() == Some(target_id.as_str());
                if is_active {
                    self.push_undo_snapshot();
                }

                if let Some(app) = self.apps.iter_mut().find(|app| app.id == target_id) {
                    app.meta = meta.clone();
                    if is_active {
                        self.source_name = meta.name.clone();
                        self.refresh_dirty_state();
                    } else {
                        app.is_dirty = true;
                    }
                }
            }
        }

        self.app_editor = None;
        self.action_menu_open = false;
        self.status_message = Some("已更新应用信息".to_string());
        Ok(())
    }

    pub fn organize_active_app(&mut self, window_size: (f32, f32)) -> Result<(), String> {
        if self.active_app_id.is_none() {
            return Err("请先打开一个应用".to_string());
        }
        if self.document.nodes.is_empty() {
            return Err("当前应用没有可整理的节点".to_string());
        }

        let positions = organized_node_positions(&self.document);
        if positions.is_empty() {
            return Ok(());
        }

        self.push_undo_snapshot();
        for node in &mut self.document.nodes {
            if let Some(position) = positions.get(&node.id) {
                node.position = *position;
            }
        }

        let (pan, zoom) = fitted_viewport(&self.document, window_size);
        self.pan = pan;
        self.zoom = zoom;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;
        self.refresh_dirty_state();
        self.status_message = Some("已整理应用节点位置".to_string());
        Ok(())
    }

    pub fn persist_active_snapshot(&mut self) {
        let Some(active_id) = self.active_app_id.clone() else {
            return;
        };

        let current_snapshot = self.current_history_snapshot();

        if let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id) {
            app.local_uuid = self.local_uuid.clone();
            app.source_path = self.source_path.clone();
            app.document = self.document.clone();
            app.environment_variables = self.environment_variables.clone();
            app.conversation_variables = self.conversation_variables.clone();
            app.pan = self.pan;
            app.zoom = self.zoom;
            app.selected_node_id = self.selected_node_id.clone();
            app.selected_edge_id = self.selected_edge_id.clone();
            app.connection_draft = self.connection_draft.clone();
            app.is_dirty = self.active_is_dirty;
            app.undo_stack = self.undo_stack.clone();
            app.redo_stack = self.redo_stack.clone();
            if let Some(saved_snapshot) = self.saved_snapshot.clone() {
                app.saved_snapshot = saved_snapshot;
            } else if let Some(snapshot) = current_snapshot.clone() {
                app.saved_snapshot = snapshot;
            }
            if self.source_name != app.meta.name {
                app.meta.name = self.source_name.clone();
            }
        }
    }

    pub fn active_entry_snapshot(&mut self) -> Option<WorkflowAppEntry> {
        self.persist_active_snapshot();
        let active_id = self.active_app_id.as_deref()?;
        self.apps.iter().find(|app| app.id == active_id).cloned()
    }
}

pub(super) fn organized_node_positions(document: &WorkflowDocument) -> HashMap<String, Point> {
    let node_order = document.nodes.iter().map(|node| node.id.clone()).collect::<Vec<_>>();
    let node_index = node_order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect::<HashMap<_, _>>();
    let node_ids = node_order.iter().cloned().collect::<HashSet<_>>();
    let mut layer_by_node =
        node_order.iter().map(|id| (id.clone(), 0_usize)).collect::<HashMap<_, _>>();

    for _ in 0..node_order.len() {
        let mut changed = false;
        for edge in &document.edges {
            if !node_ids.contains(&edge.source) || !node_ids.contains(&edge.target) {
                continue;
            }
            let source_layer = layer_by_node.get(&edge.source).copied().unwrap_or(0);
            let target_layer = layer_by_node.get(&edge.target).copied().unwrap_or(0);
            if target_layer <= source_layer {
                layer_by_node.insert(edge.target.clone(), source_layer + 1);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut layers = Vec::<Vec<String>>::new();
    for id in &node_order {
        let layer = layer_by_node.get(id).copied().unwrap_or(0);
        if layers.len() <= layer {
            layers.resize_with(layer + 1, Vec::new);
        }
        layers[layer].push(id.clone());
    }
    for layer in &mut layers {
        layer.sort_by_key(|id| node_index.get(id).copied().unwrap_or(usize::MAX));
    }

    let mut positions = HashMap::new();
    let mut x = -180.0;
    for layer in layers {
        if layer.is_empty() {
            x += 360.0;
            continue;
        }

        let mut y = 120.0;
        let mut max_width = 0.0_f32;
        for node_id in layer {
            if let Some(node) = document.node(&node_id) {
                positions.insert(node_id, Point::new(x, y));
                y += node.size.height.max(120.0) + 80.0;
                max_width = max_width.max(node.size.width);
            }
        }
        x += max_width.max(240.0) + 140.0;
    }

    positions
}
