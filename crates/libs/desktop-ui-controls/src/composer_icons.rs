use iced::widget::{Svg, svg};

#[derive(Clone, Copy)]
pub enum ComposerIcon {
    Add,
    Close,
    Attachment,
    Upload,
    Expert,
    Skill,
    Tools,
    Workflow,
    Queue,
    Connector,
    Permission,
    Microphone,
    Send,
    Agent,
}

macro_rules! outline {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</g></svg>"
        )
    };
}

impl ComposerIcon {
    pub fn svg(self, size: f32) -> Svg<'static> {
        svg(self.handle())
            .width(size)
            .height(size)
            .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().text) })
    }

    pub fn handle(self) -> svg::Handle {
        let data = match self {
            Self::Add => outline!(r#"<path d="M12 4.5v15m-7.5-7.5h15"/>"#),
            Self::Close => outline!(r#"<path d="m6 6 12 12M6 18 18 6"/>"#),
            Self::Attachment => outline!(
                r#"<path d="m8.5 13.4 6.8-6.8a2.3 2.3 0 0 1 3.2 3.3l-8.4 8.4a4.1 4.1 0 0 1-5.8-5.8l8.4-8.4a4.9 4.9 0 0 1 6.9 6.9l-7.7 7.7"/>"#
            ),
            Self::Upload => outline!(
                r#"<path d="M12 16V3.5m-4.5 4.5L12 3.5 16.5 8M4 15v4a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-4"/>"#
            ),
            Self::Expert => outline!(
                r#"<circle cx="10" cy="7.5" r="3.5"/><path d="M3 20v-1.5a5.5 5.5 0 0 1 5.5-5.5h3a5.2 5.2 0 0 1 2.8.8m4.2-.4 1.1 2.2 2.4.4-1.8 1.7.5 2.4-2.2-1.2-2.2 1.2.4-2.4-1.7-1.7 2.4-.4z"/>"#
            ),
            Self::Skill => outline!(
                r#"<path d="m3.8 17.8 9.8-9.8 3.4 3.4-9.8 9.8zm7.8-7.8 3.4 3.4M19 2.5v5m-2.5-2.5h5M7.5 3v4m-2-2h4m9 10v4m-2-2h4"/>"#
            ),
            Self::Tools => outline!(
                r#"<rect x="3" y="7" width="18" height="13" rx="2.5"/><path d="M8 7V5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M3 12h18m-11 0v2h4v-2"/>"#
            ),
            Self::Workflow => outline!(
                r#"<rect x="9" y="3" width="6" height="5" rx="1.2"/><rect x="3" y="16" width="6" height="5" rx="1.2"/><rect x="15" y="16" width="6" height="5" rx="1.2"/><path d="M12 8v4m-6 4v-2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v2"/>"#
            ),
            Self::Queue => outline!(
                r#"<path d="M8 5H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-3M8 12h5m-5 4h3m7-9v6m-3-3h6"/><rect x="8" y="3" width="6" height="4" rx="1.2"/>"#
            ),
            Self::Connector => outline!(
                r#"<path d="m8 10 6 6m-1-8 3-3m-1 5 3-3m-8 2-3 3a4 4 0 0 0 5.7 5.7l3-3M7.5 17 4 20.5"/>"#
            ),
            Self::Permission => outline!(
                r#"<path d="m12 3 7.5 3v5.8c0 4.2-2.7 7.2-7.5 9.2-4.8-2-7.5-5-7.5-9.2V6zM12 8v5m0 3v.1"/>"#
            ),
            Self::Microphone => outline!(
                r#"<rect x="8.5" y="3" width="7" height="12" rx="3.5"/><path d="M5 11v1a7 7 0 0 0 14 0v-1m-7 8v2.5m-3 0h6"/>"#
            ),
            Self::Send => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M10.8 3.7a1.4 1.4 0 0 1 2.4 0l8 15.2a1.35 1.35 0 0 1-1.8 1.8L12 17.2l-7.4 3.5a1.35 1.35 0 0 1-1.8-1.8z"/></svg>"#
            }
            Self::Agent => outline!(
                r#"<rect x="4" y="7" width="16" height="13" rx="4"/><path d="M12 7V4m-2 12h4M4 12H2m20 0h-2m-11 .2v.6m6-.6v.6"/>"#
            ),
        };
        svg::Handle::from_memory(data.as_bytes())
    }
}
