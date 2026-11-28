//! 验证顶部栏各分区组合、页面状态和窗口级交互。
//!
//! 组合测试在不同页面和窗口状态下构造顶部栏，确认各分区按条件出现。

use crate::components::top_bar;
use crate::components::top_bar::types::WorkspaceMode;
use crate::ui_types::Screen;
use iced::advanced::Layout;
use iced::advanced::layout::Limits;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree::Tag;
use iced::{Font, Length, Renderer, Size};

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

const VIEW_MENU_INDEX: usize = if cfg!(target_os = "macos") { 4 } else { 3 };

#[test]
fn view_builds_three_toolbar_sections_with_history_in_the_middle() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;
    let element = app.view();

    assert_eq!(
        element.as_widget().size(),
        Size::new(Length::Fill, Length::Fixed(top_bar::TOP_BAR_HEIGHT))
    );

    let children = element.as_widget().children();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].children.len(), VIEW_MENU_INDEX + 3);
    assert_eq!(children[1].children.len(), 2);
    assert_eq!(children[2].children.len(), 5);
}

#[tokio::test]
async fn history_buttons_are_centered_at_different_window_widths() {
    let renderer = Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap();
    let mut element = test_app().view();
    let mut tree = Tree::new(element.as_widget());

    for width in [800.0, 1440.0] {
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &Limits::new(Size::ZERO, Size::new(width, top_bar::TOP_BAR_HEIGHT)),
        );
        let layout = Layout::new(&node);
        let row = layout.children().next().unwrap();
        let history = row.children().nth(1).unwrap();
        assert!((history.bounds().center_x() - width / 2.0).abs() < 0.5);
    }
}

#[test]
fn work_top_bar_keeps_all_global_menus_and_project_tools() {
    let mut code_app = test_app();
    code_app.screen = Screen::Project;
    code_app.workspace_mode = WorkspaceMode::Code;
    let code_element = code_app.view();
    let code_children = code_element.as_widget().children();

    let mut work_app = test_app();
    work_app.screen = Screen::Project;
    work_app.workspace_mode = WorkspaceMode::Work;
    let work_element = work_app.view();
    let work_children = work_element.as_widget().children();

    let code_left = &code_children[0].children;
    let work_left = &work_children[0].children;
    let code_right = &code_children[2].children;
    let work_right = &work_children[2].children;
    // 全局分区在两种模式下保留；项目工具按各自工作空间提供。
    for (code, work) in [
        (&code_children[1..2], &work_children[1..2]),
        (&code_left[..VIEW_MENU_INDEX], &work_left[..VIEW_MENU_INDEX]),
        (&code_left[VIEW_MENU_INDEX + 1..], &work_left[VIEW_MENU_INDEX + 1..]),
        (&code_right[..3], &work_right[..3]),
        (&code_right[4..], &work_right[4..]),
    ] {
        assert_eq!(
            ordered_tree_shape(work),
            ordered_tree_shape(code),
        );
    }

    // 视图菜单保留入口，展开项随工作模式变化。
    let code_view = &code_left[VIEW_MENU_INDEX].children;
    let work_view = &work_left[VIEW_MENU_INDEX].children;
    assert_eq!(code_view.len(), 2);
    assert_eq!(work_view.len(), 2);
    assert_eq!(ordered_tree_shape(&work_view[..1]), ordered_tree_shape(&code_view[..1]));
    assert_eq!(work_view[1].children.len(), 4);
    assert_eq!(code_view[1].children.len(), if cfg!(target_arch = "wasm32") { 4 } else { 5 });

    let code_tools = &code_right[3].children;
    let work_tools = &work_right[3].children;
    let native_tools = if cfg!(target_arch = "wasm32") { 0 } else { 2 };
    assert_eq!(code_tools.len(), 4 + native_tools);
    assert_eq!(work_tools.len(), 3 + native_tools);
    // Work 的两种侧栏替代 Code 的侧栏、审查和文件树；其后公共工具保持顺序。
    assert_eq!(ordered_tree_shape(&work_tools[2..]), ordered_tree_shape(&code_tools[3..]));
    assert!(work_tools[..2].iter().all(|tool| tool.children.len() == 2));
}

#[test]
fn workspace_mode_selector_keeps_code_controls() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;

    let element = app.view();

    assert_eq!(element.as_widget().children()[0].children.len(), VIEW_MENU_INDEX + 3);
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
            screen: self.screen,
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
            left_drag_spacer: drag::drag_spacer().map(|_| ()),
            page_history: iced::widget::row![
                iced::widget::Space::new().width(24).height(24),
                iced::widget::Space::new().width(24).height(24),
            ]
            .spacing(4)
            .into(),
            right_drag_spacer: drag::drag_spacer().map(|_| ()),
            open_external_module: external::open_external_module(&external).map(|_| ()),
            gateway_services: gateway::gateway_services_module(&gateway).map(|_| ()),
            project_view_tools: controls::project_view_tools(&controls).map(|_| ()),
            settings: controls::settings_button(i18n).map(|_| ()),
        })
    }
}
