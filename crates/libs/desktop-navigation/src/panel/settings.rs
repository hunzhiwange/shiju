//! Settings/panel navigation state and explicit cross-domain requests.
use crate::{
    components::{
        cli_command_help::CliCommandsTab,
        top_bar::{
            menus::MenuType,
            types::{TopBarGatewayTab, WorkspaceMode},
        },
    },
    ui_types::SettingsTab,
};
use desktop_chat_input::popovers::Visibility;
use desktop_settings_shell::SystemTab;
pub struct State {
    pub show_settings: bool,
    pub settings_tab: SettingsTab,
    pub show_system_settings: bool,
    pub system_settings_tab: SystemTab,
    pub system_settings_help_tab: Option<SystemTab>,
    pub active_menu: Option<MenuType>,
    pub show_about_modal: bool,
    pub auto_max_mode: bool,
    pub show_diff: bool,
    pub top_bar_gateway_tab: TopBarGatewayTab,
    pub cli_commands_tab: CliCommandsTab,
}
impl Default for State {
    fn default() -> Self {
        Self {
            show_settings: false,
            settings_tab: SettingsTab::Sessions,
            show_system_settings: false,
            system_settings_tab: SystemTab::General,
            system_settings_help_tab: None,
            active_menu: None,
            show_about_modal: false,
            auto_max_mode: false,
            show_diff: true,
            top_bar_gateway_tab: Default::default(),
            cli_commands_tab: Default::default(),
        }
    }
}
pub enum Message {
    ToggleSettingsPanel,
    ProjectFileNewSession,
    ProjectFileNewProject,
    ProjectFileShowSessions,
    ProjectFileShowProjects,
    ProjectFileSaveAll,
    ToggleSystemSettings,
    OpenAcpSettings,
    OpenSystemSettingsTab(SystemTab),
    OpenSystemSettingsModelDetail(String, String),
    ToggleAboutModal,
    CliCommandsTabSelected(CliCommandsTab),
    ToggleDiffPanel,
    ToggleGitDiffSummary,
    FileManagerPanelVisible(bool),
    AutoMaxToggled(bool),
    ToggleMenu(Option<MenuType>),
    GatewayServicesTabSelected(TopBarGatewayTab),
    MenuAction,
    MenuHovered(MenuType),
}
pub enum Effect {
    ToggleWorkSidebar,
    SettingsVisibilityChanged(bool),
    CreateSession(String),
    NewProject,
    SaveAllFiles,
    SystemSettingsVisibilityChanged(bool),
    OpenAcpSettings,
    SelectSystemTab(SystemTab),
    OpenModelDetail(String, String),
    SavePreference(&'static str, serde_json::Value),
    RefreshPlugins,
    RefreshMcp,
}
pub struct Context<'a> {
    pub state: &'a mut State,
    pub popovers: &'a mut Visibility,
    pub drag: &'a mut crate::view_layout::drag::State,
    pub git_diff_summary: &'a mut bool,
    pub file_manager_visible: &'a mut bool,
    pub plugin_catalog_empty: bool,
    pub plugin_loading: bool,
    pub mcp_loading: bool,
    pub mcp_saving: bool,
}
pub fn update(
    app: Context<'_>,
    workspace_mode: WorkspaceMode,
    project_path: Option<&str>,
    message: Message,
) -> Option<Effect> {
    let state = app.state;
    match message {
        Message::ToggleSettingsPanel => {
            if workspace_mode == WorkspaceMode::Work {
                return Some(Effect::ToggleWorkSidebar);
            }
            state.show_settings = !state.show_settings;
            return Some(Effect::SettingsVisibilityChanged(state.show_settings));
        }
        Message::ProjectFileNewSession => {
            return project_path.map(|path| Effect::CreateSession(path.into()));
        }
        Message::ProjectFileNewProject => return Some(Effect::NewProject),
        Message::ProjectFileShowSessions => {
            state.show_settings = true;
            state.settings_tab = SettingsTab::Sessions;
            return Some(Effect::SavePreference("show_settings", true.into()));
        }
        Message::ProjectFileShowProjects => {
            state.show_settings = true;
            state.settings_tab = SettingsTab::Projects;
            return Some(Effect::SavePreference("show_settings", true.into()));
        }
        Message::ProjectFileSaveAll => return Some(Effect::SaveAllFiles),
        Message::ToggleSystemSettings => {
            state.show_system_settings = !state.show_system_settings;
            state.system_settings_help_tab = None;
            return Some(Effect::SystemSettingsVisibilityChanged(state.show_system_settings));
        }
        Message::OpenAcpSettings => {
            state.show_system_settings = true;
            state.system_settings_help_tab = None;
            state.active_menu = None;
            app.popovers.acp = false;
            state.system_settings_tab = SystemTab::Agents;
            return Some(Effect::OpenAcpSettings);
        }
        Message::OpenSystemSettingsTab(tab) => {
            state.show_system_settings = true;
            state.system_settings_help_tab = None;
            state.active_menu = None;
            app.popovers.model = false;
            app.popovers.mode = false;
            app.popovers.file = false;
            app.popovers.usage = false;
            app.popovers.session_tool_selector = false;
            return Some(Effect::SelectSystemTab(tab));
        }
        Message::OpenSystemSettingsModelDetail(provider, model) => {
            state.show_system_settings = true;
            state.system_settings_help_tab = None;
            state.active_menu = None;
            app.popovers.model = false;
            app.popovers.mode = false;
            app.popovers.file = false;
            app.popovers.usage = false;
            state.system_settings_tab = SystemTab::Models;
            return Some(Effect::OpenModelDetail(provider, model));
        }
        Message::ToggleAboutModal => state.show_about_modal = !state.show_about_modal,
        Message::CliCommandsTabSelected(tab) => state.cli_commands_tab = tab,
        Message::ToggleDiffPanel => {
            if workspace_mode != WorkspaceMode::Work {
                state.show_diff = !state.show_diff;
            }
        }
        Message::ToggleGitDiffSummary => {
            *app.git_diff_summary = !*app.git_diff_summary;
            return Some(Effect::SavePreference(
                "show_git_diff_summary",
                (*app.git_diff_summary).into(),
            ));
        }
        Message::FileManagerPanelVisible(visible) => {
            *app.file_manager_visible = visible;
            if !visible {
                app.drag.dragging_file_manager = false;
                app.drag.file_manager_drag_anchor_x = None;
            }
            return Some(Effect::SavePreference("show_file_manager", visible.into()));
        }
        Message::AutoMaxToggled(value) => {
            state.auto_max_mode = value;
            return Some(Effect::SavePreference("auto_max_mode", value.into()));
        }
        Message::ToggleMenu(menu) => {
            state.active_menu = menu.filter(|menu| Some(*menu) != state.active_menu)
        }
        Message::GatewayServicesTabSelected(tab) => {
            state.top_bar_gateway_tab = tab;
            if tab == TopBarGatewayTab::Plugins && app.plugin_catalog_empty && !app.plugin_loading {
                return Some(Effect::RefreshPlugins);
            }
            if tab == TopBarGatewayTab::Mcp && !app.mcp_loading && !app.mcp_saving {
                return Some(Effect::RefreshMcp);
            }
        }
        Message::MenuAction => state.close_menu(),
        Message::MenuHovered(menu) => {
            if state.active_menu.is_some() {
                state.active_menu = Some(menu);
            }
        }
    }
    None
}

impl State {
    pub fn close_menu(&mut self) {
        self.active_menu = None;
    }
    pub fn focus_chat(&mut self) {
        self.show_system_settings = false;
        self.show_settings = false;
    }
    pub fn prepare_companion_settings(&mut self) {
        self.system_settings_tab = SystemTab::General;
    }
}
