//! # Workflow 画布组件
//!
//! 该模块定义工作流画布组件，负责节点与连线的命中测试、鼠标交互和画布绘制调度。

use crate::app::Message;
use crate::app::assets;
use crate::apps::workflow::message::WorkflowMessage;
use crate::apps::workflow::model::{
    WorkflowConnectionDraft, WorkflowConnectionEndpoint, WorkflowDocument, WorkflowEdge,
    WorkflowHandle, WorkflowHandleKind, WorkflowHandleSide, WorkflowNode, pretty_block_type,
    workflow_node_accent_color, workflow_node_icon, workflow_start_node_variables,
};
use crate::apps::workflow::state::WorkflowCanvasContextMenuTarget;
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, Image, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, alignment, mouse};
use std::collections::{HashMap, HashSet};
use unicode_width::UnicodeWidthChar;

mod handles;
mod render;
mod utils;

#[cfg(test)]
#[path = "handles_tests.rs"]
mod handles_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;

use handles::*;
use render::*;
use utils::*;

pub(crate) fn export_svg(document: &WorkflowDocument) -> String {
    utils::export_svg(document)
}

#[derive(Debug, Default)]
pub struct WorkflowCanvasState {
    drag_mode: DragMode,
    last_cursor: Option<Point>,
    hovered_node_id: Option<String>,
    hovered_edge_id: Option<String>,
    hovered_handle: Option<WorkflowConnectionEndpoint>,
}

#[derive(Debug, Clone, Default)]
enum DragMode {
    #[default]
    None,
    Pan,
    Node(String),
    Connection(WorkflowConnectionEndpoint),
}

pub struct WorkflowCanvas<'a> {
    pub document: &'a WorkflowDocument,
    pub pan: Vector,
    pub zoom: f32,
    pub selected_node_id: Option<&'a str>,
    pub selected_edge_id: Option<&'a str>,
    pub connection_draft: Option<&'a WorkflowConnectionDraft>,
}

impl<'a> canvas::Program<Message> for WorkflowCanvas<'a> {
    type State = WorkflowCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_pos = cursor.position_in(bounds);
        let handle_slots = build_handle_slots(self.document);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let cursor_pos = cursor_pos?;
                state.last_cursor = Some(cursor_pos);

                if let Some(endpoint) = self.hit_test_handle(cursor_pos, &handle_slots) {
                    state.drag_mode = DragMode::Connection(endpoint.clone());
                    state.hovered_handle = Some(endpoint.clone());
                    return Some(Action::publish(Message::WorkflowTool(
                        WorkflowMessage::StartConnection(
                            endpoint,
                            world_from_screen(cursor_pos, self.pan, self.zoom),
                        ),
                    )));
                }

                if let Some(node_id) = self.hit_test_node(cursor_pos) {
                    state.drag_mode = DragMode::Node(node_id.clone());
                    return Some(Action::publish(Message::WorkflowTool(
                        WorkflowMessage::NodeDragStart(node_id),
                    )));
                }

                if let Some(edge_id) = self.hit_test_edge(cursor_pos, &handle_slots) {
                    state.drag_mode = DragMode::None;
                    return Some(Action::publish(Message::WorkflowTool(
                        WorkflowMessage::SelectEdge(edge_id),
                    )));
                }

                state.drag_mode = DragMode::Pan;
                Some(Action::publish(Message::WorkflowTool(WorkflowMessage::ClearSelection)))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let cursor_pos = cursor_pos?;
                let world = world_from_screen(cursor_pos, self.pan, self.zoom);

                state.drag_mode = DragMode::None;
                state.last_cursor = Some(cursor_pos);

                let (target, menu_size) = if let Some(node_id) = self.hit_test_node(cursor_pos) {
                    (WorkflowCanvasContextMenuTarget::Node(node_id), Size::new(228.0, 208.0))
                } else if let Some(edge_id) = self.hit_test_edge(cursor_pos, &handle_slots) {
                    (WorkflowCanvasContextMenuTarget::Edge(edge_id), Size::new(228.0, 84.0))
                } else {
                    (WorkflowCanvasContextMenuTarget::Canvas, Size::new(280.0, 332.0))
                };

