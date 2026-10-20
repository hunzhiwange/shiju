//! # Workflow 画布渲染
//!
//! 该模块负责网格、节点、连线以及临时连线草稿的具体画布渲染逻辑。

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GridPlan {
    pub(super) spacing: f32,
    pub(super) start_x: i32,
    pub(super) end_x: i32,
    pub(super) start_y: i32,
    pub(super) end_y: i32,
    pub(super) dot_color: Color,
    pub(super) major_dot_color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EdgePlan {
    pub(super) start: Point,
    pub(super) end: Point,
    pub(super) c1: Point,
    pub(super) c2: Point,
    pub(super) color: Color,
    pub(super) width: f32,
    pub(super) selected: bool,
    pub(super) hovered: bool,
}

#[derive(Clone, Copy)]
pub(super) struct EdgePlanInput<'a> {
    pub(super) node_map: &'a HashMap<&'a str, &'a WorkflowNode>,
    pub(super) pan: Vector,
    pub(super) zoom: f32,
    pub(super) theme: &'a Theme,
    pub(super) selected_node_id: Option<&'a str>,
    pub(super) selected_edge_id: Option<&'a str>,
    pub(super) hovered_edge_id: Option<&'a str>,
    pub(super) handle_slots: &'a HandleSlots,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DraftPlan {
    pub(super) start: Point,
    pub(super) end: Point,
    pub(super) c1: Point,
    pub(super) c2: Point,
    pub(super) color: Color,
    pub(super) width: f32,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(super) struct NodePlan {
    pub(super) rect: Rectangle,
    pub(super) accent: Color,
    pub(super) title_lines: Vec<String>,
    pub(super) desc_lines: Vec<String>,
    pub(super) start_variable_rows: Vec<(String, String)>,
    pub(super) is_selected: bool,
    pub(super) is_hovered: bool,
    pub(super) descendant_selected: bool,
    pub(super) show_start_variables: bool,
    pub(super) content_scale: f32,
    pub(super) content_top: f32,
    pub(super) title_x: f32,
    pub(super) description_x: f32,
    pub(super) title_font_size: f32,
    pub(super) desc_font_size: f32,
}

fn node_text_x(rect: Rectangle, content_scale: f32) -> f32 {
    rect.x + (14.0 + 24.0 + 12.0) * content_scale
}

fn node_text_width(rect: Rectangle, content_scale: f32) -> f32 {
    rect.x + rect.width - node_text_x(rect, content_scale) - 14.0 * content_scale
}

pub(super) fn draw_grid(frame: &mut Frame, size: Size, pan: Vector, zoom: f32, theme: &Theme) {
    let plan = grid_plan(size, pan, zoom, theme);

    for x_index in plan.start_x..=plan.end_x {
        for y_index in plan.start_y..=plan.end_y {
            let point = screen_from_world(
                Point::new(x_index as f32 * plan.spacing, y_index as f32 * plan.spacing),
                pan,
                zoom,
            );
            let is_major = x_index.rem_euclid(4) == 0 && y_index.rem_euclid(4) == 0;
            frame.fill(
                &Path::circle(point, if is_major { 1.6 } else { 1.0 }),
                if is_major { plan.major_dot_color } else { plan.dot_color },
            );
        }
    }
}

pub(super) fn grid_plan(size: Size, pan: Vector, zoom: f32, theme: &Theme) -> GridPlan {
    let top_left_world = world_from_screen(Point::ORIGIN, pan, zoom);
    let bottom_right_world = world_from_screen(Point::new(size.width, size.height), pan, zoom);
    let min_x = top_left_world.x.min(bottom_right_world.x);
    let max_x = top_left_world.x.max(bottom_right_world.x);
    let min_y = top_left_world.y.min(bottom_right_world.y);
    let max_y = top_left_world.y.max(bottom_right_world.y);
    let spacing = grid_spacing(zoom);
    let is_dark = theme_is_dark(theme);
    let dot_color = if is_dark {
        with_alpha(theme.palette().text, 0.12)
    } else {
        Color::from_rgba8(148, 163, 184, 0.32)
    };
    let major_dot_color = if is_dark {
        with_alpha(theme.palette().text, 0.20)
    } else {
        Color::from_rgba8(100, 116, 139, 0.28)
    };

    GridPlan {
        spacing,
        start_x: (min_x / spacing).floor() as i32 - 1,
        end_x: (max_x / spacing).ceil() as i32 + 1,
        start_y: (min_y / spacing).floor() as i32 - 1,
        end_y: (max_y / spacing).ceil() as i32 + 1,
        dot_color,
        major_dot_color,
    }
}

pub(super) fn grid_spacing(zoom: f32) -> f32 {
    if zoom < 0.35 {
        144.0
    } else if zoom < 0.6 {
        72.0
    } else if zoom < 1.0 {
        48.0
    } else {
        32.0
    }
}

pub(super) fn draw_edges(
    frame: &mut Frame,
    document: &WorkflowDocument,
    pan: Vector,
    zoom: f32,
    theme: &Theme,
    selected_node_id: Option<&str>,
    selected_edge_id: Option<&str>,
    hovered_edge_id: Option<&str>,
    handle_slots: &HandleSlots,
) {
    let element_scale = canvas_element_scale(zoom);
    let node_map =
        document.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();

    let mut edges = document.edges.iter().collect::<Vec<_>>();
    edges.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));

    for edge in edges {
        let Some(plan) = edge_plan(
            edge,
            EdgePlanInput {
                node_map: &node_map,
                pan,
                zoom,
                theme,
                selected_node_id,
                selected_edge_id,
                hovered_edge_id,
                handle_slots,
            },
        ) else {
            continue;
        };
        let edge_path = Path::new(|builder| {
            builder.move_to(plan.start);
            builder.bezier_curve_to(plan.c1, plan.c2, plan.end);
        });

        frame.stroke(&edge_path, Stroke::default().with_color(plan.color).with_width(plan.width));

        if let Some(label) = edge_handle_label(edge) {
            let badge_center = cubic_bezier_point(plan.start, plan.c1, plan.c2, plan.end, 0.5);
            let badge_font_size = (12.0 * element_scale).max(4.0);
            let badge_padding_x = 9.0 * element_scale;
            let badge_height = (22.0 * element_scale).max(8.0);
            let badge_width = (label.chars().count() as f32 * badge_font_size * 0.72
                + badge_padding_x * 2.0)
                .clamp(30.0 * element_scale, 94.0 * element_scale);
            let badge_rect = Rectangle::new(
                Point::new(badge_center.x - badge_width / 2.0, badge_center.y - badge_height / 2.0),
                Size::new(badge_width, badge_height),
            );
            let badge_path = Path::rounded_rectangle(
                badge_rect.position(),
                badge_rect.size(),
                (badge_height / 2.0).into(),
            );
            let badge_fill = if theme_is_dark(theme) {
                blend(theme.extended_palette().background.base.color, Color::WHITE, 0.08)
            } else {
                Color::from_rgba8(255, 255, 255, 0.96)
            };
            frame.fill(&badge_path, badge_fill);
            frame.stroke(
                &badge_path,
                Stroke::default()
                    .with_color(with_alpha(
                        if plan.selected || plan.hovered {
                            plan.color
                        } else {
                            edge_base_color(theme)
                        },
                        0.30,
                    ))
                    .with_width((1.0 * element_scale).max(0.4)),
            );
            frame.fill_text(Text {
                content: label,
                position: Point::new(
                    badge_rect.x + badge_rect.width / 2.0,
                    badge_rect.y + badge_rect.height / 2.0,
                ),
                color: if theme_is_dark(theme) {
                    Color::WHITE.scale_alpha(0.86)
                } else if plan.selected || plan.hovered {
                    accent_color(&edge.source_type)
                } else {
                    Color::from_rgb8(71, 85, 105)
                },
                size: Pixels(badge_font_size),
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..Text::default()
            });
        }
    }
}

