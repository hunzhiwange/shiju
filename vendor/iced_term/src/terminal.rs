use crate::AlacrittyEvent;
use crate::actions::Action;
use crate::backend;
use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::font::TermFont;
use crate::settings::{FontSettings, Settings, ThemeSettings};
use crate::theme::{ColorPalette, Theme};
use iced::Subscription;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::canvas::Cache;
use std::hash::{Hash, Hasher};
use std::io::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone)]
pub enum Event {
    BackendCall(u64, backend::Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    ChangeTheme(Box<ColorPalette>),
    ChangeFont(FontSettings),
    AddBindings(Vec<(Binding<InputKind>, BindingAction)>),
    ProxyToBackend(backend::Command),
}

pub struct Terminal {
    pub id: u64,
    widget_id: iced::widget::Id,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) backend: backend::Backend,
    backend_event_rx: Arc<Mutex<Receiver<AlacrittyEvent>>>,
}

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(100);
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);
        let backend = backend::Backend::new(id, sender, settings.backend)?;
        Ok(Self::from_backend(id, theme, font, backend, receiver))
    }

    /// Create a terminal renderer without launching a local process.
    pub fn attach(id: u64, settings: Settings) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let font = TermFont::new(settings.font);
        let backend = backend::Backend::attach(
            sender,
            settings.backend.working_directory,
            font.measure,
        );
        Self::from_backend(
            id,
            Theme::new(settings.theme),
            font,
            backend,
            receiver,
        )
    }

    fn from_backend(
        id: u64,
        theme: Theme,
        font: TermFont,
        backend: backend::Backend,
        receiver: Receiver<AlacrittyEvent>,
    ) -> Self {
        Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            backend,
            backend_event_rx: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Feed raw PTY data, retaining parser state between batches.
    pub fn feed_attached_output(
        &mut self,
        bytes: &[u8],
        reset: bool,
        reply: bool,
    ) {
        let changed = self.backend.feed_output(bytes, reset);
        if let Ok(mut receiver) = self.backend_event_rx.try_lock() {
            while let Ok(event) = receiver.try_recv() {
                if !reply {
                    continue;
                }
                match event {
                    AlacrittyEvent::PtyWrite(_)
                    | AlacrittyEvent::TextAreaSizeRequest(_) => {
                        self.backend.handle(
                            backend::Command::ProcessAlacrittyEvent(event),
                        );
                    },
                    AlacrittyEvent::ColorRequest(index, format) => {
                        use alacritty_terminal::vte::ansi::{
                            Color, NamedColor, Rgb,
                        };
                        let color = match index {
                            0..=255 => Color::Indexed(index as u8),
                            i if i == NamedColor::Foreground as usize
                                || i == NamedColor::Cursor as usize =>
                            {
                                Color::Named(NamedColor::Foreground)
                            },
                            i if i == NamedColor::Background as usize => {
                                Color::Named(NamedColor::Background)
                            },
                            _ => continue,
                        };
                        let [r, g, b, _] =
                            self.theme.get_color(color).into_rgba8();
                        self.backend
                            .write(format(Rgb { r, g, b }).into_bytes());
                    },
                    _ => {},
                }
            }
        }
        if changed {
            self.backend.sync();
            self.redraw();
        }
    }

    /// Return the pending bytes, latest grid size (rows, columns), and overflow state.
    pub fn take_attached_input(
        &mut self,
    ) -> (Vec<u8>, Option<(u16, u16)>, bool) {
        let (bytes, size, overflow) = self.backend.take_attached_input();
        (
            bytes,
            size.map(|size| (size.num_lines, size.num_cols)),
            overflow,
        )
    }

    pub fn bracketed_paste_enabled(&self) -> bool {
        self.backend.renderable_content().terminal_mode.contains(crate::TermMode::BRACKETED_PASTE)
    }

    /// The latest known shell directory, falling back to its launch directory.
    pub fn working_directory(&self) -> Option<std::path::PathBuf> {
        self.backend.working_directory()
    }

    pub fn widget_id(&self) -> &iced::widget::Id {
        &self.widget_id
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    /// The terminal's text, scrollback first and the visible screen last, one
    /// `String` per line.
    ///
    /// For callers that need to read what the terminal shows rather than only
    /// draw it: scraping the output of a command an application wrote into the
    /// PTY itself, asserting on a terminal's contents in a test, or handing the
    /// screen to an accessibility layer. [`Self::handle`] can write into the
    /// PTY already; this is the other half of that conversation.
    ///
    /// A line is a *logical* line, not a grid row: output that wrapped across
    /// the terminal's width comes back joined, so a reader is not looking at
    /// where the window happened to be split. Trailing blanks are trimmed.
    pub fn text(&self) -> Vec<String> {
        self.backend.text()
    }

    /// The current selection as plain text.
    pub fn selected_text(&self) -> String {
        self.backend.selectable_content()
    }

    pub fn has_selection(&self) -> bool {
        self.backend.has_selection()
    }

    pub fn handle(&mut self, cmd: Command) -> Action {
        let mut action = Action::default();

        match cmd {
            Command::ChangeTheme(color_pallete) => {
                self.theme = Theme::new(ThemeSettings::new(color_pallete));
            },
            Command::ChangeFont(font_settings) => {
                self.font = TermFont::new(font_settings);
            },
            Command::AddBindings(bindings) => {
                self.bindings.add_bindings(bindings);
            },
            Command::ProxyToBackend(backend::Command::Write(input)) => {
                // PTY output will sync the grid. Input only redraws immediately
                // when it changes the viewport or vi-mode selection.
                if self.backend.write_input(input) {
                    self.redraw();
                }
                return action;
            },
            Command::ProxyToBackend(
                cmd @ (backend::Command::SelectUpdate(..)
                | backend::Command::SelectScroll(..)
                | backend::Command::WheelScroll { .. }
                | backend::Command::ScrollTo(..)),
            ) => {
                let action = self.backend.handle(cmd);
                self.backend.sync();
                self.redraw();
                return action;
            },
            Command::ProxyToBackend(cmd) => {
                action = self.backend.handle(cmd);
            },
        };

        self.sync_and_redraw();
        action
    }

    fn sync_and_redraw(&mut self) {
        self.sync_font();
        self.backend.sync();
        self.redraw();
    }

    fn sync_font(&mut self) {
        self.font.sync();
        self.backend
            .handle(backend::Command::Resize(None, Some(self.font.measure)));
    }

    fn redraw(&mut self) {
        self.cache.clear();
    }
}

#[derive(Clone)]
struct TerminalSubscriptionData {
    id: u64,
    event_receiver: Arc<Mutex<Receiver<AlacrittyEvent>>>,
}

impl Hash for TerminalSubscriptionData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn terminal_subscription_stream(
    data: &TerminalSubscriptionData,
) -> BoxStream<'static, Event> {
    let id = data.id;
    let event_receiver = data.event_receiver.clone();
    iced::stream::channel(1000, async move |mut output| {
        loop {
            let event = event_receiver.lock().await.recv().await;
            let Some(event) = event else {
                // Dropping the backend can close the channel without an Exit event.
                break;
            };
            let shutdown = matches!(event, AlacrittyEvent::Exit);
            if output
                .send(Event::BackendCall(
                    id,
                    backend::Command::ProcessAlacrittyEvent(event),
                ))
                .await
                .is_err()
            {
                break;
            }
            if shutdown {
                break;
            }
        }
    })
    .boxed()
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
