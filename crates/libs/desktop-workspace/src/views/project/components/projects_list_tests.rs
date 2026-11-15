use super::*;
use std::fs;
use tempfile::tempdir;
fn assert_color_eq(actual: Color, expected: Color) {
    assert_eq!(actual.r, expected.r);
    assert_eq!(actual.g, expected.g);
    assert_eq!(actual.b, expected.b);
    assert_eq!(actual.a, expected.a);
}

#[test]
fn parse_hex_color_accepts_hash_and_trimmed_plain_values() {
    assert_color_eq(parse_hex_color("#0A1b2C").unwrap(), Color::from_rgb8(10, 27, 44));
    assert_color_eq(parse_hex_color("  ff8040  ").unwrap(), Color::from_rgb8(255, 128, 64));
}


#[test]
fn parse_hex_color_rejects_invalid_length_or_digits() {
    assert!(parse_hex_color("").is_none());
    assert!(parse_hex_color("#12345").is_none());
    assert!(parse_hex_color("#1234567").is_none());
    assert!(parse_hex_color("#12xx56").is_none());
}


#[test]
fn icon_image_handle_rejects_empty_or_missing_paths() {
    assert!(icon_image_handle("").is_none());
    assert!(icon_image_handle("   ").is_none());
    assert!(icon_image_handle("/path/that/does/not/exist.png").is_none());
}


#[test]
fn icon_image_handle_accepts_file_urls_and_plain_paths() {
    let dir = tempdir().unwrap();
    let icon_path = dir.path().join("icon.png");
    fs::write(&icon_path, b"not decoded until render").unwrap();

    assert!(icon_image_handle(icon_path.to_str().unwrap()).is_some());
    assert!(icon_image_handle(&format!("file://{}", icon_path.display())).is_some());
    assert!(icon_image_handle(&format!("file:///{}", icon_path.display())).is_some());
}


#[test]
fn project_badge_button_builds_default_text_icon() {
    let element: Element<'_, Message> = project_badge_button(
        ProjectBadge {
            path: "/tmp/project-a".to_string(),
            title: "Alpha Project".to_string(),
            selected: false,
            has_attention: false,
            is_hovered: false,
            custom_icon: None,
            custom_color: None,
        },
        false,
    );

    std::hint::black_box(element);
}


#[test]
fn project_badge_button_builds_selected_attention_custom_color_icon() {
    let element: Element<'_, Message> = project_badge_button(
        ProjectBadge {
            path: "/tmp/project-b".to_string(),
            title: "项目乙".to_string(),
            selected: true,
            has_attention: true,
            is_hovered: true,
            custom_icon: Some("字".to_string()),
            custom_color: Some(Color::from_rgb8(24, 144, 255)),
        },
        false,
    );

    std::hint::black_box(element);
}


#[test]
fn project_badge_button_builds_image_icon_branch() {
    let dir = tempdir().unwrap();
    let icon_path = dir.path().join("icon.png");
    fs::write(&icon_path, b"not decoded until render").unwrap();

    let element: Element<'_, Message> = project_badge_button(
        ProjectBadge {
            path: "/tmp/project-c".to_string(),
            title: "Gamma".to_string(),
            selected: false,
            has_attention: true,
            is_hovered: false,
            custom_icon: Some(icon_path.to_string_lossy().to_string()),
            custom_color: Some(Color::from_rgb8(180, 40, 80)),
        },
        false,
    );

    std::hint::black_box(element);
}


#[test]
fn open_project_badge_button_builds() {
    let element: Element<'_, Message> = open_project_badge_button(desktop_i18n::I18n::new(
        config_ui::DesktopLocale::SimplifiedChinese,
    ));

    std::hint::black_box(element);
}


#[cfg(not(target_arch = "wasm32"))]
#[test]
fn expert_hub_entry_follows_open_project_action() {
    assert_eq!(
        project_list_action_order(),
        ["open_project", "expert_hub", "shiju_skills", "shiju_apps"]
    );
    let element: Element<'_, Message> = expert_hub_badge_button(desktop_i18n::I18n::new(
        config_ui::DesktopLocale::SimplifiedChinese,
    ));
    std::hint::black_box(element);
    let element: Element<'_, Message> = shiju_skills_badge_button(desktop_i18n::I18n::new(
        config_ui::DesktopLocale::SimplifiedChinese,
    ));
    std::hint::black_box(element);
    let element: Element<'_, Message> = shiju_apps_badge_button(desktop_i18n::I18n::new(
        config_ui::DesktopLocale::SimplifiedChinese,
    ));
    std::hint::black_box(element);
}


#[test]
fn project_badge_button_builds_with_custom_color_only() {
    // custom_color provided without custom_icon → use_custom_bg = true
    let element: Element<'_, Message> = project_badge_button(
        ProjectBadge {
            path: "/tmp/project-d".to_string(),
            title: "Delta".to_string(),
            selected: true,
            has_attention: false,
            is_hovered: true,
            custom_icon: None,
            custom_color: Some(Color::from_rgb8(100, 200, 50)),
        },
        false,
    );

    std::hint::black_box(element);
}


#[test]
fn icon_image_handle_trims_surrounding_whitespace_from_existing_path() {
    let dir = tempdir().unwrap();
    let icon_path = dir.path().join("trimmed-icon.png");
    fs::write(&icon_path, b"deferred image bytes").unwrap();

    assert!(icon_image_handle(&format!("  {}  ", icon_path.display())).is_some());
}


#[test]
fn project_badge_button_ignores_blank_custom_icon() {
    let element: Element<'_, Message> = project_badge_button(
        ProjectBadge {
            path: "/tmp/blank-icon".to_string(),
            title: "Blank Icon".to_string(),
            selected: false,
            has_attention: false,
            is_hovered: false,
            custom_icon: Some("   ".to_string()),
            custom_color: Some(Color::from_rgb8(40, 80, 120)),
        },
        false,
    );

    std::hint::black_box(element);
}
