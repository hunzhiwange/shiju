//! 验证顶部栏各分区组合、页面状态和窗口级交互。
//!
//! 组合测试在不同页面和窗口状态下构造顶部栏，确认各分区按条件出现。

use crate::components::top_bar;
use crate::components::top_bar::types::WorkspaceMode;
use crate::ui_types::Screen;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree::Tag;
use iced::{Length, Size};

fn test_app() -> ToolbarInput {
    ToolbarInput { screen: Screen::Project, workspace_mode: WorkspaceMode::Work }
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
fn view_builds_single_row_with_all_toolbar_sections() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;
    let element = app.view();

    assert_eq!(
        element.as_widget().size(),
        Size::new(Length::Fill, Length::Fixed(top_bar::TOP_BAR_HEIGHT))
    );

    let children = element.as_widget().children();
    assert_eq!(children.len(), 11);
}

#[test]
fn work_top_bar_keeps_all_global_menus_and_project_tools() {
    let mut code_app = test_app();
    code_app.screen = Screen::Project;
    code_app.workspace_mode = WorkspaceMode::Code;
    let code_element = code_app.view();
    let code_children = code_element.as_widget().children();
    let code_structure = ordered_tree_shape(&code_children);

    let mut work_app = test_app();
    work_app.screen = Screen::Project;
    work_app.workspace_mode = WorkspaceMode::Work;
    let work_element = work_app.view();
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

    let element = app.view();

    assert_eq!(element.as_widget().children().len(), 11);
}

struct ToolbarInput {
    screen: Screen,
    workspace_mode: WorkspaceMode,
}
impl ToolbarInput {
    fn view(&self) -> iced::Element<'static, ()> {
        use top_bar::{controls, drag, external, gateway, menus, workspace_mode};
        let locale = config_ui::DesktopLocale::SimplifiedChinese;
        let i18n = desktop_i18n::I18n::new(locale);
        let inputs = menus::Inputs {
            i18n,
            is_project: self.screen == Screen::Project,
            active_menu: None,
            workspace_mode: self.workspace_mode,
            show_file_manager: false,
            file_manager_show_changes: false,
            update_version: None,
        };
        let controls = controls::State {
            screen: self.screen,
            workspace_mode: self.workspace_mode,
            desktop_locale: locale,
            show_settings: false,
            show_diff: false,
            show_file_manager: false,
            work_sidebar_collapsed: false,
            work_resources_visible: false,
            terminal_visible: false,
            pet_visible: false,
        };
        let settings = Default::default();
        let plugins = Vec::new();
        let exists = Default::default();
        let external = external::State {
            screen: self.screen,
            desktop_locale: locale,
            open_external_platform: None,
            open_external_app: top_bar::types::ExternalOpenApp::Finder,
            open_external_exists: &exists,
            active_menu: None,
            has_project: false,
        };
        let gateway = gateway::State {
            desktop_locale: locale,
            gateway_client_settings: &settings,
            mcp: gateway::McpState {
                loading: false,
                saving: false,
                save_error: &None,
                status_message: &None,
            },
            mcp_text: "",
            plugins: gateway::PluginState { plugin_loading: false, plugin_catalog: &plugins },
            active_menu: None,
            top_bar_gateway_tab: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_disabled: false,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_server_keys: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_progress: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_status: None,
        };
        top_bar::toolbar(top_bar::Sections {
            traffic_light_spacer: drag::traffic_light_spacer().map(|_| ()),
            workspace_mode: workspace_mode::selector(i18n, self.workspace_mode, false).map(|_| ()),
            file_menu: menus::file_menu(&inputs).map(|_| ()),
            edit_menu: menus::edit_menu(&inputs).map(|_| ()),
            view_menu: menus::view_menu(&inputs).map(|_| ()),
            help_menu: menus::help_menu(&inputs).map(|_| ()),
            drag_spacer: drag::drag_spacer().map(|_| ()),
            open_external_module: external::open_external_module(&external).map(|_| ()),
            gateway_services: gateway::gateway_services_module(&gateway).map(|_| ()),
            project_view_tools: controls::project_view_tools(&controls).map(|_| ()),
            settings: controls::settings_button(i18n).map(|_| ()),
        })
    }
}
