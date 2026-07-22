use super::{SvgPathParser, build_svg_path, build_svg_path_fit, svg_path_bounds};
use iced::{Point, Size};

#[test]
fn number_parser_accepts_separators_signs_decimals_and_adjacent_values() {
    let mut parser = SvgPathParser::new(" \t-12.5,+.75 4.2.3");

    assert_eq!(parser.next_number(), Some(-12.5));
    assert_eq!(parser.next_number(), Some(0.75));
    assert_eq!(parser.next_number(), Some(4.2));
    assert_eq!(parser.next_number(), Some(0.3));
    assert_eq!(parser.next_number(), None);
}

#[test]
fn parser_rejects_invalid_numbers_and_only_accepts_binary_flags() {
    let mut sign_only = SvgPathParser::new("+");
    let mut exponent = SvgPathParser::new("1e3");
    let mut flags = SvgPathParser::new("1,0 2");

    assert_eq!(sign_only.next_number(), None);
    assert_eq!(exponent.next_number(), Some(1.0));
    assert_eq!(exponent.next_number(), None);
    assert_eq!(flags.next_flag(), Some(1.0));
    assert_eq!(flags.next_flag(), Some(0.0));
    assert_eq!(flags.next_flag(), None);
}

#[test]
fn command_parser_skips_separators_and_leaves_non_commands_untouched() {
    let mut parser = SvgPathParser::new(" ,\nM 10");

    assert_eq!(parser.next_command(), Some('M'));
    assert_eq!(parser.next_command(), None);
    assert_eq!(parser.next_number(), Some(10.0));
}

#[test]
fn bounds_cover_absolute_relative_implicit_and_close_commands() {
    let bounds = svg_path_bounds("M 10 20 30 40 h -5 v 15 l -10 -20 z")
        .expect("valid geometry should have bounds");

    assert_eq!(bounds, (10.0, 20.0, 30.0, 55.0));
}

#[test]
fn bounds_include_curve_controls_and_elliptical_arc_extent() {
    let curve = svg_path_bounds("M0 0 C 10 -5 20 15 30 10 Q 40 20 50 0")
        .expect("curves should have bounds");
    let arc = svg_path_bounds("M0 0 A 20 10 0 0 1 40 0").expect("arc should have bounds");

    assert_eq!(curve, (0.0, -5.0, 50.0, 20.0));
    assert!(arc.0 <= 0.0);
    assert!(arc.2 >= 40.0);
    assert!(arc.3 - arc.1 >= 19.0);
}

#[test]
fn bounds_include_relative_curve_controls_and_relative_arc_extent() {
    let bounds = svg_path_bounds("M 10 10 c 5 -5 10 5 15 0 q 5 10 10 0 a 5 5 0 1 0 10 0")
        .expect("relative curves and arcs should have bounds");

    assert_eq!(bounds.0, 10.0);
    assert!(bounds.1 <= 5.0);
    assert!(bounds.2 >= 45.0);
    assert!(bounds.3 >= 20.0);
}

#[test]
fn bounds_return_none_when_path_never_adds_a_point() {
    assert!(svg_path_bounds("X 10 20").is_none());
    assert!(svg_path_bounds("M").is_none());
}

#[test]
fn path_building_rejects_empty_input_and_accepts_supported_commands() {
    assert!(build_svg_path("  \n", Point::ORIGIN, 1.0).is_none());
    assert!(
        build_svg_path(
            "M0 0 L10 10 H20 V5 C20 5 25 10 30 0 Q35 -5 40 0 A5 5 0 0 1 50 0 Z",
            Point::new(3.0, 4.0),
            2.0,
        )
        .is_some()
    );
}

#[test]
fn path_building_accepts_relative_commands_and_zero_radius_arc_fallback() {
    assert!(
        build_svg_path(
            "m 1 2 l 3 4 h 5 v -2 c 1 1 2 2 3 3 q 1 -1 2 -2 a 0 5 0 0 1 10 10 z",
            Point::new(2.0, 3.0),
            1.5,
        )
        .is_some()
    );
}

#[test]
fn fit_rejects_missing_degenerate_and_invalid_target_geometry() {
    assert!(build_svg_path_fit("", Point::ORIGIN, Size::new(20.0, 20.0)).is_none());
    assert!(build_svg_path_fit("M0 0 L0 10", Point::ORIGIN, Size::new(20.0, 20.0)).is_none());
    assert!(build_svg_path_fit("M0 0 L10 10", Point::ORIGIN, Size::new(0.0, 20.0)).is_none());
    assert!(
        build_svg_path_fit("M0 0 L10 10", Point::ORIGIN, Size::new(f32::INFINITY, f32::INFINITY),)
            .is_none()
    );
}

#[test]
fn fit_builds_non_degenerate_geometry_inside_positive_box() {
    assert!(
        build_svg_path_fit(
            "M -10 -5 L 30 -5 L 30 15 L -10 15 Z",
            Point::new(100.0, 50.0),
            Size::new(80.0, 80.0),
        )
        .is_some()
    );
}
