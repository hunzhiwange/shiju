use super::*;
use crate::app::components::color_picker::ColorFormat;
use crate::app::state::ProjectEditTab;
use iced::advanced::widget::Tree;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn root_element() -> Element<'static, Message> {
    text("root").into()
}

fn element_tree(element: &Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn assert_project_modal_layers(tree: &Tree) {
    assert_eq!(tree.children.len(), 2, "root and project modal should be stacked");
    assert_eq!(
        tree.children[1].children.len(),
        2,
        "project modal should contain a dismiss overlay and centered card",
    );
}

#[test]
fn with_project_edit_returns_root_when_no_project_is_being_edited() {
    let app = test_app();

    let element = with_project_edit(&app, root_element());
    let tree = element_tree(&element);

    assert!(tree.children.is_empty(), "the root text should not gain modal layers");
}

#[test]
fn with_project_edit_ignores_color_picker_when_edit_path_is_absent() {
    let mut app = test_app();
    app.project_edit_icon_color_picker_open = true;

    let element = with_project_edit(&app, root_element());
    let tree = element_tree(&element);

    assert!(tree.children.is_empty(), "a stale picker flag must not open an edit modal");
}

#[test]
fn with_project_edit_builds_all_tab_variants() {
    for tab in [
        ProjectEditTab::General,
        ProjectEditTab::Launch,
        ProjectEditTab::Refresh,
        ProjectEditTab::Scheduling,
    ] {
        let mut app = test_app();
        app.project_edit_path = Some("/repo/app".to_string());
        app.project_edit_tab = tab;
        app.project_edit_name = "Project".to_string();
        app.project_edit_icon_color = "#60a5fa".to_string();

        let element = with_project_edit(&app, root_element());
        let tree = element_tree(&element);

        assert_project_modal_layers(&tree);
    }
}

#[test]
fn with_project_edit_stacks_color_picker_over_project_modal_for_each_color_path_and_format() {
    for (color, format) in [
        ("#34d399", ColorFormat::Hex),
        ("34d399", ColorFormat::Rgba),
        ("not-a-color", ColorFormat::Hsl),
        ("#34d39980", ColorFormat::Css),
    ] {
        let mut app = test_app();
        app.project_edit_path = Some("/repo/app".to_string());
        app.project_edit_tab = ProjectEditTab::General;
        app.project_edit_icon_color_picker_open = true;
        app.project_edit_icon_color = color.to_string();
        app.project_edit_icon_color_format = format;

        let element = with_project_edit(&app, root_element());
        let tree = element_tree(&element);

        assert_eq!(tree.children.len(), 2, "project modal and color picker should be stacked");
        assert_project_modal_layers(&tree.children[0]);
        assert_eq!(
            tree.children[1].children.len(),
            2,
            "color picker should contain a dismiss overlay and centered card",
        );
    }
}
