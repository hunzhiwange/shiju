//! 解析 SVG 路径数据，并构造可缩放或适配边界的 Iced 画布路径。

use iced::{
    Point, Radians, Size,
    widget::canvas::{
        Path,
        path::{Arc, Builder},
    },
};
use std::f32::consts::PI;

/// 按给定原点和缩放构造路径；空几何数据返回 `None`。
pub fn build_svg_path(geometry: &str, origin: Point, scale: f32) -> Option<Path> {
    if geometry.trim().is_empty() {
        return None;
    }

    Some(Path::new(|builder| {
        let mut parser = SvgPathParser::new(geometry);
        build_path_from_parser(builder, &mut parser, origin, scale);
    }))
}

/// 等比缩放路径并居中放入目标矩形；无有效边界时返回 `None`。
pub fn build_svg_path_fit(geometry: &str, box_origin: Point, box_size: Size) -> Option<Path> {
    let (min_x, min_y, max_x, max_y) = svg_path_bounds(geometry)?;
    let geom_w = (max_x - min_x).max(0.0);
    let geom_h = (max_y - min_y).max(0.0);
    if geom_w <= f32::EPSILON || geom_h <= f32::EPSILON {
        return None;
    }

    let scale = (box_size.width / geom_w).min(box_size.height / geom_h);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }

    let scaled_w = geom_w * scale;
    let scaled_h = geom_h * scale;
    let origin_x = box_origin.x + (box_size.width - scaled_w) / 2.0 - min_x * scale;
    let origin_y = box_origin.y + (box_size.height - scaled_h) / 2.0 - min_y * scale;

    build_svg_path(geometry, Point::new(origin_x, origin_y), scale)
}

struct SvgPathParser<'a> {
    chars: std::str::Chars<'a>,
    current_char: Option<char>,
}

impl<'a> SvgPathParser<'a> {
    fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let current_char = chars.next();
        Self { chars, current_char }
    }

    fn advance(&mut self) {
        self.current_char = self.chars.next();
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c.is_whitespace() || c == ',' {
                self.advance();
            } else {
                break;
            }
        }
    }

    #[allow(dead_code)]
    fn next_command(&mut self) -> Option<char> {
        self.skip_whitespace();
        if let Some(c) = self.current_char
            && c.is_ascii_alphabetic()
        {
            self.advance();
            return Some(c);
        }
        None
    }

    fn next_number(&mut self) -> Option<f32> {
        self.skip_whitespace();
        let mut num_str = String::new();

        if let Some(c) = self.current_char
            && (c == '-' || c == '+')
        {
            num_str.push(c);
            self.advance();
        }

        let mut has_dot = false;
        let mut has_digits = false;

        // Check for leading dot
        if let Some(c) = self.current_char
            && c == '.'
        {
            has_dot = true;
            num_str.push(c);
            self.advance();
        }

        while let Some(c) = self.current_char {
            if c.is_ascii_digit() {
                has_digits = true;
                num_str.push(c);
                self.advance();
            } else if c == '.' && !has_dot {
                has_dot = true;
                num_str.push(c);
                self.advance();
            } else if c == '.' && has_dot {
                // Second dot starts a new number, stop here
                break;
            } else if c == 'e' || c == 'E' {
                // Scientific notation support could be added here
                // For now, simplified
                break;
            } else {
                break;
            }
        }

        if !has_digits && !has_dot {
            // If only sign, it's invalid
            return None;
        }

        num_str.parse::<f32>().ok()
    }

    fn next_flag(&mut self) -> Option<f32> {
        self.skip_whitespace();
        if let Some(c) = self.current_char
            && (c == '0' || c == '1')
        {
            self.advance();
            return Some(if c == '1' { 1.0 } else { 0.0 });
        }
        None
    }
}