pub(super) fn edge_plan(edge: &WorkflowEdge, input: EdgePlanInput<'_>) -> Option<EdgePlan> {
    let source_node = input.node_map.get(edge.source.as_str()).copied()?;
    let target_node = input.node_map.get(edge.target.as_str()).copied()?;
    let start = anchor_for_handle(
        source_node,
        WorkflowHandleKind::Source,
        edge.source_handle.as_deref().unwrap_or("source"),
        input.handle_slots,
        input.pan,
        input.zoom,
    );
    let end = anchor_for_handle(
        target_node,
        WorkflowHandleKind::Target,
        edge.target_handle.as_deref().unwrap_or("target"),
        input.handle_slots,
        input.pan,
        input.zoom,
    );
    let distance = ((end.x - start.x).abs() + (end.y - start.y).abs()) * 0.35;
    let control_distance = distance.clamp(28.0, 220.0);
    let selected = input.selected_edge_id == Some(edge.id.as_str())
        || input
            .selected_node_id
            .is_some_and(|node_id| node_id == edge.source || node_id == edge.target);
    let hovered = input.hovered_edge_id == Some(edge.id.as_str());
    let color = if selected {
        accent_color(&edge.source_type)
    } else if hovered {
        with_alpha(accent_color(&edge.source_type), 0.82)
    } else {
        edge_base_color(input.theme)
    };
    let element_scale = canvas_element_scale(input.zoom);
    let width = if selected {
        2.8 * element_scale
    } else if hovered {
        2.2 * element_scale
    } else {
        1.6 * element_scale
    };

    Some(EdgePlan {
        start,
        end,
        c1: control_for_side(start, source_node.source_side, control_distance),
        c2: control_for_side(end, target_node.target_side, control_distance),
        color,
        width,
        selected,
        hovered,
    })
}