                let anchor = Point::new(
                    cursor_pos.x.clamp(12.0, (bounds.width - menu_size.width).max(12.0)),
                    cursor_pos.y.clamp(12.0, (bounds.height - menu_size.height).max(12.0)),
                );

                Some(Action::publish(Message::WorkflowTool(
                    WorkflowMessage::OpenCanvasContextMenu(target, anchor, world),
                )))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let action = match state.drag_mode.clone() {
                    DragMode::Connection(origin) => {
                        if let Some(cursor_pos) = cursor_pos {
                            if let Some(endpoint) = self.hit_test_handle(cursor_pos, &handle_slots)
                            {
                                if endpoint.node_id != origin.node_id
                                    || endpoint.handle_id != origin.handle_id
                                {
                                    Some(Action::publish(Message::WorkflowTool(
                                        WorkflowMessage::FinishConnection(endpoint),
                                    )))
                                } else {
                                    Some(Action::publish(Message::WorkflowTool(
                                        WorkflowMessage::CancelConnection,
                                    )))
                                }
                            } else {
                                Some(Action::publish(Message::WorkflowTool(
                                    WorkflowMessage::CancelConnection,
                                )))
                            }
                        } else {
                            Some(Action::publish(Message::WorkflowTool(
                                WorkflowMessage::CancelConnection,
                            )))
                        }
                    }
                    DragMode::Node(_) => Some(Action::publish(Message::WorkflowTool(
                        WorkflowMessage::FinishNodeDrag,
                    ))),
                    _ => Some(Action::capture()),
                };
                state.drag_mode = DragMode::None;
                state.last_cursor = None;
                action
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let cursor_pos = cursor_pos?;
                let last = state.last_cursor.unwrap_or(cursor_pos);
                state.last_cursor = Some(cursor_pos);
                let delta = Vector::new(cursor_pos.x - last.x, cursor_pos.y - last.y);

