//! Chat input popover visibility transitions.
#[derive(Clone, Copy)]
pub enum Popover { Model, Mode, SendMode, File, Acp, ReasoningEffort, Usage, SessionToolSelector, SessionActions, Executor }

/// Existing visibility, ordered by the named fields.
#[derive(Clone, Copy, Default)]
pub struct Visibility {
    pub model: bool,
    pub mode: bool,
    pub send_mode: bool,
    pub file: bool,
    pub acp: bool,
    pub reasoning_effort: bool,
    pub usage: bool,
    pub session_tool_selector: bool,
    pub session_actions: bool,
    pub executor: bool,
}

pub fn toggle(mut visibility: Visibility, popover: Popover) -> Visibility {
    match popover {
        Popover::Model => {
            let new = !visibility.model;
            visibility.model = new;
            if new {
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.file = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.executor = false;
            }
        }
        Popover::Mode => {
            let new = !visibility.mode;
            visibility.mode = new;
            if new {
                visibility.model = false;
                visibility.send_mode = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.file = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.executor = false;
            }
        }
        Popover::SendMode => {
            let new = !visibility.send_mode;
            visibility.send_mode = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.file = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.executor = false;
            }
        }
        Popover::File => {
            let new = !visibility.file;
            visibility.file = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.executor = false;
            }
        }
        Popover::Acp => {
            let new = !visibility.acp;
            visibility.acp = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.usage = false;
                visibility.reasoning_effort = false;
                visibility.session_tool_selector = false;
                visibility.session_actions = false;
                visibility.executor = false;
            }
        }
        Popover::ReasoningEffort => {
            let new = !visibility.reasoning_effort;
            visibility.reasoning_effort = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.acp = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.session_actions = false;
                visibility.executor = false;
            }
        }
        Popover::Usage => {
            let new = !visibility.usage;
            visibility.usage = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.session_tool_selector = false;
                visibility.session_actions = false;
                visibility.executor = false;
            }
        }
        Popover::SessionToolSelector => {
            let new = !visibility.session_tool_selector;
            visibility.session_tool_selector = new;
            if new {
                visibility.model = false;
                visibility.mode = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.usage = false;
                visibility.session_actions = false;
                visibility.executor = false;
            }
        }
        Popover::SessionActions => {
            let new = !visibility.session_actions;
            visibility.session_actions = new;
            if new {
                visibility.mode = false;
                visibility.model = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.executor = false;
            }
        }
        Popover::Executor => {
            let new = !visibility.executor;
            visibility.executor = new;
            if new {
                visibility.mode = false;
                visibility.model = false;
                visibility.send_mode = false;
                visibility.file = false;
                visibility.acp = false;
                visibility.reasoning_effort = false;
                visibility.usage = false;
                visibility.session_tool_selector = false;
                visibility.session_actions = false;
            }
        }
    }
    visibility
}

#[cfg(test)]
#[path = "popovers_tests.rs"]
mod popovers_tests;

pub fn close(mut visibility: Visibility, popover: Popover) -> Visibility {
    match popover {
        Popover::Model => visibility.model = false,
        Popover::Mode => visibility.mode = false,
        Popover::SendMode => visibility.send_mode = false,
        Popover::File => visibility.file = false,
        Popover::Acp => visibility.acp = false,
        Popover::ReasoningEffort => visibility.reasoning_effort = false,
        Popover::Usage => visibility.usage = false,
        Popover::SessionToolSelector => visibility.session_tool_selector = false,
        Popover::SessionActions => visibility.session_actions = false,
        Popover::Executor => visibility.executor = false,
    }
    visibility
}
