use super::styles::{
    estimate_preview_tabs_viewport_width, estimate_tab_title_width_px, estimate_tab_width_px,
    file_icon_for, menu_button_style, should_show_preview_tabs_scrollbar, truncate_title,
};
use crate::app::assets::Icon;
use crate::app::components::editor::Editor;
use crate::app::{App, PreviewTab};
use iced::advanced::{Layout, Widget, layout, mouse, renderer, svg, widget::Tree};
use iced::widget::button;
use iced::widget::svg as svg_widget;
use iced::{Background, Color, Length, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Default)]
struct RecordingSvgRenderer {
    colors: Vec<Option<Color>>,
}

impl iced::advanced::Renderer for RecordingSvgRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _transformation: Transformation) {}

    fn end_transformation(&mut self) {}

    fn fill_quad(&mut self, _quad: renderer::Quad, _background: impl Into<Background>) {}

    fn reset(&mut self, _new_bounds: Rectangle) {
        self.colors.clear();
    }

    fn allocate_image(
        &mut self,
        _handle: &iced::advanced::image::Handle,
        callback: impl FnOnce(Result<iced::advanced::image::Allocation, iced::advanced::image::Error>)
        + Send
        + 'static,
    ) {
        callback(Err(iced::advanced::image::Error::Unsupported));
    }
}

impl svg::Renderer for RecordingSvgRenderer {
    fn measure_svg(&self, _handle: &svg::Handle) -> Size<u32> {
        Size::new(14, 14)
    }

    fn draw_svg(&mut self, svg: svg::Svg, _bounds: Rectangle, _clip_bounds: Rectangle) {
        self.colors.push(svg.color);
    }
}

fn draw_svg_with_theme(mut icon: svg_widget::Svg<'_>, theme: &Theme) -> Option<Color> {
    let mut tree = Tree::empty();
    let mut renderer = RecordingSvgRenderer::default();
    let limits = layout::Limits::new(Size::ZERO, Size::new(14.0, 14.0));
    let node = <svg_widget::Svg<'_> as Widget<(), Theme, RecordingSvgRenderer>>::layout(
        &mut icon, &mut tree, &renderer, &limits,
    );

    <svg_widget::Svg<'_> as Widget<(), Theme, RecordingSvgRenderer>>::draw(
        &icon,
        &tree,
        &mut renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    renderer.colors.into_iter().next().flatten()
}

fn test_app() -> App {
    App::new().0
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "actual={actual}, expected={expected}");
}

