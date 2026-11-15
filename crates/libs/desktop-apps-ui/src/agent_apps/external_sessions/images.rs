//! Images are fetched only from the authorized SDK, never from source paths or URLs.
use super::{Event as ParentEvent, message, reader};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;
use crate::{Connection, Notice, error as notify_error};
use iced::{
    Element, Length, Task,
    widget::{button, column, image, text},
};
use gateway_external_session::AgentAppsReadError;
use vw_api_external_session::*;

fn event(value: Event) -> Message {
    message(ParentEvent::Image(value))
}
pub(super) fn references(block: &SessionBlock) -> &[ImageRef] {
    match block {
        SessionBlock::Image { image } => std::slice::from_ref(image),
        SessionBlock::ToolResult { images, .. } => images,
        _ => &[],
    }
}
pub(super) fn update(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, value: Event) -> Task<Message> {
    let Some(scope) = reader::current_scope(state, connection) else {
        return Task::none();
    };
    let Some(reader) = &mut state.reader else {
        return Task::none();
    };
    match value {
        Event::Close => reader.images.enlarged = None,
        Event::Enlarge(reference) => {
            if reader.images.entries.iter().any(|e| e.reference == reference && e.handle.is_some())
            {
                reader.images.enlarged = Some(reference);
            }
        }
        Event::Load(reference) => {
            if !reader
                .messages
                .iter()
                .filter(|m| !m.injected)
                .flat_map(|m| &m.blocks)
                .any(|b| references(b).contains(&reference))
            {
                return Task::none();
            }
            let client = connection.client.clone();
            let index =
                reader.images.entries.iter().position(|e| e.reference == reference).unwrap_or_else(
                    || {
                        reader.images.entries.push(Entry {
                            reference: reference.clone(),
                            handle: None,
                            serial: 0,
                            pending: false,
                            abort: None,
                        });
                        reader.images.entries.len() - 1
                    },
                );
            let entry = &mut reader.images.entries[index];
            if entry.pending || entry.handle.is_some() {
                return Task::none();
            }
            let serial = rand::random();
            entry.serial = serial;
            entry.pending = true;
            let request =
                SessionImageRequest { identity: reader.identity.clone(), image: reference.clone() };
            let token = connection.token.clone();
            let task = Task::perform(
                async move {
                    let result = match gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_session_image(&request, &token).await {
                        Ok(response) => decode(response),
                        Err(error) => Err(error),
                    };
                    event(Event::Loaded(scope, serial, reference, result))
                },
                |m| m,
            );
            let (task, abort) = task.abortable();
            entry.abort = Some(abort.abort_on_drop());
            return task;
        }
        Event::Loaded(expected, serial, reference, result) => {
            if scope != expected {
                return Task::none();
            }
            let Some(entry) = reader
                .images
                .entries
                .iter_mut()
                .find(|e| e.reference == reference && e.serial == serial && e.pending)
            else {
                return Task::none();
            };
            entry.pending = false;
            entry.abort = None;
            match result {
                Ok(handle) => entry.handle = Some(handle),
                Err(_) => return notify_error(notice, "图片读取失败或格式不支持，可重试"),
            }
        }
    }
    Task::none()
}
fn decode(response: SessionImageResponse) -> Result<image::Handle, AgentAppsReadError> {
    let invalid = || AgentAppsReadError::Decode("unsupported or oversized bitmap".into());
    let format = match response.media_type.as_str() {
        "image/png" => ::image::ImageFormat::Png,
        "image/jpeg" => ::image::ImageFormat::Jpeg,
        "image/gif" => ::image::ImageFormat::Gif,
        "image/webp" => ::image::ImageFormat::WebP,
        "image/bmp" => ::image::ImageFormat::Bmp,
        _ => return Err(invalid()),
    };
    if response.bytes.len() > 20 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut decoder =
        ::image::ImageReader::with_format(std::io::Cursor::new(response.bytes), format);
    let mut limits = ::image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    decoder.limits(limits);
    let bitmap = decoder.decode().map_err(|_| invalid())?.into_rgba8();
    Ok(image::Handle::from_rgba(bitmap.width(), bitmap.height(), bitmap.into_raw()))
}
pub(super) fn view<'a>(reader: &'a reader::State, reference: &'a ImageRef) -> Element<'a, Message> {
    let entry = reader.images.entries.iter().find(|e| &e.reference == reference);
    let label = reference.alt.as_deref().unwrap_or("会话图片");
    if let Some(handle) = entry.and_then(|e| e.handle.as_ref()) {
        column![
            text(label),
            button(image(handle.clone()).height(160).content_fit(iced::ContentFit::Contain))
                .padding(8)
                .style(rounded_action_btn_style)
                .on_press(event(Event::Enlarge(reference.clone())))
        ]
        .spacing(6)
        .into()
    } else {
        button(text(if entry.is_some_and(|e| e.pending) {
            "正在读取图片…".to_owned()
        } else {
            format!("显示图片 · {label}")
        }))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            (!entry.is_some_and(|e| e.pending)).then(|| event(Event::Load(reference.clone()))),
        )
        .into()
    }
}
pub(super) fn overlay<'a>(
    base: Element<'a, Message>,
    reader: &'a reader::State,
) -> Element<'a, Message> {
    let handle = reader
        .images
        .enlarged
        .as_ref()
        .and_then(|r| reader.images.entries.iter().find(|e| &e.reference == r))
        .and_then(|e| e.handle.as_ref());
    if let Some(handle) = handle {
        let close = event(Event::Close);
        let card = settings_modal_card(
            column![
                text("图片预览").size(18),
                image(handle.clone())
                    .width(Length::Fill)
                    .height(480)
                    .content_fit(iced::ContentFit::Contain),
                button(text("关闭"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(close.clone())
            ]
            .spacing(12),
        )
        .width(800);
        settings_modal_overlay(Some(base), close, card)
    } else {
        base
    }
}

#[derive(Clone, Default)]
pub struct State {
    pub entries: Vec<Entry>,
    pub enlarged: Option<ImageRef>,
}
#[derive(Clone)]
pub struct Entry {
    pub reference: ImageRef,
    pub handle: Option<image::Handle>,
    pub serial: u64,
    pub pending: bool,
    pub abort: Option<iced::task::Handle>,
}
impl State {
    pub fn cancel(&self) {
        for entry in &self.entries {
            if let Some(abort) = &entry.abort {
                abort.abort();
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Load(ImageRef),
    Enlarge(ImageRef),
    Close,
    Loaded(reader::Scope, u64, ImageRef, Result<image::Handle, AgentAppsReadError>),
}
