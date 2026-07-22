//! # Workflow 画布句柄
//!
//! 该模块负责节点句柄的布局计算、锚点定位、连线命中判断与句柄可视化绘制细节。

use super::*;

#[derive(Clone, Copy)]
struct Slot {
    index: usize,
    total: usize,
}

#[derive(Default)]
pub(super) struct HandleSlots {
    sources: HashMap<String, HashMap<String, Slot>>,
    targets: HashMap<String, HashMap<String, Slot>>,
}

pub(super) fn build_handle_slots(document: &WorkflowDocument) -> HandleSlots {
    let mut sources: HashMap<String, Vec<String>> = HashMap::new();
    let mut targets: HashMap<String, Vec<String>> = HashMap::new();

    for node in &document.nodes {
        for handle in &node.source_handles {
            sources.entry(node.id.clone()).or_default().push(handle.id.clone());
        }

        for handle in &node.target_handles {
            targets.entry(node.id.clone()).or_default().push(handle.id.clone());
        }
    }

    for edge in &document.edges {
        sources
            .entry(edge.source.clone())
            .or_default()
            .push(edge.source_handle.clone().unwrap_or_else(|| "source".to_string()));
        targets
            .entry(edge.target.clone())
            .or_default()
            .push(edge.target_handle.clone().unwrap_or_else(|| "target".to_string()));
    }

    HandleSlots { sources: build_slot_map(sources), targets: build_slot_map(targets) }
}

fn build_slot_map(source: HashMap<String, Vec<String>>) -> HashMap<String, HashMap<String, Slot>> {
    let mut slots = HashMap::new();

    for (node_id, mut handles) in source {
        handles.sort();
        handles.dedup();

        let total = handles.len().max(1);
        let per_node = handles
            .into_iter()
            .enumerate()
            .map(|(index, handle)| (handle, Slot { index, total }))
            .collect::<HashMap<_, _>>();
        slots.insert(node_id, per_node);
    }

    slots
}

fn lookup_slot(
    slots: &HashMap<String, HashMap<String, Slot>>,
    node_id: &str,
    handle: &str,
) -> Slot {
    slots
        .get(node_id)
        .and_then(|per_node| per_node.get(handle).copied())
        .unwrap_or(Slot { index: 0, total: 1 })
}

pub(super) fn node_screen_rect(node: &WorkflowNode, pan: Vector, zoom: f32) -> Rectangle {
    Rectangle::new(
        screen_from_world(node.position, pan, zoom),
        Size::new(node.size.width * zoom, node.size.height * zoom),
    )
}

fn handle_anchor(rect: Rectangle, side: WorkflowHandleSide, slot: Slot) -> Point {
    let step_x = rect.width / (slot.total + 1) as f32;
    let step_y = rect.height / (slot.total + 1) as f32;

    match side {
        WorkflowHandleSide::Left => Point::new(rect.x, rect.y + step_y * (slot.index + 1) as f32),
        WorkflowHandleSide::Right => {
            Point::new(rect.x + rect.width, rect.y + step_y * (slot.index + 1) as f32)
        }
        WorkflowHandleSide::Top => Point::new(rect.x + step_x * (slot.index + 1) as f32, rect.y),
        WorkflowHandleSide::Bottom => {
            Point::new(rect.x + step_x * (slot.index + 1) as f32, rect.y + rect.height)
        }
    }
}

pub(super) fn anchor_for_handle(
    node: &WorkflowNode,
    kind: WorkflowHandleKind,
    handle_id: &str,
    handle_slots: &HandleSlots,
    pan: Vector,
    zoom: f32,
) -> Point {
    let rect = node_screen_rect(node, pan, zoom);
    let (slots, side) = match kind {
        WorkflowHandleKind::Source => (&handle_slots.sources, node.source_side),
        WorkflowHandleKind::Target => (&handle_slots.targets, node.target_side),
    };
    let slot = lookup_slot(slots, &node.id, handle_id);
    handle_anchor(rect, side, slot)
}

pub(super) fn handle_bounds(
    node: &WorkflowNode,
    handle: &WorkflowHandle,
    handle_slots: &HandleSlots,
    pan: Vector,
    zoom: f32,
) -> Rectangle {
    let anchor = anchor_for_handle(node, handle.kind, &handle.id, handle_slots, pan, zoom);
    let hit_size = (22.0 * canvas_element_scale(zoom)).max(8.0);
    Rectangle::new(
        Point::new(anchor.x - hit_size / 2.0, anchor.y - hit_size / 2.0),
        Size::new(hit_size, hit_size),
    )
}

pub(super) fn control_for_side(point: Point, side: WorkflowHandleSide, distance: f32) -> Point {
    match side {
        WorkflowHandleSide::Left => Point::new(point.x - distance, point.y),
        WorkflowHandleSide::Right => Point::new(point.x + distance, point.y),
        WorkflowHandleSide::Top => Point::new(point.x, point.y - distance),
        WorkflowHandleSide::Bottom => Point::new(point.x, point.y + distance),
    }
}

pub(super) fn edge_handle_label(edge: &WorkflowEdge) -> Option<String> {
    let label = edge.source_handle.as_deref()?;
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed == "source" || trimmed == "target" {
        return None;
    }
    if trimmed == "true" {
        return Some("是".to_string());
    }
    if trimmed == "false" {
        return Some("否".to_string());
    }
    if trimmed.len() > 24 || trimmed.contains('-') {
        return None;
    }
    Some(trimmed.to_string())
}