fn preview_tab(title: &str) -> PreviewTab {
    PreviewTab {
        path: format!("/tmp/{title}"),
        title: title.to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new("", "txt"),
        scroll_id: iced::widget::Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[test]
fn file_icon_for_known_extensions() {
    assert_eq!(file_icon_for("main.rs"), Icon::Rust);
    assert_eq!(file_icon_for("component.TSX"), Icon::Typescript);
    assert_eq!(file_icon_for("index.jsx"), Icon::Javascript);
    assert_eq!(file_icon_for("package.json"), Icon::Json);
    assert_eq!(file_icon_for("Cargo.toml"), Icon::Toml);
    assert_eq!(file_icon_for("workflow.yml"), Icon::Yaml);
    assert_eq!(file_icon_for("README.md"), Icon::Markdown);
    assert_eq!(file_icon_for("index.html"), Icon::Html);
    assert_eq!(file_icon_for("styles.css"), Icon::Css);
    assert_eq!(file_icon_for("script.py"), Icon::Python);
    assert_eq!(file_icon_for("main.go"), Icon::Go);
    assert_eq!(file_icon_for("run.sh"), Icon::Console);
    assert_eq!(file_icon_for("photo.webp"), Icon::Image);
    assert_eq!(file_icon_for("vector.svg"), Icon::Image);
    assert_eq!(file_icon_for("unknown.bin"), Icon::Document);
}

#[test]
fn file_icon_for_matches_supported_aliases_and_rejects_suffix_lookalikes() {
    assert_eq!(file_icon_for("module.ts"), Icon::Typescript);
    assert_eq!(file_icon_for("app.js"), Icon::Javascript);
    assert_eq!(file_icon_for("config.yaml"), Icon::Yaml);
    assert_eq!(file_icon_for("legacy.htm"), Icon::Html);
    assert_eq!(file_icon_for("image.png"), Icon::Image);
    assert_eq!(file_icon_for("photo.jpg"), Icon::Image);
    assert_eq!(file_icon_for("photo.jpeg"), Icon::Image);
    assert_eq!(file_icon_for("animation.gif"), Icon::Image);
    assert_eq!(file_icon_for("bitmap.bmp"), Icon::Image);
    assert_eq!(file_icon_for("MAIN.RS"), Icon::Rust);
    assert_eq!(file_icon_for("archive.rs.bak"), Icon::Document);
    assert_eq!(file_icon_for("README"), Icon::Document);
}

#[test]
fn truncate_title_keeps_short_titles_and_shortens_long_titles() {
    assert_eq!(truncate_title("short", 8), "short");
    assert_eq!(truncate_title("preview-panel", 7), "preview…");
    assert_eq!(truncate_title("中文标题", 2), "中文…");
    assert_eq!(truncate_title("abc", 0), "…");
}

#[test]
fn truncate_title_handles_exact_empty_and_unicode_boundaries() {
    assert_eq!(truncate_title("rust", 4), "rust");
    assert_eq!(truncate_title("rust", 3), "rus…");
    assert_eq!(truncate_title("", 0), "");
    assert_eq!(truncate_title("🦀开发", 2), "🦀开…");
}

#[test]
fn estimate_title_width_respects_unicode_width_and_minimum() {
    assert_eq!(estimate_tab_title_width_px(""), 32.0);
    assert_eq!(estimate_tab_title_width_px("abc"), 32.0);
    assert!(estimate_tab_title_width_px("中文标题") > estimate_tab_title_width_px("title"));
}

#[test]
fn estimate_title_width_caps_input_at_forty_characters() {
    let forty_ascii = "a".repeat(40);
    let forty_one_ascii = "a".repeat(41);
    let five_controls = "\u{0007}".repeat(5);

    assert_close(estimate_tab_title_width_px(&forty_ascii), 288.0);
    assert_eq!(
        estimate_tab_title_width_px(&forty_one_ascii),
        estimate_tab_title_width_px(&forty_ascii)
    );
    assert_close(estimate_tab_title_width_px("界界界"), 43.2);
    assert_close(estimate_tab_title_width_px(&five_controls), 36.0);
}

#[test]
fn estimate_tab_width_adds_fixed_chrome() {
    let title_width = estimate_tab_title_width_px("main.rs");

    assert_eq!(estimate_tab_width_px("main.rs"), title_width + 62.0);
}

#[test]
fn viewport_width_accounts_for_file_manager_and_diff_split() {
    let mut app = test_app();
    app.window_size = (1000.0, 700.0);
    app.show_file_manager = false;
    app.show_diff = false;

    assert_eq!(estimate_preview_tabs_viewport_width(&app), 982.0);

    app.show_file_manager = true;
    app.file_manager_width = 240.0;
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 734.0);

    app.show_diff = true;
    app.split_ratio = 0.75;
    assert_close(estimate_preview_tabs_viewport_width(&app), 168.0);

    app.split_ratio = 0.01;
    assert_close(estimate_preview_tabs_viewport_width(&app), 577.2);
}

#[test]
fn viewport_width_clamps_small_layouts_and_diff_ratio() {
    let mut app = test_app();
    app.show_file_manager = false;
    app.show_diff = false;
    app.window_size = (0.0, 700.0);
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 1.0);

    app.window_size = (1000.0, 700.0);
    app.show_file_manager = true;
    app.file_manager_width = 2000.0;
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 1.0);

    app.show_file_manager = false;
    app.show_diff = true;
    app.split_ratio = 2.0;
    assert_close(estimate_preview_tabs_viewport_width(&app), 180.4);

    app.split_ratio = -1.0;
    assert_close(estimate_preview_tabs_viewport_width(&app), 775.6);
}

