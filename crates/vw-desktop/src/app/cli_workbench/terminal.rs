//! Workbench terminal views are ephemeral attachments to gateway sessions.
use desktop_terminal::attached::AttachedTerminal;
use iced::{Task, Theme, window};
use vw_api_types::cli_workbench::{
    CliWorkbenchError, CliWorkbenchErrorCode as Code, CliWorkbenchProcessState as Process,
    CliWorkbenchTerminalInput, CliWorkbenchTerminalOutput, CliWorkbenchTerminalSize,
};

use super::{
    Message, SessionKey, Workbench,
    update::{Action, client, current_scope, response},
};

#[derive(Default)]
pub(super) struct Attachment {
    pub(super) view: Option<AttachedTerminal>,
    pub(super) error: Option<&'static str>,
    pub(super) truncated: bool,
    pub(super) fresh: bool,
    pending_paste: Option<(String, bool)>,
    reading: bool,
    writing: bool,
    connected: bool,
    finished: bool,
    /// Resend the last geometry after reconnecting. Input is never retried.
    size: Option<CliWorkbenchTerminalSize>,
    size_pending: bool,
}

impl Attachment {
    fn disconnect(&mut self) {
        self.connected = false;
        self.pending_paste = None;
        self.size_pending = self.size.is_some();
        if let Some(view) = &mut self.view {
            view.take_input();
        }
    }
}

pub(super) fn paste_prompt(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    prompt: String,
    submit: bool,
) -> Task<Message> {
    let prompt = prompt.replace("\r\n", "\n").replace('\r', "\n");
    // Reserve space for the bracketed paste delimiters and an optional Enter.
    let framing = 12 + usize::from(submit);
    if prompt.len() > vw_api_types::cli_workbench::CLI_WORKBENCH_MAX_TERMINAL_INPUT - framing {
        state.error = Some("终端输入过长，已丢弃");
        return Task::none();
    }
    if !state.terminal_available
        || !state.projects.get(&key.project).is_some_and(|project| {
            !project.disconnected && project.tabs.get(&key.session).is_some_and(|tab| {
                tab.session.capabilities.terminal && tab.can_send() && tab.terminal.pending_paste.is_none()
            })
        })
        || !state.locate(&key)
    {
        return Task::none();
    }
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    tab.terminal.pending_paste = Some((prompt, submit));
    state.dialog = None;
    Task::batch([focus(state), flush(state, id, key)])
}

fn visible(state: &Workbench, key: &SessionKey) -> bool {
    state.selected.as_ref() == Some(&key.project)
        && state.projects.get(&key.project).is_some_and(|project| {
            project.tabs.get(&key.session).is_some_and(|tab| project.panes.is_visible_tab(tab.view_id))
        })
}

fn writable(state: &Workbench, key: &SessionKey) -> bool {
    state.terminal_available
        && visible(state, key)
        && state.projects.get(&key.project).is_some_and(|project| !project.disconnected)
}

pub(super) fn focus(state: &Workbench) -> Task<Message> {
    let Some(project) = state.selected.as_ref().and_then(|id| state.projects.get(id)) else {
        return Task::none();
    };
    let Some(view) = project.active_tab().and_then(|tab| tab.terminal.view.as_ref())
    else {
        return Task::none();
    };
    iced_term::TerminalView::focus(view.widget_id())
}

pub(super) fn tick(state: &mut Workbench, id: window::Id, theme: &Theme) -> Task<Message> {
    let (Some(project), Some(gateway)) = (state.selected.clone(), client(state)) else {
        return Task::none();
    };
    if !state.terminal_available {
        return Task::none();
    }
    let Some(current) = state.projects.get_mut(&project) else { return Task::none() };
    if current.disconnected {
        return Task::none();
    }

    let mut tasks = Vec::new();
    let generation = state.scope_generation;
    for (pane, pane_state) in current.panes.grid.iter() {
        let Some(view_id) = pane_state.active_id else { continue };
        if !current.panes.is_visible_tab(view_id) {
            continue;
        }
        let Some((session, tab)) = current.tabs.iter_mut().find(|(_, tab)| tab.view_id == view_id) else { continue };
        if !tab.session.capabilities.terminal || tab.session.state == Process::Disconnected {
            continue;
        }
        let terminal = &mut tab.terminal;
        let focus_new =
            terminal.view.is_none() && *pane == current.panes.focused && state.dialog.is_none();
        let fresh = terminal.fresh;
        let view = terminal.view.get_or_insert_with(|| {
            AttachedTerminal::new(tab.session.directory.as_str().into(), theme, fresh)
        });
        view.set_theme(theme);
        if focus_new {
            // The first redraw mounts the new widget before it receives keyboard input.
            tasks.push(iced_term::TerminalView::focus(view.widget_id()));
        }
        if terminal.reading || terminal.finished {
            continue;
        }
        terminal.reading = true;
        let cursor = view.cursor();
        let gateway = gateway.clone();
        let key = SessionKey { project: project.clone(), session: session.clone() };
        tasks.push(Task::perform(
            async move {
                let result =
                    gateway.cli_workbench_terminal_read(&key.project, &key.session, cursor).await;
                (key, result)
            },
            move |(key, result)| {
                response(id, generation, Action::TerminalOutput(key, cursor, result))
            },
        ));
    }
    Task::batch(tasks)
}