pub(super) fn draw_node_handles<CanvasRenderer>(
    frame: &mut Frame<CanvasRenderer>,
    node: &WorkflowNode,
    handle_slots: &HandleSlots,
    pan: Vector,
    zoom: f32,
    accent: Color,
    background: Color,
    emphasize: bool,
    hovered_handle: Option<&WorkflowConnectionEndpoint>,
    connected_sources: Option<&HashSet<String>>,
    connected_targets: Option<&HashSet<String>>,
) where
    CanvasRenderer: iced::advanced::graphics::geometry::Renderer,
{
    for handle in &node.source_handles {
        draw_single_handle(
            frame,
            node,
            handle,
            handle_slots,
            pan,
            zoom,
            accent,
            background,
            emphasize,
            hovered_handle,
            connected_sources,
        );
    }

    for handle in &node.target_handles {
        draw_single_handle(
            frame,
            node,
            handle,
            handle_slots,
            pan,
            zoom,
            accent,
            background,
            emphasize,
            hovered_handle,
            connected_targets,
        );
    }
}

fn draw_single_handle<CanvasRenderer>(
    frame: &mut Frame<CanvasRenderer>,
    node: &WorkflowNode,
    handle: &WorkflowHandle,
    handle_slots: &HandleSlots,
    pan: Vector,
    zoom: f32,
    accent: Color,
    background: Color,
    emphasize: bool,
    hovered_handle: Option<&WorkflowConnectionEndpoint>,
    connected_handles: Option<&HashSet<String>>,
) where
    CanvasRenderer: iced::advanced::graphics::geometry::Renderer,
{
    let anchor = anchor_for_handle(node, handle.kind, &handle.id, handle_slots, pan, zoom);
    let element_scale = canvas_element_scale(zoom);
    let connected = connected_handles.is_some_and(|set| set.contains(handle.id.as_str()));
    let hovered = hovered_handle.is_some_and(|endpoint| {
        endpoint.node_id == node.id
            && endpoint.handle_id == handle.id
            && endpoint.kind == handle.kind
    });
    let visible = emphasize || hovered || connected;

    let outer_radius = if hovered {
        6.0 * element_scale
    } else if visible {
        5.2 * element_scale
    } else {
        4.2 * element_scale
    };
    let outer_path = Path::circle(anchor, outer_radius);
    frame.fill(
        &outer_path,
        blend(
            background,
            accent,
            if hovered {
                0.28
            } else if connected {
                0.20
            } else {
                0.10
            },
        ),
    );
    frame.stroke(
        &outer_path,
        Stroke::default()
            .with_color(with_alpha(
                accent,
                if hovered {
                    0.92
                } else if connected {
                    0.64
                } else if visible {
                    0.34
                } else {
                    0.18
                },
            ))
            .with_width((1.2 * element_scale).max(0.4)),
    );

    let inner_path = Path::circle(anchor, (outer_radius - 2.3 * element_scale).max(0.8));
    frame.fill(&inner_path, if background.a < 0.99 { Color::WHITE } else { background });

    if visible && !handle.label.is_empty() {
        let chip_font_size = (11.0 * element_scale).max(4.0);
        let chip_padding_x = 8.0 * element_scale;
        let chip_height = (20.0 * element_scale).max(8.0);
        let chip_width = (display_width(&handle.label) as f32 * chip_font_size * 0.74
            + chip_padding_x * 2.0)
            .clamp(34.0 * element_scale, 76.0 * element_scale);
        let chip_origin = match handle.kind {
            WorkflowHandleKind::Source => Point::new(
                anchor.x - chip_width - 10.0 * element_scale,
                anchor.y - chip_height / 2.0,
            ),
            WorkflowHandleKind::Target => {
                Point::new(anchor.x + 10.0 * element_scale, anchor.y - chip_height / 2.0)
            }
        };
        let chip_rect = Rectangle::new(chip_origin, Size::new(chip_width, chip_height));
        let chip_path = Path::rounded_rectangle(
            chip_rect.position(),
            chip_rect.size(),
            (chip_height / 2.0).into(),
        );
        frame.fill(&chip_path, blend(Color::WHITE, accent, 0.08));
        frame.stroke(
            &chip_path,
            Stroke::default()
                .with_color(with_alpha(accent, 0.18))
                .with_width((1.0 * element_scale).max(0.4)),
        );
        frame.fill_text(Text {
            content: handle.label.clone(),
            position: Point::new(
                chip_rect.x + chip_rect.width / 2.0,
                chip_rect.y + chip_rect.height / 2.0,
            ),
            color: accent,
            size: Pixels(chip_font_size),
            align_x: iced::widget::text::Alignment::Center,
            align_y: alignment::Vertical::Center,
            ..Text::default()
        });
    }
}

pub(super) fn connected_handles(
    document: &WorkflowDocument,
) -> (HashMap<&str, HashSet<String>>, HashMap<&str, HashSet<String>>) {
    let mut sources = HashMap::<&str, HashSet<String>>::new();
    let mut targets = HashMap::<&str, HashSet<String>>::new();

    for edge in &document.edges {
        sources
            .entry(edge.source.as_str())
            .or_default()
            .insert(edge.source_handle.clone().unwrap_or_else(|| "source".to_string()));
        targets
            .entry(edge.target.as_str())
            .or_default()
            .insert(edge.target_handle.clone().unwrap_or_else(|| "target".to_string()));
    }

    (sources, targets)
}

pub(super) fn bezier_hit_test(
    point: Point,
    start: Point,
    c1: Point,
    c2: Point,
    end: Point,
    tolerance: f32,
) -> bool {
    let mut last = start;
    for step in 1..=18 {
        let current = cubic_bezier_point(start, c1, c2, end, step as f32 / 18.0);
        if distance_to_segment(point, last, current) <= tolerance {
            return true;
        }
        last = current;
    }
    false
}
