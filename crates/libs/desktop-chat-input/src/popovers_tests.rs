use super::*;
fn visibility(state: &Visibility) -> [bool; 10] {
    [state.model, state.mode, state.send_mode, state.file, state.acp, state.reasoning_effort, state.usage, state.session_tool_selector, state.session_actions, state.executor]
}

fn set_target_visibility(state: &mut Visibility, message: &Popover, visible: bool) {
    match message {
        Popover::Model => state.model = visible,
        Popover::Mode => state.mode = visible,
        Popover::SendMode => state.send_mode = visible,
        Popover::File => state.file = visible,
        Popover::Acp => state.acp = visible,
        Popover::ReasoningEffort => {
            state.reasoning_effort = visible;
        }
        Popover::Usage => state.usage = visible,
        Popover::SessionToolSelector => {
            state.session_tool_selector = visible;
        }
        Popover::SessionActions => {
            state.session_actions = visible;
        }
        Popover::Executor => state.executor = visible,
    }
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn opening_each_popover_applies_its_existing_exclusivity_rules() {
    let cases = [
        (
            Popover::Model,
            [true, false, false, false, false, false, false, false, true, false],
        ),
        (
            Popover::Mode,
            [false, true, false, false, false, false, false, false, true, false],
        ),
        (
            Popover::SendMode,
            [false, false, true, false, false, false, false, false, true, false],
        ),
        (
            Popover::File,
            [false, false, false, true, false, false, false, false, true, false],
        ),
        (
            Popover::Acp,
            [false, false, false, false, true, false, false, false, false, false],
        ),
        (
            Popover::ReasoningEffort,
            [false, false, false, false, false, true, false, false, false, false],
        ),
        (
            Popover::Usage,
            [false, false, false, false, false, false, true, false, false, false],
        ),
        (
            Popover::SessionToolSelector,
            [false, false, false, false, false, false, false, true, false, false],
        ),
        (
            Popover::SessionActions,
            [false, false, false, false, false, false, false, false, true, false],
        ),
        (
            Popover::Executor,
            [false, false, false, false, false, false, false, false, false, true],
        ),
    ];

    for (message, expected) in cases {
        let mut state = Visibility { model: true, mode: true, send_mode: true, file: true, acp: true, reasoning_effort: true, usage: true, session_tool_selector: true, session_actions: true, executor: true };
        set_target_visibility(&mut state, &message, false);

        state = toggle(state, message);

        assert_eq!(visibility(&state), expected);
    }
}

#[test]
fn toggling_an_open_popover_closes_only_that_popover() {
    let cases = [
        (Popover::Model, 0),
        (Popover::Mode, 1),
        (Popover::SendMode, 2),
        (Popover::File, 3),
        (Popover::Acp, 4),
        (Popover::ReasoningEffort, 5),
        (Popover::Usage, 6),
        (Popover::SessionToolSelector, 7),
        (Popover::SessionActions, 8),
        (Popover::Executor, 9),
    ];

    for (message, closed_index) in cases {
        let mut state = Visibility { model: true, mode: true, send_mode: true, file: true, acp: true, reasoning_effort: true, usage: true, session_tool_selector: true, session_actions: true, executor: true };

        state = toggle(state, message);

        let mut expected = [true; 10];
        expected[closed_index] = false;
        assert_eq!(visibility(&state), expected);
    }
}