pub(super) fn edge_base_color(theme: &Theme) -> Color {
    if theme_is_dark(theme) {
        with_alpha(theme.palette().text, 0.22)
    } else {
        Color::from_rgba8(148, 163, 184, 0.88)
    }
}

pub(super) fn draw_connection_draft(
    frame: &mut Frame,
    document: &WorkflowDocument,
    pan: Vector,
    zoom: f32,
    draft: Option<&WorkflowConnectionDraft>,
    handle_slots: &HandleSlots,
) {
    let Some(plan) = draft_plan(document, pan, zoom, draft, handle_slots) else {
        return;
    };
    let draft_path = Path::new(|builder| {
        builder.move_to(plan.start);
        builder.bezier_curve_to(plan.c1, plan.c2, plan.end);
    });

    frame.stroke(&draft_path, Stroke::default().with_color(plan.color).with_width(plan.width));
}

pub(super) fn draft_plan(
    document: &WorkflowDocument,
    pan: Vector,
    zoom: f32,
    draft: Option<&WorkflowConnectionDraft>,
    handle_slots: &HandleSlots,
) -> Option<DraftPlan> {
    let draft = draft?;
    let node = document.node(&draft.from.node_id)?;
    let start =
        anchor_for_handle(node, draft.from.kind, &draft.from.handle_id, handle_slots, pan, zoom);
    let end = screen_from_world(draft.cursor_world, pan, zoom);
    let start_side = match draft.from.kind {
        WorkflowHandleKind::Source => node.source_side,
        WorkflowHandleKind::Target => node.target_side,
    };
    let end_side = match draft.from.kind {
        WorkflowHandleKind::Source => WorkflowHandleSide::Left,
        WorkflowHandleKind::Target => WorkflowHandleSide::Right,
    };
    let distance = ((end.x - start.x).abs() + (end.y - start.y).abs()) * 0.35;
    let control_distance = distance.clamp(28.0, 180.0);

    Some(DraftPlan {
        start,
        end,
        c1: control_for_side(start, start_side, control_distance),
        c2: control_for_side(end, end_side, control_distance),
        color: with_alpha(accent_color(&node.block_type), 0.86),
        width: 2.2 * canvas_element_scale(zoom),
    })
}