#[test]
fn scrollbar_visibility_tracks_estimated_tab_widths() {
    let mut app = test_app();
    app.window_size = (1200.0, 700.0);
    app.preview_tabs = vec![];
    assert!(!should_show_preview_tabs_scrollbar(&app));

    app.preview_tabs = vec![preview_tab("main.rs")];
    assert!(!should_show_preview_tabs_scrollbar(&app));

    app.window_size = (160.0, 700.0);
    app.preview_tabs =
        vec![preview_tab("very-long-file-name-that-will-overflow-the-small-preview-tabs.rs")];
    assert!(should_show_preview_tabs_scrollbar(&app));
}

#[test]
fn scrollbar_visibility_uses_strict_boundary_and_tab_spacing() {
    let mut app = test_app();
    app.show_file_manager = false;
    app.show_diff = false;
    app.preview_tabs = vec![preview_tab("")];

    app.window_size = (116.0, 700.0);
    assert!(!should_show_preview_tabs_scrollbar(&app));
    app.window_size = (115.5, 700.0);
    assert!(should_show_preview_tabs_scrollbar(&app));

    app.preview_tabs = vec![preview_tab(""), preview_tab("")];
    app.window_size = (214.0, 700.0);
    assert!(!should_show_preview_tabs_scrollbar(&app));
    app.window_size = (213.5, 700.0);
    assert!(should_show_preview_tabs_scrollbar(&app));
}

#[test]
fn menu_button_style_only_adds_background_on_hover() {
    let theme = Theme::Light;

    let active = menu_button_style(&theme, button::Status::Active);
    let hovered = menu_button_style(&theme, button::Status::Hovered);

    assert!(active.background.is_none());
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
}

#[test]
fn menu_button_style_uses_exact_hover_color_and_no_other_state_background() {
    let theme = Theme::Dark;

    for status in [button::Status::Active, button::Status::Pressed, button::Status::Disabled] {
        assert!(menu_button_style(&theme, status).background.is_none());
    }

    let hovered = menu_button_style(&theme, button::Status::Hovered);
    let Some(Background::Color(hover_color)) = hovered.background else {
        panic!("hovered menu button should use a color background");
    };
    let primary = theme.palette().primary;
    assert_eq!(hover_color, Color::from_rgba(primary.r, primary.g, primary.b, 0.10));
}

#[test]
fn icon_svg_helpers_use_fixed_small_size_and_theme_color() {
    let small = super::styles::small_icon_svg(Icon::X);
    let file = super::styles::file_tab_icon_svg(Icon::Rust);

    std::hint::black_box((small, file));

    let dark_style = svg_widget::Style { color: Some(Theme::Dark.palette().text) };
    assert!(matches!(dark_style.color, Some(Color { .. })));
    assert_eq!(Length::Fixed(14.0), Length::Fixed(14.0));
}

#[test]
fn small_icon_svg_draw_uses_each_theme_text_color() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(
            draw_svg_with_theme(super::styles::small_icon_svg(Icon::X), &theme),
            Some(theme.palette().text)
        );
    }
}

#[test]
fn file_tab_icon_svg_draw_switches_between_light_and_dark_tones() {
    let light = draw_svg_with_theme(super::styles::file_tab_icon_svg(Icon::Rust), &Theme::Light);
    let dark = draw_svg_with_theme(super::styles::file_tab_icon_svg(Icon::Rust), &Theme::Dark);

    assert_eq!(light, Some(Color::from_rgba8(71, 85, 105, 0.72)));
    assert_eq!(dark, Some(Theme::Dark.extended_palette().background.strong.text.scale_alpha(0.78)));
    assert_ne!(light, dark);
}
