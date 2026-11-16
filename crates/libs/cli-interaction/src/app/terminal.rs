//! 终端事件循环与绘制。

use std::time::Instant;

use super::{
    ActiveRunner,
    CrosstermBackend,
    Duration,
    Event,
    KeyEventKind,
    Rect,
    Result,
    RunnerControl,
    TERMINAL_EVENT_BUDGET,
    TerminalMode,
    TranscriptPager,
    V3Action,
    V3GatewayApp,
    V3Terminal,
    event,
    io,
};

#[derive(Debug, Default)]
pub(crate) struct TerminalBatchOutcome {
    pub(crate) processed: usize,
    pub(crate) quit: bool,
    pub(crate) editor_terminal_suspended: Option<vw_tui::ExternalEditorSuspension>,
}

pub(crate) fn apply_ready_terminal_events(
    app: &mut V3GatewayApp,
    runner: &mut ActiveRunner,
    first_event: Event,
    mut read_ready: impl FnMut() -> io::Result<Option<Event>>,
) -> io::Result<TerminalBatchOutcome> {
    let mut outcome = TerminalBatchOutcome::default();
    let mut next_event = Some(first_event);

    while let Some(current_event) = next_event {
        outcome.processed += 1;
        if runner.has_transcript_pager() {
            match current_event {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    runner.handle_pager_key(key);
                }
                Event::Resize(width, _) => runner.refresh_pager(app, width),
                _ => {}
            }
        } else {
            let action = match current_event {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    app.widget_mut().handle_key_event(key)
                }
                Event::Paste(text) => app.widget_mut().handle_paste(text),
                Event::Mouse(mouse) => app.widget_mut().handle_mouse_event(mouse),
                Event::Resize(width, _) => {
                    let _ = app.resize_reflow_lines(width);
                    V3Action::None
                }
                _ => V3Action::None,
            };

            let opens_editor = matches!(action, V3Action::OpenExternalEditor);
            let suspension = if opens_editor && !runner.editor_in_flight() {
                Some(vw_tui::ExternalEditorSuspension::suspend(app.terminal_mode())?)
            } else {
                None
            };
            let control = runner.execute_action(app, action);
            if let Some(suspension) = suspension {
                if runner.editor_in_flight() {
                    outcome.editor_terminal_suspended = Some(suspension);
                } else {
                    suspension.resume()?;
                }
            }
            if control == RunnerControl::Quit {
                outcome.quit = true;
                break;
            }
        }

        // Once the editor owns terminal input, leave remaining events to it.
        if runner.editor_in_flight()
            || runner.native_copy_requested
            || outcome.processed == TERMINAL_EVENT_BUDGET
        {
            break;
        }
        next_event = read_ready()?;
    }

    Ok(outcome)
}

pub(super) fn run_event_loop(mut app: V3GatewayApp) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = V3Terminal::with_options(backend)?;
    let mut runner = ActiveRunner::new();
    runner.request_session_bootstrap(&mut app);
    let mut editor_terminal_suspended: Option<vw_tui::ExternalEditorSuspension> = None;

    loop {
        let background_work_remaining = runner.drain(&mut app);
        if !runner.editor_in_flight()
            && let Some(suspension) = editor_terminal_suspended.take()
        {
            suspension.resume()?;
            terminal.clear()?;
        }
        if runner.editor_in_flight() {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let wait =
            if background_work_remaining { Duration::ZERO } else { Duration::from_millis(50) };
        let wait = if runner.has_transcript_pager() {
            wait
        } else {
            app.widget().next_paste_flush_deadline().map_or(wait, |deadline| {
                wait.min(deadline.saturating_duration_since(Instant::now()))
            })
        };
        if event::poll(wait)? {
            let first_event = event::read()?;
            let outcome = apply_ready_terminal_events(&mut app, &mut runner, first_event, || {
                if event::poll(Duration::ZERO)? { event::read().map(Some) } else { Ok(None) }
            })?;
            if outcome.editor_terminal_suspended.is_some() {
                editor_terminal_suspended = outcome.editor_terminal_suspended;
            }
            if outcome.quit {
                return Ok(());
            }
        }

        // The external editor owns stdout as well as stdin until completion.
        if runner.editor_in_flight() {
            continue;
        }

        if runner.native_copy_requested {
            let result = super::native_copy::run(&mut app, &mut runner);
            runner.native_copy_requested = false;
            result?;
            // The static view wrote outside the diff renderer. Invalidate its cache.
            terminal.clear()?;
        }

        if !runner.has_transcript_pager() && !runner.editor_in_flight() {
            app.widget_mut().flush_paste_burst_if_due(Instant::now());
        }
        let width = crossterm::terminal::size()?.0;
        runner.refresh_pager(&app, width);
        draw(&app, runner.transcript_pager.as_mut(), &mut terminal)?;
    }
}

pub(super) fn draw(
    app: &V3GatewayApp,
    pager: Option<&mut TranscriptPager>,
    terminal: &mut V3Terminal,
) -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?;
    let y = match app.terminal_mode() {
        TerminalMode::Inline => terminal.viewport_area.y.min(height.saturating_sub(1)),
        TerminalMode::Alternate => 0,
    };
    terminal.set_viewport_area(Rect::new(0, y, width, height.saturating_sub(y)));
    terminal.draw(|frame| {
        let area = frame.area();
        if let Some(pager) = pager {
            pager.render(area, frame.buffer_mut());
        } else {
            app.widget().render(area, frame.buffer_mut());
            frame.set_cursor_style(app.widget().cursor_style(area));
            if let Some(position) = app.widget().cursor_pos(area) {
                frame.set_cursor_position(position);
            }
        }
    })
}
