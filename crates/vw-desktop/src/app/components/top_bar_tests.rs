//! 验证顶部栏各分区组合、页面状态和窗口级交互。
//!
//! 组合测试在不同页面和窗口状态下构造顶部栏，确认各分区按条件出现。

use crate::app::App;
use crate::app::Screen;
use crate::app::state::WorkspaceMode;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree::Tag;
use iced::{Background, Color, Length, Size, Theme};

fn test_app() -> App {
    App::new().0
}

fn ordered_tree_shape(trees: &[Tree]) -> Vec<(Tag, usize)> {
    fn collect(tree: &Tree, shape: &mut Vec<(Tag, usize)>) {
        shape.push((tree.tag, tree.children.len()));
        for child in &tree.children {
            collect(child, shape);
        }
    }

    let mut shape = Vec::new();
    for tree in trees {
        collect(tree, &mut shape);
    }
    shape
}

#[test]
fn top_bar_height_remains_28_pixels_for_project_layout() {
    assert_eq!(super::top_bar::TOP_BAR_HEIGHT, 28.0);
}

#[test]
fn view_builds_single_row_with_all_toolbar_sections() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;
    let element = super::top_bar::view(&app);

    assert_eq!(
        element.as_widget().size(),
        Size::new(Length::Fill, Length::Fixed(super::top_bar::TOP_BAR_HEIGHT))
    );

    let children = element.as_widget().children();
    assert_eq!(children.len(), 11);
}

#[test]
fn work_top_bar_keeps_all_global_menus_and_project_tools() {
    let mut code_app = test_app();
    code_app.screen = Screen::Project;
    code_app.workspace_mode = WorkspaceMode::Code;
    let code_element = super::top_bar::view(&code_app);
    let code_children = code_element.as_widget().children();
    let code_structure = ordered_tree_shape(&code_children);

    let mut work_app = test_app();
    work_app.screen = Screen::Project;
    work_app.workspace_mode = WorkspaceMode::Work;
    let work_element = super::top_bar::view(&work_app);
    let work_children = work_element.as_widget().children();
    let work_structure = ordered_tree_shape(&work_children);

    assert_eq!(code_children.len(), 11);
    assert_eq!(work_children.len(), 11);
    assert_eq!(work_structure, code_structure);
}

#[test]
fn workspace_mode_selector_keeps_code_controls() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;

    let element = super::top_bar::view(&app);

    assert_eq!(element.as_widget().children().len(), 11);
}

#[test]
fn top_bar_style_uses_theme_palette_with_fixed_border_and_no_shadow() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = super::top_bar::top_bar_style(&theme);

        assert_eq!(style.text_color, None);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.background.strong.color.scale_alpha(0.60));
        assert_eq!(style.border.radius.top_left, 0.0);
        assert_eq!(style.border.radius.top_right, 0.0);
        assert_eq!(style.border.radius.bottom_right, 0.0);
        assert_eq!(style.border.radius.bottom_left, 0.0);
        assert_eq!(style.shadow, Default::default());

        let Some(Background::Color(background)) = style.background else {
            panic!("expected top bar style to set a color background");
        };

        assert_eq!(background, Color { a: 0.88, ..palette.background.weak.color });
    }
}
