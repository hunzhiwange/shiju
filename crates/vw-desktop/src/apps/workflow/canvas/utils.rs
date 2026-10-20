//! # Workflow 画布工具
//!
//! 该模块提供画布渲染与导出所需的几何、文本、颜色和 SVG 导出辅助函数。

use super::*;
use serde_yaml::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::fmt::Write as _;

pub(super) fn distance_to_segment(point: Point, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;

    if dx.abs() < f32::EPSILON && dy.abs() < f32::EPSILON {
        return ((point.x - start.x).powi(2) + (point.y - start.y).powi(2)).sqrt();
    }

    let t = (((point.x - start.x) * dx) + ((point.y - start.y) * dy)) / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);
    let projection = Point::new(start.x + t * dx, start.y + t * dy);
    ((point.x - projection.x).powi(2) + (point.y - projection.y).powi(2)).sqrt()
}

pub(super) fn wrap_text_lines(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    let clean = text.trim();
    if clean.is_empty() {
        return vec![String::new()];
    }

    let max_width = max_width.max(4);
    let max_lines = max_lines.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut truncated = false;

    'outer: for raw_line in clean.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        for ch in trimmed.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
            if current_width + ch_width > max_width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
                if lines.len() >= max_lines {
                    truncated = true;
                    break 'outer;
                }
            }
            current.push(ch);
            current_width += ch_width;
        }

        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            if lines.len() >= max_lines {
                if clean.lines().count() > lines.len() {
                    truncated = true;
                }
                break;
            }
        }
    }

    if lines.is_empty() {
        lines.push(clean.to_string());
    }

    if truncated && let Some(last) = lines.last_mut() {
        if !last.ends_with('…') {
            last.push('…');
        }
    }

    lines
}

pub(super) fn display_width(text: &str) -> usize {
    text.chars().map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1).max(1)).sum()
}

pub(super) fn accent_color(kind: &str) -> Color {
    workflow_node_accent_color(kind)
}

pub(super) fn node_glyph(kind: &str) -> &'static str {
    match kind {
        "start" => "S",
        "end" => "E",
        "answer" => "A",
        "llm" => "AI",
        "if-else" => "IF",
        "code" => "C",
        "tool" => "T",
        "knowledge-retrieval" => "K",
        "question-classifier" => "Q",
        "http-request" => "H",
        "iteration" => "IT",
        "loop" => "L",
        _ => "•",
    }
}

pub(super) fn line_block_height(line_count: usize, font_size: f32, line_step: f32) -> f32 {
    if line_count == 0 { 0.0 } else { font_size + line_step * line_count.saturating_sub(1) as f32 }
}

pub(super) fn start_variable_badge_text(value_type: &str) -> &'static str {
    match value_type {
        "number" => "#",
        "boolean" => "B",
        "file" | "array[file]" => "F",
        _ => "T",
    }
}

pub(super) fn node_description_text(document: &WorkflowDocument, node: &WorkflowNode) -> String {
    if node.is_group() {
        let child_count = document.group_child_count(&node.id);
        if child_count > 0 {
            format!("包含 {} 个子节点", child_count)
        } else if node.description.trim().is_empty() {
            "容器节点，可拖动带走内部节点".to_string()
        } else {
            node.description.clone()
        }
    } else if node.block_type == "code" {
        code_node_description_text(node)
    } else if node.description.trim().is_empty() {
        String::new()
    } else {
        node.description.clone()
    }
}

fn code_node_description_text(node: &WorkflowNode) -> String {
    match code_node_error_strategy(node) {
        Some("default-value") => "异常时 输出默认值".to_string(),
        Some("fail-branch") => "异常时 异常分支".to_string(),
        _ if node.description.trim().is_empty() => String::new(),
        _ => node.description.clone(),
    }
}

