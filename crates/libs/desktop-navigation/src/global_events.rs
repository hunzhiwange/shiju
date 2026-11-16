//! Native/Iced event interpretation; the shell maps the resulting cross-domain actions.
#[derive(Debug, Clone)]
pub enum Message {
    PointerMoved(f32, f32),
    HoveredFilePath(String),
    DroppedFilePath(String),
    HoveredFilesLeft,
    GlobalMouseReleased,
    GlobalCursorLeft,
    WindowResized(iced::window::Id, f32, f32),
    WindowMoved(iced::window::Id, f32, f32),
    CloseRequested(iced::window::Id),
    WindowClosed(iced::window::Id),
    WindowFocused(iced::window::Id),
    WindowUnfocused(iced::window::Id),
    FileTreeDragCancel,
    PreviewWindowUnfocused,
    GlobalKeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),
    ContextMenuClosed,
    FileTreeRenameCancel,
    CloseSearch,
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionClosed,
    #[cfg(not(target_arch = "wasm32"))]
    VoiceChatInterrupt,
    FileTreeFindCaseSensitiveShortcut,
    FileTreeFindWholeWordShortcut,
    FileTreeFindRegexShortcut,
    OpenReplace,
    OpenSearch,
    TraverseFormFocus(iced::window::Id, bool),
    Batch(Vec<Message>),
}
pub fn map_global_event(
    event: iced::Event,
    status: iced::event::Status,
    id: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::PointerMoved(position.x, position.y))
        }
        iced::Event::Window(iced::window::Event::FileHovered(path)) => {
            Some(Message::HoveredFilePath(path.to_string_lossy().to_string()))
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            if status == iced::event::Status::Captured {
                None
            } else {
                Some(Message::DroppedFilePath(path.to_string_lossy().into_owned()))
            }
        }
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
            Some(Message::HoveredFilesLeft)
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
            Some(Message::GlobalMouseReleased)
        }
        iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(Message::GlobalCursorLeft),
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(id, size.width, size.height))
        }
        iced::Event::Window(iced::window::Event::Moved(pos)) => {
            Some(Message::WindowMoved(id, pos.x, pos.y))
        }
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            Some(Message::CloseRequested(id))
        }
        iced::Event::Window(iced::window::Event::Closed) => Some(Message::WindowClosed(id)),
        iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(id)),
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Message::Batch(vec![
            Message::FileTreeDragCancel,
            Message::WindowUnfocused(id),
            Message::PreviewWindowUnfocused,
        ])),
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                let escape_key_message = Message::GlobalKeyPressed(key.clone(), modifiers);
                #[cfg(not(target_arch = "wasm32"))]
                let mut messages = vec![
                    Message::ContextMenuClosed,
                    Message::FileTreeDragCancel,
                    Message::FileTreeRenameCancel,
                    Message::CloseSearch,
                    escape_key_message,
                ];
                #[cfg(target_arch = "wasm32")]
                let messages = vec![
                    Message::ContextMenuClosed,
                    Message::FileTreeDragCancel,
                    Message::FileTreeRenameCancel,
                    Message::CloseSearch,
                    escape_key_message,
                ];
                #[cfg(not(target_arch = "wasm32"))]
                messages.push(Message::LspCompletionClosed);
                return Some(Message::Batch(messages));
            }

            #[cfg(not(target_arch = "wasm32"))]
            if modifiers.command()
                && modifiers.shift()
                && matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Space))
            {
                return Some(Message::VoiceChatInterrupt);
            }

            if modifiers.command() && modifiers.alt() && !modifiers.shift() {
                let shortcut = match &key {
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") => {
                        Some(Message::FileTreeFindCaseSensitiveShortcut)
                    }
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("w") => {
                        Some(Message::FileTreeFindWholeWordShortcut)
                    }
                    iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("x") => {
                        Some(Message::FileTreeFindRegexShortcut)
                    }
                    _ => None,
                };
                if let Some(shortcut) = shortcut {
                    return Some(shortcut);
                }
            }

            if modifiers.command()
                && matches!(key, iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("f"))
            {
                return Some(if modifiers.alt() {
                    Message::OpenReplace
                } else {
                    Message::OpenSearch
                });
            }

            if status == iced::event::Status::Captured {
                return None;
            }
            if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab))
                && (modifiers - iced::keyboard::Modifiers::SHIFT).is_empty()
            {
                return Some(Message::TraverseFormFocus(id, modifiers.shift()));
            }
            Some(Message::GlobalKeyPressed(key, modifiers))
        }
        _ => None,
    }
}
