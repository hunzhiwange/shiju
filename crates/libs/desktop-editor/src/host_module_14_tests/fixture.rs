use crate::message::editor::{self, EditorMessage};
use crate::preview::message::PreviewMessage;
#[derive(Default)]
pub struct Fixture {
    pub active_preview_path: Option<String>,

    pub cursor_position: iced::Point,

    pub editor_popover: Option<crate::components::preview_panel::view::options::EditorPopover>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_applying_completion: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_disabled: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_hide_deadline: Option<std::time::Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_pending: Option<crate::preview::LspHoverPending>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_manager: Option<desktop_lsp::LspServiceManager>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_overlay: iced_code_editor::LspOverlayState,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_overlay_path: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_progress: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::preview::LspProgress>,
    >,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_status_message: Option<(String, desktop_i18n::format::Message)>,
    #[cfg(not(target_arch = "wasm32"))]
    pub notifications: crate::preview::lsp::Notifications,

    pub preview_auto_save_mode: config_ui::PreviewAutoSaveMode,

    pub preview_context_menu_pos: Option<(f32, f32)>,

    pub preview_context_target: Option<(String, usize, usize, usize, usize)>,
    pub preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)>,

    pub preview_tabs: Vec<crate::preview::PreviewTab>,
    #[cfg(not(target_arch = "wasm32"))]
    pub project_path: Option<String>,

    pub show_preview_context_menu: bool,

    pub show_preview_fullscreen_overlay: bool,

    pub focus_preview: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub persist_lsp: Option<bool>,
    pub show_preview_settings: bool,
    pub preview_trace_back: Vec<(String, usize, usize)>,
    pub preview_trace_forward: Vec<(String, usize, usize)>,
    pub preview_trace_navigating: bool,
    pub pending_preview_goto: Option<(String, usize, usize)>,
    pub preview_messages: std::collections::HashMap<String, desktop_i18n::format::Message>,
    pub preview_tab_menu_path: Option<String>,
    pub preview_tab_menu_pos: Option<iced::Point>,
}
impl Fixture {
    pub fn tab(&mut self, message: PreviewMessage) {
        crate::preview::tab::prepare_popover(&mut self.editor_popover, &message);
        let appearance = crate::preview::tab::Appearance {
            theme: iced::Theme::Dark,
            font_size: 14.0,
            line_height: 20.0,
            locale: config_ui::DesktopLocale::default(),
            cursor_position: self.cursor_position,
        };
        let _ = crate::preview::tab::update(
            &mut crate::preview::tab::TabState {
                active_preview_path: &mut self.active_preview_path,
                preview_tabs: &mut self.preview_tabs,
                preview_context_target: &mut self.preview_context_target,
                preview_trace_back: &mut self.preview_trace_back,
                preview_trace_forward: &mut self.preview_trace_forward,
                preview_trace_navigating: &mut self.preview_trace_navigating,
                pending_preview_goto: &mut self.pending_preview_goto,
                preview_messages: &mut self.preview_messages,
                preview_auto_save_mode: &mut self.preview_auto_save_mode,
                show_preview_context_menu: &mut self.show_preview_context_menu,
                preview_context_menu_pos: &mut self.preview_context_menu_pos,
                preview_nav_popup: &mut self.preview_nav_popup,
                preview_tab_menu_path: &mut self.preview_tab_menu_path,
                preview_tab_menu_pos: &mut self.preview_tab_menu_pos,
            },
            appearance,
            &mut Vec::new(),
            message,
        );
    }

    pub fn send(
        &mut self,
        event: iced_code_editor::Message,
    ) -> iced::Task<crate::preview::editor::Effect> {
        self.dispatch(PreviewMessage::EditorEvent(event))
    }
    pub fn dispatch(
        &mut self,
        message: PreviewMessage,
    ) -> iced::Task<crate::preview::editor::Effect> {
        crate::preview::editor::update(
            &mut self.active_preview_path,
            &mut self.cursor_position,
            &mut self.editor_popover,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_applying_completion,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_disabled,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_hover_anchor,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_hover_hide_deadline,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_hover_pending,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_manager,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_overlay,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_overlay_path,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_progress,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_status,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.lsp_status_message,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.notifications,
            &mut self.preview_auto_save_mode,
            &mut self.preview_context_menu_pos,
            &mut self.preview_context_target,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.preview_nav_popup,
            &mut self.preview_tabs,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.project_path,
            &mut self.show_preview_context_menu,
            &mut self.show_preview_fullscreen_overlay,
            &mut self.focus_preview,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.persist_lsp,
            message,
        )
    }
}
pub fn update(state: &mut Fixture, message: EditorMessage) -> iced::Task<editor::Event> {
    let tab = state
        .preview_tabs
        .iter_mut()
        .find(|tab| Some(&tab.path) == state.active_preview_path.as_ref());
    if !editor::prepare_message(
        false,
        tab.as_ref().is_some_and(|tab| tab.read_only),
        &mut state.editor_popover,
        &message,
    ) {
        return iced::Task::none();
    }
    if let EditorMessage::TogglePopover(kind) = message {
        return editor::open_popover(
            tab.map(|tab| &mut tab.editor),
            &mut state.editor_popover,
            &mut state.show_preview_context_menu,
            &mut state.preview_nav_popup,
            &mut state.show_preview_settings,
            #[cfg(not(target_arch = "wasm32"))]
            &mut state.lsp_overlay,
            #[cfg(not(target_arch = "wasm32"))]
            &mut state.lsp_overlay_path,
            kind,
        );
    }
    editor::update(
        tab.map(|tab| &mut tab.editor),
        &mut state.editor_popover,
        &mut state.show_preview_settings,
        message,
    )
}
pub fn options(
    state: &Fixture,
) -> iced::Element<'_, crate::components::preview_panel::view::Message> {
    let editor = state
        .preview_tabs
        .iter()
        .find(|tab| Some(&tab.path) == state.active_preview_path.as_ref())
        .map(|tab| &tab.editor.inner);
    crate::components::preview_panel::view::editor_options::view(
        editor,
        desktop_i18n::I18n::new(config_ui::DesktopLocale::default()),
    )
}
