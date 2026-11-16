//! Project/session overlay transitions. Cross-domain navigation and picker cleanup are explicit effects.
pub struct State<'a> {
    pub code_menu: &'a mut bool,
    pub hovered: &'a mut Option<String>,
    pub menu_id: &'a mut Option<String>,
    pub menu_anchor: &'a mut Option<iced::Point>,
    pub tools_path: &'a mut Option<String>,
    pub creating: &'a mut bool,
    pub picker_open: bool,
}
pub enum Event {
    Open(String),
    Switch(String),
    Menu(bool),
    Hover(Option<String>),
    Close,
    CreateFailed(String),
}
pub enum Effect {
    ResetPicker,
    Open(String),
    Switch(String),
    Hover(String),
    CreateFailed(String),
}
pub fn update(state: &mut State<'_>, event: Event) -> Vec<Effect> {
    match event {
        Event::Open(path) => {
            *state.code_menu = false;
            *state.hovered = None;
            *state.menu_id = None;
            *state.menu_anchor = None;
            vec![Effect::ResetPicker, Effect::Open(path)]
        }
        Event::Switch(path) => {
            *state.hovered = None;
            *state.menu_id = None;
            *state.menu_anchor = None;
            vec![Effect::ResetPicker, Effect::Switch(path)]
        }
        Event::Menu(open) => {
            *state.code_menu = open;
            *state.hovered = None;
            Vec::new()
        }
        Event::Hover(path) => {
            *state.hovered = path.clone();
            path.map(|path| vec![Effect::Hover(path)]).unwrap_or_default()
        }
        Event::Close => {
            if state.picker_open {
                return Vec::new();
            }
            *state.hovered = None;
            *state.menu_id = None;
            *state.menu_anchor = None;
            *state.tools_path = None;
            vec![Effect::ResetPicker]
        }
        Event::CreateFailed(error) => {
            *state.creating = false;
            vec![Effect::CreateFailed(error)]
        }
    }
}