pub(super) fn draw_nodes(
    frame: &mut Frame,
    document: &WorkflowDocument,
    pan: Vector,
    zoom: f32,
    selected_node_id: Option<&str>,
    selected_edge_id: Option<&str>,
    hovered_node_id: Option<&str>,
    hovered_handle: Option<&WorkflowConnectionEndpoint>,
    theme: &Theme,
    background: Color,
    handle_slots: &HandleSlots,
) {
    let mut nodes = document.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));
    let selected_group_ids = selected_node_id
        .map(|node_id| document.ancestor_ids(node_id).into_iter().collect::<HashSet<_>>())
        .unwrap_or_default();
    let (connected_sources, connected_targets) = connected_handles(document);
    let is_dark = theme_is_dark(theme);
    let text_primary =
        if is_dark { Color::WHITE.scale_alpha(0.92) } else { Color::from_rgb8(15, 23, 42) };
    let text_secondary =
        if is_dark { Color::WHITE.scale_alpha(0.64) } else { Color::from_rgba8(71, 85, 105, 0.92) };
    let content_scale = zoom.max(0.3);
    let icon_offset_x = 14.0 * content_scale;
    let icon_size = 24.0 * content_scale;
    let icon_inset = (4.0 * content_scale).max(1.2);
    let icon_corner_radius = 11.0 * content_scale;
    let card_padding_x = 14.0 * content_scale;
    let desc_gap = 10.0 * content_scale;
    let text_line_gap = 3.0 * content_scale;
    let start_variable_row_height = 34.0 * content_scale;
    let start_variable_row_gap = 8.0 * content_scale;
    let start_variable_top_gap = 14.0 * content_scale;
    let start_variable_row_padding = 12.0 * content_scale;
    let start_variable_badge_size = 18.0 * content_scale;
    let start_variable_badge_gap = 10.0 * content_scale;

    for node in nodes {
        let rect = node_screen_rect(node, pan, zoom);
        let accent = accent_color(&node.block_type);
        let icon = workflow_node_icon(&node.block_type);
        let is_selected = selected_node_id == Some(node.id.as_str());
        let is_hovered = hovered_node_id == Some(node.id.as_str());
        let descendant_selected = selected_group_ids.contains(node.id.as_str());
        let description = node_description_text(document, node);
        let start_variables = workflow_start_node_variables(node);
        let show_start_variables = node.block_type == "start" && !start_variables.is_empty();
        let title_font_size = if rect.height < 70.0 { 13.0 } else { 15.0 } * content_scale;
        let title_line_step = title_font_size + text_line_gap;
        let desc_font_size = if rect.height > 140.0 { 12.0 } else { 11.0 } * content_scale;
        let desc_line_step = desc_font_size + text_line_gap;
        let title_text = if node.title.trim().is_empty() {
            pretty_block_type(&node.block_type)
        } else {
            node.title.clone()
        };
        let title_max_chars = ((node_text_width(rect, content_scale) / (title_font_size * 0.63))
            .floor()
            .max(8.0)) as usize;
        let title_lines =
            wrap_text_lines(&title_text, title_max_chars, if rect.height > 120.0 { 2 } else { 1 });
        let desc_max_chars = ((node_text_width(rect, content_scale) / (desc_font_size * 0.62))
            .floor()
            .max(10.0)) as usize;
        let desc_lines = if !show_start_variables
            && rect.height >= 84.0
            && !description.trim().is_empty()
        {
            wrap_text_lines(&description, desc_max_chars, if rect.height > 140.0 { 3 } else { 2 })
        } else {
            Vec::new()
        };
        let title_block_height =
            line_block_height(title_lines.len(), title_font_size, title_line_step);
        let desc_block_height = line_block_height(desc_lines.len(), desc_font_size, desc_line_step);
        let start_variable_block_height = if show_start_variables {
            start_variable_top_gap
                + start_variables.len() as f32 * start_variable_row_height
                + start_variables.len().saturating_sub(1) as f32 * start_variable_row_gap
        } else {
            0.0
        };
        let text_block_height = if show_start_variables {
            title_block_height + start_variable_block_height
        } else {
            title_block_height
                + if desc_lines.is_empty() { 0.0 } else { desc_gap + desc_block_height }
        };
        let content_block_height = if show_start_variables {
            icon_size.max(title_block_height) + start_variable_block_height
        } else {
            icon_size.max(text_block_height)
        };
        let content_top = if show_start_variables {
            rect.y + 18.0 * content_scale
        } else {
            rect.y + (rect.height - content_block_height).max(0.0) / 2.0
        };

        let fill = if is_dark {
            blend(background, Color::WHITE, 0.06)
        } else {
            Color::from_rgba8(255, 255, 255, 0.98)
        };
        let border = if is_selected {
            accent
        } else if descendant_selected {
            with_alpha(accent, 0.55)
        } else if is_hovered {
            with_alpha(accent, 0.38)
        } else {
            if is_dark {
                with_alpha(theme.palette().text, 0.18)
            } else {
                Color::from_rgba8(148, 163, 184, 0.34)
            }
        };

        let radius = 20.0;
        let shadow_rect = Rectangle::new(Point::new(rect.x, rect.y + 8.0), rect.size());
        let shadow_path =
            Path::rounded_rectangle(shadow_rect.position(), shadow_rect.size(), radius.into());
        frame.fill(
            &shadow_path,
            Color::from_rgba8(
                15,
                23,
                42,
                if is_selected {
                    if is_dark { 22.0 / 255.0 } else { 18.0 / 255.0 }
                } else if is_hovered {
                    if is_dark { 18.0 / 255.0 } else { 14.0 / 255.0 }
                } else if is_dark {
                    12.0 / 255.0
                } else {
                    9.0 / 255.0
                },
            ),
        );

        if is_selected {
            let halo_rect = Rectangle::new(
                Point::new(rect.x - 4.0, rect.y - 4.0),
                Size::new(rect.width + 8.0, rect.height + 8.0),
            );
            let halo_path = Path::rounded_rectangle(
                halo_rect.position(),
                halo_rect.size(),
                (radius + 4.0).into(),
            );
            frame.fill(&halo_path, with_alpha(accent, if is_dark { 0.10 } else { 0.08 }));
        }

        let node_path = Path::rounded_rectangle(rect.position(), rect.size(), radius.into());
        frame.fill(&node_path, fill);
        frame.stroke(
            &node_path,
            Stroke::default().with_color(border).with_width(if is_selected { 2.0 } else { 1.1 }),
        );

        let icon_rect = Rectangle::new(
            Point::new(
                rect.x + icon_offset_x,
                if show_start_variables {
                    content_top
                } else {
                    content_top + (content_block_height - icon_size).max(0.0) / 2.0
                },
            ),
            Size::new(icon_size, icon_size),
        );
        let icon_path = Path::rounded_rectangle(
            icon_rect.position(),
            icon_rect.size(),
            icon_corner_radius.into(),
        );
        frame.fill(&icon_path, blend(fill, accent, if is_dark { 0.24 } else { 0.16 }));
        frame.stroke(
            &icon_path,
            Stroke::default()
                .with_color(with_alpha(accent, 0.18))
                .with_width((1.0 * content_scale).clamp(0.7, 2.0)),
        );
        let image_rect = Rectangle::new(
            Point::new(icon_rect.x + icon_inset, icon_rect.y + icon_inset),
            Size::new(
                (icon_rect.width - icon_inset * 2.0).max(1.0),
                (icon_rect.height - icon_inset * 2.0).max(1.0),
            ),
        );
        if let Some(handle) = assets::get_named_icon_image(icon.family, icon.name, accent) {
            frame.draw_image(image_rect, Image::new(handle));
        } else {
            frame.fill_text(Text {
                content: node_glyph(&node.block_type).to_string(),
                position: Point::new(
                    icon_rect.x + icon_rect.width / 2.0,
                    icon_rect.y + icon_rect.height / 2.0,
                ),
                color: accent,
                size: Pixels((12.0 * content_scale).max(4.0)),
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..Text::default()
            });
        }

        let title_x = node_text_x(rect, content_scale);
        let title_y = if show_start_variables {
            icon_rect.y + (icon_rect.height - title_block_height).max(0.0) / 2.0
        } else {
            content_top + (content_block_height - text_block_height).max(0.0) / 2.0
        };

        for (index, line) in title_lines.iter().enumerate() {
            frame.fill_text(Text {
                content: line.clone(),
                position: Point::new(title_x, title_y + index as f32 * title_line_step),
                color: text_primary,
                size: Pixels(title_font_size),
                align_x: iced::widget::text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                ..Text::default()
            });
        }

        if show_start_variables {
            let row_width = rect.width - card_padding_x * 2.0;
            let row_label_max_chars = ((row_width
                - start_variable_row_padding * 2.0
                - start_variable_badge_size
                - start_variable_badge_gap)
                / (12.0 * content_scale * 0.62))
                .floor()
                .max(6.0) as usize;
            let row_fill = if is_dark {
                blend(fill, Color::WHITE, 0.08)
            } else {
                Color::from_rgba8(241, 245, 249, 0.96)
            };
            let row_border = if is_dark {
                with_alpha(Color::WHITE, 0.10)
            } else {
                Color::from_rgba8(203, 213, 225, 0.72)
            };

            for (index, variable) in start_variables.iter().enumerate() {
                let row_rect = Rectangle::new(
                    Point::new(
                        rect.x + card_padding_x,
                        icon_rect.y
                            + icon_rect.height
                            + start_variable_top_gap
                            + index as f32 * (start_variable_row_height + start_variable_row_gap),
                    ),
                    Size::new(row_width, start_variable_row_height),
                );
                let row_path = Path::rounded_rectangle(
                    row_rect.position(),
                    row_rect.size(),
                    (12.0 * content_scale).into(),
                );
                let badge_rect = Rectangle::new(
                    Point::new(
                        row_rect.x + row_rect.width
                            - start_variable_row_padding
                            - start_variable_badge_size,
                        row_rect.y + (row_rect.height - start_variable_badge_size).max(0.0) / 2.0,
                    ),
                    Size::new(start_variable_badge_size, start_variable_badge_size),
                );
                let badge_path = Path::rounded_rectangle(
                    badge_rect.position(),
                    badge_rect.size(),
                    (6.0 * content_scale).into(),
                );
                let row_label = wrap_text_lines(&variable.name, row_label_max_chars, 1)
                    .into_iter()
                    .next()
                    .unwrap_or_default();

                frame.fill(&row_path, row_fill);
                frame.stroke(
                    &row_path,
                    Stroke::default()
                        .with_color(row_border)
                        .with_width((1.0 * content_scale).clamp(0.7, 1.6)),
                );
                frame.fill_text(Text {
                    content: row_label,
                    position: Point::new(
                        row_rect.x + start_variable_row_padding,
                        row_rect.y + row_rect.height / 2.0,
                    ),
                    color: text_primary,
                    size: Pixels(12.0 * content_scale),
                    align_x: iced::widget::text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                    ..Text::default()
                });
                frame.fill(&badge_path, blend(row_fill, accent, if is_dark { 0.18 } else { 0.12 }));
                frame.stroke(
                    &badge_path,
                    Stroke::default()
                        .with_color(with_alpha(accent, 0.20))
                        .with_width((1.0 * content_scale).clamp(0.7, 1.4)),
                );
                frame.fill_text(Text {
                    content: start_variable_badge_text(&variable.value_type).to_string(),
                    position: Point::new(
                        badge_rect.x + badge_rect.width / 2.0,
                        badge_rect.y + badge_rect.height / 2.0,
                    ),
                    color: accent,
                    size: Pixels((10.0 * content_scale).max(4.0)),
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..Text::default()
                });
            }
        } else if !desc_lines.is_empty() {
            let desc_y = title_y + title_block_height + desc_gap;

            for (index, line) in desc_lines.iter().enumerate() {
                frame.fill_text(Text {
                    content: line.clone(),
                    position: Point::new(title_x, desc_y + index as f32 * desc_line_step),
                    color: text_secondary,
                    size: Pixels(desc_font_size),
                    align_x: iced::widget::text::Alignment::Left,
                    align_y: alignment::Vertical::Top,
                    ..Text::default()
                });
            }
        }

        draw_node_handles(
            frame,
            node,
            handle_slots,
            pan,
            zoom,
            accent,
            background,
            is_selected || descendant_selected || is_hovered || selected_edge_id.is_some(),
            hovered_handle,
            connected_sources.get(node.id.as_str()),
            connected_targets.get(node.id.as_str()),
        );
    }
}