fn build_path_from_parser(
    builder: &mut Builder,
    parser: &mut SvgPathParser,
    origin: Point,
    scale: f32,
) {
    let mut current = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    let mut last_cmd = 'M';
    let mut first_cmd = true;

    // We loop trying to get a command.
    // If no command, we reuse last_cmd (if implicit).
    // But how to detect implicit?
    // Usually: Read command. Loop reading arguments for that command until failure.

    // Better structure:
    // Loop:
    //   Peek for command letter.
    //   If letter found -> update last_cmd.
    //   If no letter -> use last_cmd (if valid for repetition).
    //   If no letter and no args -> break.

    loop {
        parser.skip_whitespace();
        let cmd = if let Some(c) = parser.current_char {
            if c.is_ascii_alphabetic() {
                parser.advance();
                last_cmd = c;
                c
            } else {
                // Implicit repetition
                if first_cmd {
                    // Cannot have implicit command at start
                    break;
                }
                // If last command was M/m, implicit is L/l
                if last_cmd == 'M' {
                    last_cmd = 'L';
                }
                if last_cmd == 'm' {
                    last_cmd = 'l';
                }
                last_cmd
            }
        } else {
            break;
        };
        first_cmd = false;

        match cmd {
            'M' | 'm' => {
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (nx, ny) = if cmd == 'm' { (current.0 + x, current.1 + y) } else { (x, y) };
                current = (nx, ny);
                start = current;
                builder.move_to(Point::new(origin.x + nx * scale, origin.y + ny * scale));
            }
            'L' | 'l' => {
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (nx, ny) = if cmd == 'l' { (current.0 + x, current.1 + y) } else { (x, y) };
                current = (nx, ny);
                builder.line_to(Point::new(origin.x + nx * scale, origin.y + ny * scale));
            }
            'H' | 'h' => {
                let Some(x) = parser.next_number() else { break };
                let nx = if cmd == 'h' { current.0 + x } else { x };
                current.0 = nx;
                builder.line_to(Point::new(origin.x + nx * scale, origin.y + current.1 * scale));
            }
            'V' | 'v' => {
                let Some(y) = parser.next_number() else { break };
                let ny = if cmd == 'v' { current.1 + y } else { y };
                current.1 = ny;
                builder.line_to(Point::new(origin.x + current.0 * scale, origin.y + ny * scale));
            }
            'C' | 'c' => {
                let Some(x1) = parser.next_number() else {
                    break;
                };
                let Some(y1) = parser.next_number() else {
                    break;
                };
                let Some(x2) = parser.next_number() else {
                    break;
                };
                let Some(y2) = parser.next_number() else {
                    break;
                };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (cx1, cy1, cx2, cy2, nx, ny) = if cmd == 'c' {
                    (
                        current.0 + x1,
                        current.1 + y1,
                        current.0 + x2,
                        current.1 + y2,
                        current.0 + x,
                        current.1 + y,
                    )
                } else {
                    (x1, y1, x2, y2, x, y)
                };

                builder.bezier_curve_to(
                    Point::new(origin.x + cx1 * scale, origin.y + cy1 * scale),
                    Point::new(origin.x + cx2 * scale, origin.y + cy2 * scale),
                    Point::new(origin.x + nx * scale, origin.y + ny * scale),
                );
                current = (nx, ny);
            }
            'Q' | 'q' => {
                let Some(x1) = parser.next_number() else {
                    break;
                };
                let Some(y1) = parser.next_number() else {
                    break;
                };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (cx, cy, nx, ny) = if cmd == 'q' {
                    (current.0 + x1, current.1 + y1, current.0 + x, current.1 + y)
                } else {
                    (x1, y1, x, y)
                };

                builder.quadratic_curve_to(
                    Point::new(origin.x + cx * scale, origin.y + cy * scale),
                    Point::new(origin.x + nx * scale, origin.y + ny * scale),
                );
                current = (nx, ny);
            }
            'Z' | 'z' => {
                builder.close();
                current = start;
            }
            'A' | 'a' => {
                let Some(rx) = parser.next_number() else {
                    break;
                };
                let Some(ry) = parser.next_number() else {
                    break;
                };
                let Some(x_axis_rotation) = parser.next_number() else {
                    break;
                };
                let Some(large_arc_flag) = parser.next_flag() else {
                    break;
                };
                let Some(sweep_flag) = parser.next_flag() else {
                    break;
                };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (nx, ny) = if cmd == 'a' { (current.0 + x, current.1 + y) } else { (x, y) };

                let p1 = current;
                let p2 = (nx, ny);

                if rx == 0.0 || ry == 0.0 {
                    builder.line_to(Point::new(origin.x + nx * scale, origin.y + ny * scale));
                    current = (nx, ny);
                    continue;
                }

                let phi = x_axis_rotation * PI / 180.0;
                let rx = rx.abs();
                let ry = ry.abs();

                let dx = (p1.0 - p2.0) / 2.0;
                let dy = (p1.1 - p2.1) / 2.0;
                let x1p = phi.cos() * dx + phi.sin() * dy;
                let y1p = -phi.sin() * dx + phi.cos() * dy;

                let rx_sq = rx * rx;
                let ry_sq = ry * ry;
                let x1p_sq = x1p * x1p;
                let y1p_sq = y1p * y1p;

                let lambda = x1p_sq / rx_sq + y1p_sq / ry_sq;
                let (rx, ry) = if lambda > 1.0 {
                    let lambda_sqrt = lambda.sqrt();
                    (rx * lambda_sqrt, ry * lambda_sqrt)
                } else {
                    (rx, ry)
                };
                let rx_sq = rx * rx;
                let ry_sq = ry * ry;

                let numerator = (rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq).max(0.0);
                let denominator = rx_sq * y1p_sq + ry_sq * x1p_sq;
                let coef_sq = numerator / denominator;
                let coef = if coef_sq < 0.0 { 0.0 } else { coef_sq.sqrt() };

                let coef = if large_arc_flag == sweep_flag { -coef } else { coef };

                let cxp = coef * (rx * y1p / ry);
                let cyp = coef * -(ry * x1p / rx);

                let cx = phi.cos() * cxp - phi.sin() * cyp + (p1.0 + p2.0) / 2.0;
                let cy = phi.sin() * cxp + phi.cos() * cyp + (p1.1 + p2.1) / 2.0;

                let ux = (x1p - cxp) / rx;
                let uy = (y1p - cyp) / ry;
                let vx = (-x1p - cxp) / rx;
                let vy = (-y1p - cyp) / ry;

                let start_angle = uy.atan2(ux);
                let mut delta_angle = vy.atan2(vx) - start_angle;

                if sweep_flag == 0.0 && delta_angle > 0.0 {
                    delta_angle -= 2.0 * PI;
                } else if sweep_flag != 0.0 && delta_angle < 0.0 {
                    delta_angle += 2.0 * PI;
                }

                if (rx - ry).abs() < 0.1 {
                    builder.arc(Arc {
                        center: Point::new(origin.x + cx * scale, origin.y + cy * scale),
                        radius: rx * scale,
                        start_angle: Radians(start_angle),
                        end_angle: Radians(start_angle + delta_angle),
                    });
                } else {
                    builder.line_to(Point::new(origin.x + nx * scale, origin.y + ny * scale));
                }

                current = (nx, ny);
            }
            _ => {
                // Unknown command, skip or break
                break;
            }
        }
    }
}

