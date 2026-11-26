//! 输入区互斥菜单和新会话推荐场景。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ComposerScene {
    #[default]
    Office,
    Development,
    Design,
}

#[derive(Debug, Clone, Default)]
pub struct InputPopoverState {
    pub scene: ComposerScene,
    pub category: Option<usize>,
    pub recommended_prompt: bool,
    pub quick_message_group: Option<config_ui::QuickMessageGroup>,
    pub employee_commands: bool,
    pub shiju_skill: bool,
    pub media_shortcuts: bool,
    pub agent_sessions: bool,
}

impl InputPopoverState {
    pub fn set_recommended_prompt(&mut self, open: bool) {
        self.recommended_prompt = open;
        if open {
            self.employee_commands = false;
            self.quick_message_group = None;
            self.shiju_skill = false;
            self.media_shortcuts = false;
        }
    }

    pub fn set_quick_message_group(&mut self, group: Option<config_ui::QuickMessageGroup>) {
        if group.is_some() {
            self.employee_commands = false;
            self.recommended_prompt = false;
            self.shiju_skill = false;
            self.media_shortcuts = false;
        }
        self.quick_message_group = group;
    }

    pub fn toggle_shiju_skill(&mut self) -> bool {
        self.shiju_skill = !self.shiju_skill;
        if self.shiju_skill {
            self.employee_commands = false;
            self.quick_message_group = None;
            self.recommended_prompt = false;
            self.media_shortcuts = false;
        }
        self.shiju_skill
    }

    pub fn set_media_shortcuts(&mut self, open: bool) {
        self.media_shortcuts = open;
        if open {
            self.employee_commands = false;
            self.quick_message_group = None;
            self.recommended_prompt = false;
            self.shiju_skill = false;
        }
    }

    pub fn set_employee_commands(&mut self, open: bool) {
        self.employee_commands = open;
        if open {
            self.quick_message_group = None;
            self.recommended_prompt = false;
            self.shiju_skill = false;
            self.media_shortcuts = false;
        }
    }

    pub fn toggle_agent_sessions(&mut self, has_running_children: bool) {
        self.agent_sessions = has_running_children && !self.agent_sessions;
    }
}