#[allow(dead_code)]
pub(super) fn node_plan(
    document: &WorkflowDocument,
    node: &WorkflowNode,
    pan: Vector,
    zoom: f32,
    selected_node_id: Option<&str>,
    hovered_node_id: Option<&str>,
    selected_group_ids: &HashSet<&str>,
) -> NodePlan {
    let rect = node_screen_rect(node, pan, zoom);
    let content_scale = zoom.max(0.3);
    let icon_size = 24.0 * content_scale;
    let desc_gap = 10.0 * content_scale;
    let text_line_gap = 3.0 * content_scale;
    let start_variable_row_height = 34.0 * content_scale;
    let start_variable_row_gap = 8.0 * content_scale;
    let start_variable_top_gap = 14.0 * content_scale;
    let start_variables = workflow_start_node_variables(node);
    let show_start_variables = node.block_type == "start" && !start_variables.is_empty();
    let title_font_size = if rect.height < 70.0 { 13.0 } else { 15.0 } * content_scale;
    let title_line_step = title_font_size + text_line_gap;
    let desc_font_size = if rect.height > 140.0 { 12.0 } else { 11.0 } * content_scale;
    let desc_line_step = desc_font_size + text_line_gap;
    let title_text = if node.title.trim().is_empty() {
        pretty_block_type(&node.block_type)
    } else {
        node.title.clone()
    };
    let title_max_chars = ((node_text_width(rect, content_scale) / (title_font_size * 0.63))
        .floor()
        .max(8.0)) as usize;
    let title_lines =
        wrap_text_lines(&title_text, title_max_chars, if rect.height > 120.0 { 2 } else { 1 });
    let description = node_description_text(document, node);
    let desc_max_chars = ((node_text_width(rect, content_scale) / (desc_font_size * 0.62))
        .floor()
        .max(10.0)) as usize;
    let desc_lines =
        if !show_start_variables && rect.height >= 84.0 && !description.trim().is_empty() {
            wrap_text_lines(&description, desc_max_chars, if rect.height > 140.0 { 3 } else { 2 })
        } else {
            Vec::new()
        };
    let title_block_height = line_block_height(title_lines.len(), title_font_size, title_line_step);
    let desc_block_height = line_block_height(desc_lines.len(), desc_font_size, desc_line_step);
    let start_variable_block_height = if show_start_variables {
        start_variable_top_gap
            + start_variables.len() as f32 * start_variable_row_height
            + start_variables.len().saturating_sub(1) as f32 * start_variable_row_gap
    } else {
        0.0
    };
    let text_block_height = if show_start_variables {
        title_block_height + start_variable_block_height
    } else {
        title_block_height + if desc_lines.is_empty() { 0.0 } else { desc_gap + desc_block_height }
    };
    let content_block_height = if show_start_variables {
        icon_size.max(title_block_height) + start_variable_block_height
    } else {
        icon_size.max(text_block_height)
    };
    let content_top = if show_start_variables {
        rect.y + 18.0 * content_scale
    } else {
        rect.y + (rect.height - content_block_height).max(0.0) / 2.0
    };

    NodePlan {
        rect,
        accent: accent_color(&node.block_type),
        title_lines,
        desc_lines,
        start_variable_rows: start_variables
            .into_iter()
            .map(|variable| {
                (variable.name, start_variable_badge_text(&variable.value_type).to_string())
            })
            .collect(),
        is_selected: selected_node_id == Some(node.id.as_str()),
        is_hovered: hovered_node_id == Some(node.id.as_str()),
        descendant_selected: selected_group_ids.contains(node.id.as_str()),
        show_start_variables,
        content_scale,
        content_top,
        title_x: node_text_x(rect, content_scale),
        description_x: node_text_x(rect, content_scale),
        title_font_size,
        desc_font_size,
    }
}