fn svg_path_bounds(geometry: &str) -> Option<(f32, f32, f32, f32)> {
    if geometry.trim().is_empty() {
        return None;
    }

    let mut parser = SvgPathParser::new(geometry);
    let mut current = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    let mut last_cmd = 'M';
    let mut first_cmd = true;

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    let mut add_point = |x: f32, y: f32| {
        if x < min_x {
            min_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if x > max_x {
            max_x = x;
        }
        if y > max_y {
            max_y = y;
        }
    };

    loop {
        parser.skip_whitespace();
        let cmd = if let Some(c) = parser.current_char {
            if c.is_ascii_alphabetic() {
                parser.advance();
                last_cmd = c;
                c
            } else {
                if first_cmd {
                    break;
                }
                if last_cmd == 'M' {
                    last_cmd = 'L';
                }
                if last_cmd == 'm' {
                    last_cmd = 'l';
                }
                last_cmd
            }
        } else {
            break;
        };
        first_cmd = false;

        match cmd {
            'M' | 'm' => {
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };
                let (nx, ny) = if cmd == 'm' { (current.0 + x, current.1 + y) } else { (x, y) };
                current = (nx, ny);
                start = current;
                add_point(nx, ny);
            }
            'L' | 'l' => {
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };
                let (nx, ny) = if cmd == 'l' { (current.0 + x, current.1 + y) } else { (x, y) };
                current = (nx, ny);
                add_point(nx, ny);
            }
            'H' | 'h' => {
                let Some(x) = parser.next_number() else { break };
                let nx = if cmd == 'h' { current.0 + x } else { x };
                current.0 = nx;
                add_point(nx, current.1);
            }
            'V' | 'v' => {
                let Some(y) = parser.next_number() else { break };
                let ny = if cmd == 'v' { current.1 + y } else { y };
                current.1 = ny;
                add_point(current.0, ny);
            }
            'C' | 'c' => {
                let Some(x1) = parser.next_number() else { break };
                let Some(y1) = parser.next_number() else { break };
                let Some(x2) = parser.next_number() else { break };
                let Some(y2) = parser.next_number() else { break };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (cx1, cy1, cx2, cy2, nx, ny) = if cmd == 'c' {
                    (
                        current.0 + x1,
                        current.1 + y1,
                        current.0 + x2,
                        current.1 + y2,
                        current.0 + x,
                        current.1 + y,
                    )
                } else {
                    (x1, y1, x2, y2, x, y)
                };
                add_point(cx1, cy1);
                add_point(cx2, cy2);
                add_point(nx, ny);
                current = (nx, ny);
            }
            'Q' | 'q' => {
                let Some(x1) = parser.next_number() else { break };
                let Some(y1) = parser.next_number() else { break };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (cx, cy, nx, ny) = if cmd == 'q' {
                    (current.0 + x1, current.1 + y1, current.0 + x, current.1 + y)
                } else {
                    (x1, y1, x, y)
                };
                add_point(cx, cy);
                add_point(nx, ny);
                current = (nx, ny);
            }
            'A' | 'a' => {
                let Some(rx) = parser.next_number() else { break };
                let Some(ry) = parser.next_number() else { break };
                let Some(x_axis_rotation) = parser.next_number() else { break };
                let Some(large_arc_flag) = parser.next_flag() else { break };
                let Some(sweep_flag) = parser.next_flag() else { break };
                let Some(x) = parser.next_number() else { break };
                let Some(y) = parser.next_number() else { break };

                let (nx, ny) = if cmd == 'a' { (current.0 + x, current.1 + y) } else { (x, y) };
                add_point(nx, ny);

                if rx != 0.0 && ry != 0.0 {
                    let phi = x_axis_rotation * PI / 180.0;
                    let rx = rx.abs();
                    let ry = ry.abs();

                    let p1 = current;
                    let p2 = (nx, ny);
                    let dx = (p1.0 - p2.0) / 2.0;
                    let dy = (p1.1 - p2.1) / 2.0;
                    let x1p = phi.cos() * dx + phi.sin() * dy;
                    let y1p = -phi.sin() * dx + phi.cos() * dy;

                    let rx_sq = rx * rx;
                    let ry_sq = ry * ry;
                    let x1p_sq = x1p * x1p;
                    let y1p_sq = y1p * y1p;

                    let lambda = x1p_sq / rx_sq + y1p_sq / ry_sq;
                    let (rx, ry) = if lambda > 1.0 {
                        let lambda_sqrt = lambda.sqrt();
                        (rx * lambda_sqrt, ry * lambda_sqrt)
                    } else {
                        (rx, ry)
                    };
                    let rx_sq = rx * rx;
                    let ry_sq = ry * ry;

                    let numerator = (rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq).max(0.0);
                    let denominator = rx_sq * y1p_sq + ry_sq * x1p_sq;
                    let coef_sq = if denominator.abs() <= f32::EPSILON {
                        0.0
                    } else {
                        numerator / denominator
                    };
                    let coef = coef_sq.max(0.0).sqrt();
                    let coef = if large_arc_flag == sweep_flag { -coef } else { coef };

                    let cxp = coef * (rx * y1p / ry);
                    let cyp = coef * -(ry * x1p / rx);

                    let cx = phi.cos() * cxp - phi.sin() * cyp + (p1.0 + p2.0) / 2.0;
                    let cy = phi.sin() * cxp + phi.cos() * cyp + (p1.1 + p2.1) / 2.0;

                    let ux = (x1p - cxp) / rx;
                    let uy = (y1p - cyp) / ry;
                    let vx = (-x1p - cxp) / rx;
                    let vy = (-y1p - cyp) / ry;

                    let start_angle = uy.atan2(ux);
                    let mut delta_angle = vy.atan2(vx) - start_angle;
                    if sweep_flag == 0.0 && delta_angle > 0.0 {
                        delta_angle -= 2.0 * PI;
                    } else if sweep_flag != 0.0 && delta_angle < 0.0 {
                        delta_angle += 2.0 * PI;
                    }

                    if (rx - ry).abs() < 0.1 {
                        let r = rx;
                        let steps = 16usize;
                        for i in 0..=steps {
                            let t = i as f32 / steps as f32;
                            let a = start_angle + delta_angle * t;
                            add_point(cx + r * a.cos(), cy + r * a.sin());
                        }
                    } else {
                        add_point(cx - rx, cy - ry);
                        add_point(cx + rx, cy + ry);
                    }
                }

                current = (nx, ny);
            }
            'Z' | 'z' => {
                current = start;
                add_point(current.0, current.1);
            }
            _ => break,
        }
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "svg_tests.rs"]
mod svg_tests;

#[cfg(test)]
#[path = "svg_path_tests.rs"]
mod svg_path_tests;
