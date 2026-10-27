use super::install_cli_modal::{
    compact_url, format_bytes, format_rate, hash_copy_text, logo_badge_style,
    update_progress_block, update_progress_card_style, view, view_update_check,
    view_update_check_with_progress,
};
use crate::app::Message;
use iced::{Background, Color, Theme};

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn child_count(element: iced::Element<'_, Message>) -> usize {
    element.as_widget().children().len()
}

fn progress(
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    bytes_per_second: f64,
    average_bytes_per_second: f64,
) -> vw_shared::update::UpdateProgress {
    vw_shared::update::UpdateProgress {
        asset_name: "VibeWindow.dmg".to_string(),
        download_url: "https://example.com/releases/vibewindow.dmg".to_string(),
        downloaded_bytes,
        total_bytes,
        bytes_per_second,
        average_bytes_per_second,
    }
}

fn style_background_color(style: iced::widget::container::Style) -> Color {
    match style.background.expect("style should define background") {
        Background::Color(color) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

fn assert_color_close(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red channel {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green channel {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue channel {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha channel {actual:?} != {expected:?}");
}

#[test]
fn task_743_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("install_cli_modal_tests.rs"));
}

#[test]
fn view_builds_plain_install_modal() {
    assert!(child_count(view("安装 CLI", "请先安装命令行工具。")) > 0);
}

#[test]
fn view_builds_completion_modal_with_large_logo_layout() {
    assert!(child_count(view("CLI 安装完成", "现在可以继续使用。")) > 0);
}

#[test]
fn update_check_builds_cli_detection_without_install_action() {
    assert!(
        child_count(view_update_check(
            "检测 CLI 更新",
            "正在检查版本。",
            "1.0.0",
            "1.1.0",
            false,
            false,
            false,
        )) > 0
    );
}

#[test]
fn update_check_builds_cli_install_action_and_disabled_checking_state() {
    keep(view_update_check("安装 CLI", "发现可安装版本。", "0.0.0", "1.1.0", true, true, false));
}

#[test]
fn update_check_builds_app_update_action() {
    keep(view_update_check("更新应用", "发现应用新版本。", "2.0.0", "2.1.0", false, true, true));
}

#[test]
fn update_check_with_progress_builds_known_total_progress() {
    let progress = progress(512 * 1024, Some(1024 * 1024), 1280.0, 2.5 * 1024.0 * 1024.0);

    keep(view_update_check_with_progress(
        "更新应用",
        "正在下载。",
        "2.0.0",
        "2.1.0",
        false,
        true,
        true,
        Some(&progress),
        None,
    ));
}

#[test]
fn update_check_with_progress_builds_copied_unknown_total_progress() {
    let progress = progress(3 * 1024 * 1024, None, 0.0, 0.0);
    let copied_hash = hash_copy_text(&progress.download_url);

    keep(view_update_check_with_progress(
        "更新应用",
        "正在下载。",
        "2.0.0",
        "2.1.0",
        true,
        true,
        true,
        Some(&progress),
        Some(copied_hash),
    ));
}

#[test]
fn update_check_with_progress_ignores_progress_for_cli_install_action() {
    let progress = progress(512 * 1024, Some(1024 * 1024), 2048.0, 4096.0);

    assert!(
        child_count(view_update_check_with_progress(
            "安装 CLI",
            "正在安装。",
            "0.0.0",
            "1.1.0",
            false,
            true,
            false,
            Some(&progress),
            Some(hash_copy_text(&progress.download_url)),
        )) > 0
    );
}

#[test]
fn update_progress_block_handles_zero_and_overflow_totals() {
    let zero_total = progress(4096, Some(0), 512.0, 1024.0);
    assert_eq!(child_count(update_progress_block(&zero_total, None)), 5);

    let overflow_total = progress(3 * 1024 * 1024, Some(1024 * 1024), 1024.0 * 1024.0, 512.0);
    assert_eq!(
        child_count(update_progress_block(
            &overflow_total,
            Some(hash_copy_text(&overflow_total.download_url)),
        )),
        5
    );
}

#[test]
fn format_bytes_uses_kb_below_one_mb_and_mb_at_threshold() {
    assert_eq!(format_bytes(0), "0.0 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
    assert_eq!(format_bytes(5 * 1024 * 1024 + 512 * 1024), "5.5 MB");
}

#[test]
fn format_rate_handles_zero_negative_kb_and_mb_rates() {
    assert_eq!(format_rate(0.0), "0 KB/s");
    assert_eq!(format_rate(-1.0), "0 KB/s");
    assert_eq!(format_rate(1536.0), "1.5 KB/s");
    assert_eq!(format_rate(2.5 * 1024.0 * 1024.0), "2.5 MB/s");
}

#[test]
fn format_rate_switches_to_mb_at_exact_threshold() {
    assert_eq!(format_rate(1024.0 * 1024.0 - 1.0), "1024.0 KB/s");
    assert_eq!(format_rate(1024.0 * 1024.0), "1.0 MB/s");
}

#[test]
fn compact_url_preserves_short_urls_and_compacts_long_urls() {
    let short = "https://example.com/file.dmg";
    assert_eq!(compact_url(short), short);

    let long = "https://example.com/releases/download/v1.2.3/vibewindow-aarch64-apple-darwin.dmg";
    let compact = compact_url(long);

    assert_eq!(compact.chars().count(), 69);
    assert!(compact.starts_with("https://example.com/releases/download/v1.2"));
    assert!(compact.ends_with("aarch64-apple-darwin.dmg"));
}

#[test]
fn compact_url_counts_unicode_chars_not_bytes() {
    let long = "https://example.com/发布/版本/一二三四五六七八九十/abcdefghijklmnopqrstuvwxyz0123456789.zip";
    let compact = compact_url(long);

    assert!(compact.contains("..."));
    assert!(compact.starts_with("https://example.com/发布/版本/一二三四五六七八九十/abcde"));
    assert!(compact.ends_with("qrstuvwxyz0123456789.zip"));
}

#[test]
fn hash_copy_text_is_deterministic_and_content_sensitive() {
    assert_eq!(hash_copy_text("same"), hash_copy_text("same"));
    assert_ne!(hash_copy_text("same"), hash_copy_text("different"));
}

#[test]
fn logo_badge_style_adjusts_alpha_for_light_and_dark_themes() {
    let light = logo_badge_style(&Theme::Light, 20.0);
    assert_color_close(
        style_background_color(light),
        Theme::Light.palette().primary.scale_alpha(0.08),
    );
    assert_eq!(light.border.width, 1.0);
    assert_eq!(light.border.radius.top_left, 20.0);

    let dark = logo_badge_style(&Theme::Dark, 26.0);
    assert_color_close(
        style_background_color(dark),
        Theme::Dark.palette().primary.scale_alpha(0.20),
    );
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.radius.top_left, 26.0);
}

#[test]
fn update_progress_card_style_uses_theme_background_and_border() {
    let light = update_progress_card_style(&Theme::Light);
    assert_color_close(
        style_background_color(light),
        Theme::Light.extended_palette().background.weak.color.scale_alpha(0.55),
    );
    assert_eq!(light.border.width, 1.0);
    assert_eq!(light.border.radius.top_left, 8.0);

    let dark = update_progress_card_style(&Theme::Dark);
    assert_color_close(
        style_background_color(dark),
        Theme::Dark.extended_palette().background.weak.color.scale_alpha(0.55),
    );
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.radius.top_left, 8.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::{Font, Pixels};
    let mut app = crate::app::App::new().0;
    app.cli_install_modal_title = "安装包已就绪".into();
    app.cli_install_modal_message = "安装包路径：/tmp/下载目录/安装包.dmg；请完成系统安装。".into();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        let mut element = super::install_cli_modal::view_app_update(&app);
        let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 900.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &iced::advanced::renderer::Style::default(),
            iced::advanced::Layout::new(&node),
            iced::mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(iced::Size::new(960, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }
}