fn code_node_error_strategy(node: &WorkflowNode) -> Option<&str> {
    node.raw_node
        .as_mapping()
        .and_then(|node_map| node_map.get(&Value::String("data".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|data_map| data_map.get(&Value::String("error_strategy".to_string())))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn theme_is_dark(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

pub(super) fn blend(left: Color, right: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color {
        r: left.r + (right.r - left.r) * factor,
        g: left.g + (right.g - left.g) * factor,
        b: left.b + (right.b - left.b) * factor,
        a: left.a + (right.a - left.a) * factor,
    }
}

pub(super) fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha.clamp(0.0, 1.0), ..color }
}

pub(super) fn canvas_element_scale(zoom: f32) -> f32 {
    zoom.max(0.3)
}

pub(super) fn screen_from_world(world: Point, pan: Vector, zoom: f32) -> Point {
    Point::new(world.x * zoom + pan.x, world.y * zoom + pan.y)
}

pub(super) fn world_from_screen(screen: Point, pan: Vector, zoom: f32) -> Point {
    Point::new((screen.x - pan.x) / zoom.max(0.0001), (screen.y - pan.y) / zoom.max(0.0001))
}

pub(super) fn cubic_bezier_point(start: Point, c1: Point, c2: Point, end: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    let x = mt.powi(3) * start.x
        + 3.0 * mt.powi(2) * t * c1.x
        + 3.0 * mt * t.powi(2) * c2.x
        + t.powi(3) * end.x;
    let y = mt.powi(3) * start.y
        + 3.0 * mt.powi(2) * t * c1.y
        + 3.0 * mt * t.powi(2) * c2.y
        + t.powi(3) * end.y;
    Point::new(x, y)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn export_svg(document: &WorkflowDocument) -> String {
    fn escape_xml(text: &str) -> String {
        let mut escaped = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&apos;"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    fn color_to_css(color: Color) -> String {
        let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        let a = color.a.clamp(0.0, 1.0);
        format!("rgba({r},{g},{b},{a:.3})")
    }

    let Some(bounds) = document.bounds() else {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"640\" viewBox=\"0 0 960 640\"><rect width=\"960\" height=\"640\" fill=\"rgba(248,250,253,1)\"/></svg>".to_string();
    };

    let padding = 96.0;
    let view_x = bounds.x - padding;
    let view_y = bounds.y - padding;
    let view_width = bounds.width + padding * 2.0;
    let view_height = bounds.height + padding * 2.0;

    let background = Color::from_rgba8(248, 250, 253, 1.0);
    let node_fill = Color::from_rgba8(255, 255, 255, 0.98);
    let border_base = Color::from_rgba8(148, 163, 184, 0.34);
    let edge_base = Color::from_rgba8(148, 163, 184, 0.88);
    let text_primary = Color::from_rgb8(15, 23, 42);
    let text_secondary = Color::from_rgba8(71, 85, 105, 0.92);

    let handle_slots = build_handle_slots(document);
    let node_map =
        document.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let mut edges = document.edges.iter().collect::<Vec<_>>();
    edges.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));
    let mut nodes = document.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.z_index.total_cmp(&right.z_index));

    let mut svg = String::new();
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\">",
        view_width, view_height, view_x, view_y, view_width, view_height,
    );
    let _ = writeln!(
        svg,
        "<defs><filter id=\"node-shadow\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"160%\"><feDropShadow dx=\"0\" dy=\"8\" stdDeviation=\"10\" flood-color=\"rgba(15,23,42,0.12)\"/></filter></defs>"
    );
    let _ = writeln!(
        svg,
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        view_x,
        view_y,
        view_width,
        view_height,
        color_to_css(background),
    );

    for edge in edges {
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
            &handle_slots,
            Vector::new(0.0, 0.0),
            1.0,
        );
        let end = anchor_for_handle(
            target_node,
            WorkflowHandleKind::Target,
            edge.target_handle.as_deref().unwrap_or("target"),
            &handle_slots,
            Vector::new(0.0, 0.0),
            1.0,
        );
        let distance = ((end.x - start.x).abs() + (end.y - start.y).abs()) * 0.35;
        let control_distance = distance.clamp(28.0, 220.0);
        let c1 = control_for_side(start, source_node.source_side, control_distance);
        let c2 = control_for_side(end, target_node.target_side, control_distance);
        let color = if edge.selected { accent_color(&edge.source_type) } else { edge_base };

        let _ = writeln!(
            svg,
            "<path d=\"M {:.2} {:.2} C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\"/>",
            start.x,
            start.y,
            c1.x,
            c1.y,
            c2.x,
            c2.y,
            end.x,
            end.y,
            color_to_css(color),
            if edge.selected { 2.8 } else { 1.6 },
        );

        if let Some(label) = edge_handle_label(edge) {
            let badge_center = cubic_bezier_point(start, c1, c2, end, 0.5);
            let badge_width = (label.chars().count() as f32 * 8.0 + 18.0).clamp(30.0, 94.0);
            let badge_x = badge_center.x - badge_width / 2.0;
            let badge_y = badge_center.y - 11.0;
            let _ = writeln!(
                svg,
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"22\" rx=\"11\" ry=\"11\" fill=\"rgba(255,255,255,0.96)\" stroke=\"{}\" stroke-width=\"1\"/>",
                badge_x,
                badge_y,
                badge_width,
                color_to_css(with_alpha(color, 0.30)),
            );
            let _ = writeln!(
                svg,
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"12\" fill=\"{}\">{}</text>",
                badge_center.x,
                badge_center.y,
                color_to_css(accent_color(&edge.source_type)),
                escape_xml(&label),
            );
        }
    }

    for node in nodes {
        let rect = node.rect_world();
        let accent = accent_color(&node.block_type);
        let border = if node.selected { accent } else { border_base };
        let title = if node.title.trim().is_empty() {
            pretty_block_type(&node.block_type)
        } else {
            node.title.clone()
        };
        let start_variables = workflow_start_node_variables(node);
        let show_start_variables = node.block_type == "start" && !start_variables.is_empty();
        let title_font_size = if rect.height < 70.0 { 13.0 } else { 15.0 };
        let title_line_step = title_font_size + 3.0;
        let desc_font_size = if rect.height > 140.0 { 12.0 } else { 11.0 };
        let desc_line_step = desc_font_size + 3.0;
        let description = node_description_text(document, node);
        let title_x = rect.x + 14.0 + 24.0 + 12.0;
        let title_width = (rect.x + rect.width - title_x - 14.0).max(rect.width - 96.0);
        let title_max_chars = (title_width / (title_font_size * 0.63)).floor().max(8.0) as usize;
        let title_lines =
            wrap_text_lines(&title, title_max_chars, if rect.height > 120.0 { 2 } else { 1 });
        let desc_max_chars = (title_width / (desc_font_size * 0.62)).floor().max(10.0) as usize;
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
            14.0 + start_variables.len() as f32 * 34.0
                + start_variables.len().saturating_sub(1) as f32 * 8.0
        } else {
            0.0
        };
        let text_block_height = if show_start_variables {
            title_block_height + start_variable_block_height
        } else {
            title_block_height + if desc_lines.is_empty() { 0.0 } else { 10.0 + desc_block_height }
        };
        let content_block_height = if show_start_variables {
            24.0_f32.max(title_block_height) + start_variable_block_height
        } else {
            24.0_f32.max(text_block_height)
        };
        let content_top = if show_start_variables {
            rect.y + 18.0
        } else {
            rect.y + (rect.height - content_block_height).max(0.0) / 2.0
        };

        if node.selected {
            let _ = writeln!(
                svg,
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"24\" ry=\"24\" fill=\"{}\"/>",
                rect.x - 4.0,
                rect.y - 4.0,
                rect.width + 8.0,
                rect.height + 8.0,
                color_to_css(with_alpha(accent, 0.08)),
            );
        }

        let _ = writeln!(svg, "<g filter=\"url(#node-shadow)\">");
        let _ = writeln!(
            svg,
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"20\" ry=\"20\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>",
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            color_to_css(node_fill),
            color_to_css(border),
            if node.selected { 2.0 } else { 1.1 },
        );
        let _ = writeln!(svg, "</g>");

        let badge_x = rect.x + 14.0;
        let badge_y = if show_start_variables {
            content_top
        } else {
            content_top + (content_block_height - 24.0).max(0.0) / 2.0
        };
        let _ = writeln!(
            svg,
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"24\" height=\"24\" rx=\"11\" ry=\"11\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            badge_x,
            badge_y,
            color_to_css(blend(node_fill, accent, 0.16)),
            color_to_css(with_alpha(accent, 0.18)),
        );
        let _ = writeln!(
            svg,
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"11\" font-weight=\"600\" fill=\"{}\">{}</text>",
            badge_x + 12.0,
            badge_y + 12.0,
            color_to_css(accent),
            escape_xml(node_glyph(&node.block_type)),
        );

        let title_y = if show_start_variables {
            badge_y + (24.0 - title_block_height).max(0.0) / 2.0
        } else {
            content_top + (content_block_height - text_block_height).max(0.0) / 2.0
        };
        for (index, line) in title_lines.iter().enumerate() {
            let _ = writeln!(
                svg,
                "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"{:.2}\" font-weight=\"600\" fill=\"{}\">{}</text>",
                title_x,
                title_y + title_font_size + index as f32 * title_line_step,
                title_font_size,
                color_to_css(text_primary),
                escape_xml(line),
            );
        }

        if show_start_variables {
            let row_width = rect.width - 28.0;
            let row_label_max_chars =
                ((row_width - 24.0 - 18.0 - 10.0) / (12.0 * 0.62)).floor().max(6.0) as usize;

            for (index, variable) in start_variables.iter().enumerate() {
                let row_x = rect.x + 14.0;
                let row_y = badge_y + 24.0 + 14.0 + index as f32 * (34.0 + 8.0);
                let badge_box_x = row_x + row_width - 12.0 - 18.0;
                let badge_box_y = row_y + 8.0;
                let row_label = wrap_text_lines(&variable.name, row_label_max_chars, 1)
                    .into_iter()
                    .next()
                    .unwrap_or_default();

                let _ = writeln!(
                    svg,
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"34\" rx=\"12\" ry=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    row_x,
                    row_y,
                    row_width,
                    color_to_css(Color::from_rgba8(241, 245, 249, 0.96)),
                    color_to_css(Color::from_rgba8(203, 213, 225, 0.72)),
                );
                let _ = writeln!(
                    svg,
                    "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"12\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                    row_x + 12.0,
                    row_y + 17.0,
                    color_to_css(text_primary),
                    escape_xml(&row_label),
                );
                let _ = writeln!(
                    svg,
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"18\" height=\"18\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    badge_box_x,
                    badge_box_y,
                    color_to_css(blend(Color::from_rgba8(241, 245, 249, 0.96), accent, 0.12)),
                    color_to_css(with_alpha(accent, 0.20)),
                );
                let _ = writeln!(
                    svg,
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"10\" font-weight=\"600\" fill=\"{}\">{}</text>",
                    badge_box_x + 9.0,
                    badge_box_y + 9.0,
                    color_to_css(accent),
                    escape_xml(start_variable_badge_text(&variable.value_type)),
                );
            }
        } else if !desc_lines.is_empty() {
            let desc_y = title_y + title_block_height + 10.0;

            for (index, line) in desc_lines.iter().enumerate() {
                let _ = writeln!(
                    svg,
                    "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                    title_x,
                    desc_y + desc_font_size + index as f32 * desc_line_step,
                    desc_font_size,
                    color_to_css(text_secondary),
                    escape_xml(line),
                );
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn export_svg(_document: &WorkflowDocument) -> String {
    String::new()
}
