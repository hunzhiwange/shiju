//! Minimal real-rendering acceptance fixture for the Web IME runtime patch.
use iced::widget::{column, text, text_editor, text_input};
use iced::{Element, Length};

#[derive(Default)]
struct Fixture {
    single: String,
    multi: text_editor::Content,
    password: String,
}

#[derive(Debug, Clone)]
enum Message {
    Single(String),
    Multi(text_editor::Action),
    Password(String),
}

impl Fixture {
    fn update(&mut self, message: Message) {
        match message {
            Message::Single(value) => self.single = value,
            Message::Multi(action) => self.multi.perform(action),
            Message::Password(value) => self.password = value,
        }
        // Acceptance fixture only. Never publish the password contents.
        #[cfg(target_arch = "wasm32")]
        if let Some(document) =
            web_sys::window().and_then(|window| window.document())
        {
            let snapshot = format!(
                "{}|{}|{}",
                self.single,
                self.multi.text().trim_end(),
                self.password.chars().count()
            );
            document.set_title(&snapshot);
            if let Some(root) = document.document_element() {
                let _ = root.set_attribute("data-ime-fixture", &snapshot);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("Single line"),
            text_input("Type here", &self.single).on_input(Message::Single),
            text("Multiple lines"),
            text_editor(&self.multi).on_action(Message::Multi).height(150),
            text("Password"),
            text_input("Password", &self.password)
                .secure(true)
                .on_input(Message::Password),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fixed(600.0))
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(Fixture::default, Fixture::update, Fixture::view)
        .title("Web IME fixture")
        .font(
            include_bytes!("../../../assets/fonts/NotoSansCJKsc-Regular.otf")
                .as_slice(),
        )
        .default_font(iced::Font::with_name("Noto Sans CJK SC"))
        .theme(iced::Theme::Dark)
        .run()
}
