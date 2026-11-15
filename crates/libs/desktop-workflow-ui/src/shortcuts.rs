//! Workflow keyboard commands, independent of the desktop router.
use iced::Vector;

pub fn workflow_key_message(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    has_selected_node: bool,
) -> Option<crate::message::WorkflowMessage> {
    if modifiers.command() {
        return match key {
            iced::keyboard::Key::Character(c) if c == "+" || c == "=" => {
                Some(crate::message::WorkflowMessage::Zoom(1.1, None))
            }
            iced::keyboard::Key::Character(c) if c == "-" => {
                Some(crate::message::WorkflowMessage::Zoom(1.0 / 1.1, None))
            }
            iced::keyboard::Key::Character(c) if c == "0" => {
                Some(crate::message::WorkflowMessage::ZoomFit)
            }
            iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("o") => {
                Some(crate::message::WorkflowMessage::OpenFile)
            }
            iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                Some(crate::message::WorkflowMessage::Reload)
            }
            _ => None,
        };
    }

    match key {
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
            Some(crate::message::WorkflowMessage::PanBy(Vector::new(-48.0, 0.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
            Some(crate::message::WorkflowMessage::PanBy(Vector::new(48.0, 0.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
            Some(crate::message::WorkflowMessage::PanBy(Vector::new(0.0, -48.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
            Some(crate::message::WorkflowMessage::PanBy(Vector::new(0.0, 48.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
        | iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete) => {
            if has_selected_node {
                Some(crate::message::WorkflowMessage::DeleteSelectedNode)
            } else {
                Some(crate::message::WorkflowMessage::DeleteSelectedEdge)
            }
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
            Some(crate::message::WorkflowMessage::CancelInteraction)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
