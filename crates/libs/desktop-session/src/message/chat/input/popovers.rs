//! 输入框上方互斥菜单的局部状态。
#[derive(Debug, Clone, Default)]
pub struct InputPopoverState {
    pub recommended_prompt: bool,
    pub quick_message_group: Option<config_ui::QuickMessageGroup>,
    pub shiju_skill: bool,
    pub agent_sessions: bool,
}

impl InputPopoverState {
    pub fn set_recommended_prompt(&mut self, open: bool) {
        self.recommended_prompt = open;
        if open {
            self.quick_message_group = None;
            self.shiju_skill = false;
        }
    }

    pub fn set_quick_message_group(&mut self, group: Option<config_ui::QuickMessageGroup>) {
        if group.is_some() {
            self.recommended_prompt = false;
            self.shiju_skill = false;
        }
        self.quick_message_group = group;
    }

    pub fn toggle_shiju_skill(&mut self) -> bool {
        self.shiju_skill = !self.shiju_skill;
        if self.shiju_skill {
            self.quick_message_group = None;
            self.recommended_prompt = false;
        }
        self.shiju_skill
    }

    pub fn toggle_agent_sessions(&mut self, has_running_children: bool) {
        self.agent_sessions = has_running_children && !self.agent_sessions;
    }
}