                match &state.drag_mode {
                    DragMode::Pan => {
                        Some(Action::publish(Message::WorkflowTool(WorkflowMessage::PanBy(delta))))
                    }
                    DragMode::Node(node_id) => {
                        Some(Action::publish(Message::WorkflowTool(WorkflowMessage::NodeDragged(
                            node_id.clone(),
                            Vector::new(
                                delta.x / self.zoom.max(0.0001),
                                delta.y / self.zoom.max(0.0001),
                            ),
                        ))))
                    }
                    DragMode::Connection(_) => Some(Action::publish(Message::WorkflowTool(
                        WorkflowMessage::UpdateConnectionCursor(world_from_screen(
                            cursor_pos, self.pan, self.zoom,
                        )),
                    ))),
                    DragMode::None => {
                        let hovered = self.hit_test_node(cursor_pos);
                        let hovered_edge = self.hit_test_edge(cursor_pos, &handle_slots);
                        let hovered_handle = self.hit_test_handle(cursor_pos, &handle_slots);
                        if hovered != state.hovered_node_id
                            || hovered_edge != state.hovered_edge_id
                            || hovered_handle != state.hovered_handle
                        {
                            state.hovered_node_id = hovered;
                            state.hovered_edge_id = hovered_edge;
                            state.hovered_handle = hovered_handle;
                            Some(Action::request_redraw())
                        } else {
                            None
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let (dx, dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => (*x * 60.0, *y * 60.0),
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                };

                if dx.abs() < f32::EPSILON && dy.abs() < f32::EPSILON {
                    return None;
                }

                Some(Action::publish(Message::WorkflowTool(WorkflowMessage::PanBy(Vector::new(
                    dx, dy,
                )))))
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state.drag_mode {
            DragMode::Pan | DragMode::Node(_) => mouse::Interaction::Grabbing,
            DragMode::Connection(_) => mouse::Interaction::Pointer,
            DragMode::None => {
                let Some(point) = cursor.position_in(bounds) else {
                    return mouse::Interaction::Grab;
                };
                if self
                    .hit_test_handle(point, &build_handle_slots(self.document))
                    .or_else(|| {
                        self.hit_test_node(point).map(|node_id| WorkflowConnectionEndpoint {
                            node_id,
                            handle_id: String::new(),
                            kind: WorkflowHandleKind::Source,
                        })
                    })
                    .is_some()
                    || self.hit_test_edge(point, &build_handle_slots(self.document)).is_some()
                {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::Grab
                }
            }
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let background = if theme_is_dark(theme) {
            theme.extended_palette().background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(248, 250, 253, 1.0)
        };
        let handle_slots = build_handle_slots(self.document);

        frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), background);
        draw_grid(&mut frame, bounds.size(), self.pan, self.zoom, theme);
        draw_edges(
            &mut frame,
            self.document,
            self.pan,
            self.zoom,
            theme,
            self.selected_node_id,
            self.selected_edge_id,
            state.hovered_edge_id.as_deref(),
            &handle_slots,
        );
        draw_connection_draft(
            &mut frame,
            self.document,
            self.pan,
            self.zoom,
            self.connection_draft,
            &handle_slots,
        );
        draw_nodes(
            &mut frame,
            self.document,
            self.pan,
            self.zoom,
            self.selected_node_id,
            self.selected_edge_id,
            state.hovered_node_id.as_deref(),
            state.hovered_handle.as_ref(),
            theme,
            background,
            &handle_slots,
        );

        vec![frame.into_geometry()]
    }
}

impl WorkflowCanvas<'_> {
    fn hit_test_node(&self, cursor_pos: Point) -> Option<String> {
        let mut nodes = self.document.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));

        for node in nodes.into_iter().rev() {
            if node_screen_rect(node, self.pan, self.zoom).contains(cursor_pos) {
                return Some(node.id.clone());
            }
        }

        None
    }

    fn hit_test_handle(
        &self,
        cursor_pos: Point,
        handle_slots: &HandleSlots,
    ) -> Option<WorkflowConnectionEndpoint> {
        let mut nodes = self.document.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));

        for node in nodes.into_iter().rev() {
            for handle in &node.source_handles {
                if handle_bounds(node, handle, handle_slots, self.pan, self.zoom)
                    .contains(cursor_pos)
                {
                    return Some(WorkflowConnectionEndpoint {
                        node_id: node.id.clone(),
                        handle_id: handle.id.clone(),
                        kind: WorkflowHandleKind::Source,
                    });
                }
            }

            for handle in &node.target_handles {
                if handle_bounds(node, handle, handle_slots, self.pan, self.zoom)
                    .contains(cursor_pos)
                {
                    return Some(WorkflowConnectionEndpoint {
                        node_id: node.id.clone(),
                        handle_id: handle.id.clone(),
                        kind: WorkflowHandleKind::Target,
                    });
                }
            }
        }

        None
    }

    fn hit_test_edge(&self, cursor_pos: Point, handle_slots: &HandleSlots) -> Option<String> {
        let node_map = self
            .document
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<HashMap<_, _>>();
        let mut edges = self.document.edges.iter().collect::<Vec<_>>();
        edges.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));

        for edge in edges.into_iter().rev() {
            let Some(source_node) = node_map.get(edge.source.as_str()).copied() else {
                continue;
            };
            let Some(target_node) = node_map.get(edge.target.as_str()).copied() else {
                continue;
            };

            let start = anchor_for_handle(
                source_node,
                WorkflowHandleKind::Source,
                edge.source_handle.as_deref().unwrap_or("source"),
                handle_slots,
                self.pan,
                self.zoom,
            );
            let end = anchor_for_handle(
                target_node,
                WorkflowHandleKind::Target,
                edge.target_handle.as_deref().unwrap_or("target"),
                handle_slots,
                self.pan,
                self.zoom,
            );
            let distance = ((end.x - start.x).abs() + (end.y - start.y).abs()) * 0.35;
            let control_distance = distance.clamp(28.0, 220.0);
            let c1 = control_for_side(start, source_node.source_side, control_distance);
            let c2 = control_for_side(end, target_node.target_side, control_distance);

            if bezier_hit_test(cursor_pos, start, c1, c2, end, 9.0) {
                return Some(edge.id.clone());
            }
        }

        None
    }
}
