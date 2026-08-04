use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

use super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandRegistry, CommandSource,
    CommandSpec,
};
use super::completion::slash_palette_overlay;
use super::controller::{TuiController, TuiControllerEvent};
use super::model::{UiMessage, UiMessageBase, UiMessageId, UiOverlay, UiUserMessage};
use super::render::TuiRenderer;
use super::render::layout::{FullscreenLayoutSlots, compute_fullscreen_layout};
use super::state::{TuiState, select_visible_grouped_transcript_window};

#[test]
fn tui_stress_and_security_boundaries_remain_safe_and_bounded() {
    let mut state = TuiState::default();
    state.messages = (0..10_000)
        .map(|index| {
            UiMessage::User(UiUserMessage {
                base: UiMessageBase::new(UiMessageId::local(format!("stress-{index}"))),
                text: format!("第 {index} 条消息：你好世界 👩🏽‍💻"),
            })
        })
        .collect();
    state.refresh_transcript_projection();
    state.clamp_scroll();

    state.scroll.sync_viewport(0, 0);
    let zero_window = select_visible_grouped_transcript_window(&state);
    assert!(zero_window.is_empty(), "0x0 viewport must not materialize the transcript");
    assert_eq!(
        compute_fullscreen_layout(Rect::new(0, 0, 0, 0), true, true, 0),
        FullscreenLayoutSlots::default()
    );

    let stable_prefix = state.transcript.keys()[..4].to_vec();
    state.append_message(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::local("stress-tail")),
        text: "尾部消息 🧪".to_string(),
    }));
    assert_eq!(&state.transcript.keys()[..4], stable_prefix.as_slice());
    assert_eq!(state.transcript.keys().len(), state.messages.len());

    let mut maximum_window_items = 0usize;
    for width in 1..=64 {
        state.scroll.sync_viewport(3, width);
        state.refresh_transcript_layout_for_current_width();
        let window = select_visible_grouped_transcript_window(&state);
        maximum_window_items = maximum_window_items.max(window.len());
        assert!(window.len() <= 16, "virtual window grew to {} items", window.len());
    }
    assert!(maximum_window_items > 0);
    assert!(
        state.transcript_layout.bucket_count() <= 16,
        "resize storm retained {} layout buckets",
        state.transcript_layout.bucket_count()
    );

    state.prompt.set_value(format!("{}{}", "长输入".repeat(20_000), "👩🏽‍💻"));
    let before = state.prompt.cursor.char_index;
    TuiController::default().handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::NONE,
        ))),
    );
    assert!(state.prompt.cursor.char_index < before);

    state.scroll.sync_viewport(1, 20);
    state.refresh_transcript_layout_for_current_width();
    let backend = TestBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).expect("tiny terminal");
    terminal
        .draw(|frame| {
            TuiRenderer::default().render_frame(frame, &state, "ready", "local", 0);
        })
        .expect("20x5 render remains safe");
}

#[test]
fn tui_stress_thousand_dynamic_commands_keep_narrow_palette_bounded() {
    let dynamic = (0..1_000)
        .map(|index| CommandSpec {
            name: format!("workflow:stress-{index:04}"),
            aliases: Vec::new(),
            summary: format!("stress workflow {index}"),
            takes_argument: true,
            argument_hint: Some("<input>".to_string()),
            source: CommandSource::Workflow,
            availability: CommandAvailability::Available,
            execution_policy: CommandExecutionPolicy::Queued,
            action: CommandAction::Gateway(format!("workflow:uuid-{index:04}")),
        })
        .collect::<Vec<_>>();
    let mut state = TuiState::default();
    state.command_registry = CommandRegistry::merge(vec![
        (CommandSource::Builtin, Ok(CommandRegistry::builtin().commands().to_vec())),
        (CommandSource::Workflow, Ok(dynamic)),
    ]);
    let palette = slash_palette_overlay(&state.command_registry, "workflow:stress");
    assert_eq!(palette.items.len(), 1_000);
    assert_eq!(palette.visible_rows, 8);
    assert_eq!(palette.selected_index, Some(0));
    state.overlays.push(UiOverlay::CommandPalette(palette));

    let backend = TestBackend::new(24, 8);
    let mut terminal = Terminal::new(backend).expect("narrow palette terminal");
    terminal
        .draw(|frame| {
            TuiRenderer::default().render_frame(frame, &state, "ready", "local", 0);
        })
        .expect("1,000 dynamic commands must render without panic");
}
