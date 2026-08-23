//! 验证文件扩展名图标映射、固定尺寸及主题色绘制契约。
//! 记录型渲染器捕获 SVG 绘制颜色，避免只验证控件构造而遗漏实际主题着色路径。

use super::icons::{file_icon_for, static_icon_svg, themed_icon_svg};
use crate::app::assets::Icon;
use iced::advanced::{Layout, Widget, layout, mouse, renderer, svg, widget::Tree};
use iced::{Background, Color, Element, Length, Rectangle, Size, Theme, Transformation};

// 真实 SVG 绘制调用会把最终主题色记录下来，空实现的方法不属于本测试观察面。
#[derive(Debug, Default)]
struct RecordingRenderer {
    svg_colors: Vec<Option<Color>>,
}

impl iced::advanced::Renderer for RecordingRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _transformation: Transformation) {}

    fn end_transformation(&mut self) {}

    fn fill_quad(&mut self, _quad: renderer::Quad, _background: impl Into<Background>) {}

    fn reset(&mut self, _new_bounds: Rectangle) {
        self.svg_colors.clear();
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

impl svg::Renderer for RecordingRenderer {
    fn measure_svg(&self, _handle: &svg::Handle) -> Size<u32> {
        Size::new(14, 14)
    }

    fn draw_svg(&mut self, svg: svg::Svg, _bounds: Rectangle, _clip_bounds: Rectangle) {
        self.svg_colors.push(svg.color);
    }
}

fn assert_fixed_icon_size(element: &Element<'_, ()>) {
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(14.0));
    assert_eq!(size.height, Length::Fixed(14.0));
}

#[test]
fn file_icon_for_maps_every_supported_extension() {
    let cases = [
        ("main.rs", Icon::Rust),
        ("app.ts", Icon::Typescript),
        ("component.tsx", Icon::Typescript),
        ("index.js", Icon::Javascript),
        ("view.jsx", Icon::Javascript),
        ("package.json", Icon::Json),
        ("Cargo.toml", Icon::Toml),
        ("pipeline.yaml", Icon::Yaml),
        ("config.yml", Icon::Yaml),
        ("README.md", Icon::Markdown),
        ("index.html", Icon::Html),
        ("legacy.htm", Icon::Html),
        ("style.css", Icon::Css),
        ("script.py", Icon::Python),
        ("server.go", Icon::Go),
        ("install.sh", Icon::Console),
    ];

    for (name, expected) in cases {
        assert_eq!(file_icon_for(name), expected, "unexpected icon for {name}");
    }
}

#[test]
fn file_icon_for_matches_extensions_case_insensitively() {
    assert_eq!(file_icon_for("MAIN.RS"), Icon::Rust);
    assert_eq!(file_icon_for("COMPONENT.TSX"), Icon::Typescript);
    assert_eq!(file_icon_for("README.MD"), Icon::Markdown);
    assert_eq!(file_icon_for("INDEX.HTML"), Icon::Html);
}

#[test]
fn file_icon_for_falls_back_when_supported_suffix_is_not_final() {
    for name in ["", "README", "main.rst", "app.ts.map", "README.md.bak"] {
        assert_eq!(file_icon_for(name), Icon::Document, "unexpected icon for {name}");
    }
}

#[test]
fn static_icon_svg_uses_fixed_square_size() {
    let element: Element<'_, ()> = static_icon_svg(Icon::File).into();

    assert_fixed_icon_size(&element);
}

#[test]
fn themed_icon_svg_uses_fixed_square_size() {
    let element: Element<'_, ()> = themed_icon_svg(Icon::FolderOpen).into();

    assert_fixed_icon_size(&element);
}

#[test]
fn themed_icon_svg_draw_uses_current_theme_text_color() {
    for theme in [Theme::Light, Theme::Dark] {
        let mut icon = themed_icon_svg(Icon::FolderOpen);
        let mut tree = Tree::empty();
        let mut renderer = RecordingRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(14.0, 14.0));
        let node = <iced::widget::svg::Svg<'_> as Widget<(), Theme, RecordingRenderer>>::layout(
            &mut icon, &mut tree, &renderer, &limits,
        );

        <iced::widget::svg::Svg<'_> as Widget<(), Theme, RecordingRenderer>>::draw(
            &icon,
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::INFINITE,
        );

        assert_eq!(renderer.svg_colors, vec![Some(theme.palette().text)]);
    }
}