pub(super) fn output(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    cursor: u64,
    result: Result<CliWorkbenchTerminalOutput, CliWorkbenchError>,
    surface_visible: bool,
) -> Task<Message> {
    let reply = surface_visible && writable(state, &key);
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    let terminal = &mut tab.terminal;
    terminal.reading = false;
    let running = tab.session.state == Process::Running;
    match result {
        Ok(output) => {
            if !terminal.view.as_mut().is_some_and(|view| {
                view.apply_output(
                    cursor,
                    output.start,
                    output.next,
                    &output.bytes,
                    output.truncated,
                    reply && !output.finished && running,
                )
            }) {
                terminal.disconnect();
                terminal.error = Some("终端输出无效");
                return Task::none();
            }
            terminal.connected = reply && !output.finished;
            terminal.finished = output.finished;
            terminal.truncated |= output.truncated;
            if matches!(terminal.error, Some("终端连接已断开" | "终端输出无效")) {
                terminal.error = None;
            }
            if output.finished {
                terminal.pending_paste = None;
                tab.session.state = Process::Exited;
            }
        }
        Err(error) => {
            terminal.disconnect();
            terminal.error = Some("终端连接已断开");
            if error.code == Code::NotFound {
                tab.session.state = Process::Disconnected;
            }
            return Task::none();
        }
    }
    if surface_visible {
        flush(state, id, key)
    } else {
        terminal.size_pending = terminal.size.is_some();
        Task::none()
    }
}

pub(super) fn event(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    event: iced_term::Event,
) -> Task<Message> {
    use iced_term::{BackendCommand as Command, Event, MouseButton};

    if !visible(state, &key) {
        return Task::none();
    }
    let Event::BackendCall(_, ref command) = event;
    let resize = matches!(command, Command::Resize(..));
    if state.dialog.is_some() && !resize {
        return Task::none();
    }
    let clicked = matches!(command, Command::SelectStart(..))
        || matches!(
            command,
            Command::MouseReport(
                MouseButton::LeftButton | MouseButton::MiddleButton | MouseButton::RightButton,
                _,
                _,
                true
            )
        );

    let Some(project) = state.projects.get_mut(&key.project) else { return Task::none() };
    if clicked {
        project.select(&key.session);
    }
    let active = project.active_tab().is_some_and(|tab| tab.session.id == key.session);
    if !resize && !active {
        return Task::none();
    }
    let Some(view) = project.tabs.get_mut(&key.session).and_then(|tab| tab.terminal.view.as_mut())
    else {
        return Task::none();
    };
    view.handle(event);
    let focus = if clicked { focus(state) } else { Task::none() };
    Task::batch([focus, flush(state, id, key)])
}

pub(super) fn written(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    result: Result<(), CliWorkbenchError>,
    surface_visible: bool,
) -> Task<Message> {
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    tab.terminal.writing = false;
    if let Err(error) = result {
        tab.terminal.disconnect();
        tab.terminal.error = Some(super::update::error_label(error.code));
        return Task::none();
    }
    if surface_visible {
        flush(state, id, key)
    } else {
        tab.terminal.size_pending = tab.terminal.size.is_some();
        Task::none()
    }
}

pub(super) fn backend_action(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    command: iced_term::BackendCommand,
) -> Task<Message> {
    let Some(terminal_id) = state.tab_mut(&key).and_then(|tab| tab.terminal.view.as_ref())
        .map(|view| view.terminal().id) else { return Task::none() };
    event(state, id, key, iced_term::Event::BackendCall(terminal_id, command))
}

fn flush(state: &mut Workbench, id: window::Id, key: SessionKey) -> Task<Message> {
    let send = writable(state, &key);
    let generation = state.scope_generation;
    let gateway = client(state);
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    let terminal = &mut tab.terminal;
    let Some(view) = terminal.view.as_mut() else { return Task::none() };
    if terminal.writing && send {
        return Task::none();
    }
    let (mut bytes, size, overflow) = view.take_input();
    if let Some((rows, cols)) = size {
        terminal.size = Some(CliWorkbenchTerminalSize { rows, cols });
        terminal.size_pending = true;
    }
    if overflow {
        terminal.error = Some("终端输入过长，已丢弃");
    }
    if !send
        || terminal.writing
        || !terminal.connected
        || tab.stopping
        || tab.session.state != Process::Running
    {
        return Task::none();
    }
    let Some(gateway) = gateway else { return Task::none() };
    if bytes.is_empty() && let Some((prompt, submit)) = terminal.pending_paste.take() {
        if !view.paste_prompt(prompt) {
            terminal.error = Some("终端当前无法安全粘贴这段文字");
            return Task::none();
        }
        let (pasted, _, overflow) = view.take_input();
        if overflow {
            terminal.error = Some("终端输入过长，已丢弃");
            return Task::none();
        }
        bytes = pasted;
        if submit {
            // Enter follows the closing bracketed paste delimiter in the same PTY write.
            bytes.push(b'\r');
        }
    }
    let size = if terminal.size_pending { terminal.size } else { None };
    if bytes.is_empty() && size.is_none() {
        return Task::none();
    }
    if !current_scope(state) {
        return Task::none();
    }
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    tab.terminal.writing = true;
    tab.terminal.size_pending = false;
    if !overflow {
        tab.terminal.error = None;
    }
    Task::perform(
        async move {
            let input = CliWorkbenchTerminalInput { bytes, size };
            let result =
                gateway.cli_workbench_terminal_write(&key.project, &key.session, &input).await;
            (key, result)
        },
        move |(key, result)| response(id, generation, Action::TerminalWritten(key, result)),
    )
}
