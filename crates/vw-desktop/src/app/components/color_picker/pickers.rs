//! 颜色选择器属性模块，负责颜色解析、格式转换和拾色控件渲染。

use iced::mouse;
use iced::widget::canvas::{Action, Event, Geometry, Program};
use iced::{Point, Rectangle, Theme};

use crate::app::Message;

use super::hsv::Hsv;

fn cursor_position_inclusive(cursor: mouse::Cursor, bounds: Rectangle) -> Option<Point> {
    let position = cursor.position()?;
    if position.x < bounds.x
        || position.y < bounds.y
        || position.x > bounds.x + bounds.width
        || position.y > bounds.y + bounds.height
    {
        return None;
    }

    Some(Point::new(
        (position.x - bounds.x).clamp(0.0, bounds.width),
        (position.y - bounds.y).clamp(0.0, bounds.height),
    ))
}

/// SaturationValuePicker 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct SaturationValuePicker {
    pub hsv: Hsv,
    pub on_change: Box<dyn Fn(Hsv) -> Message>,
}

#[derive(Default)]
/// SaturationValueState 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct SaturationValueState {
    pub is_dragging: bool,
}

impl Program<Message> for SaturationValuePicker {
    type State = SaturationValueState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let _ = (renderer, bounds);
        vec![]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position = cursor_position_inclusive(cursor, bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.is_dragging = true;
                let s = (cursor_position.x / bounds.width).clamp(0.0, 1.0);
                let v = 1.0 - (cursor_position.y / bounds.height).clamp(0.0, 1.0);

                let new_hsv = Hsv { s, v, ..self.hsv };
                return Some(Action::publish((self.on_change)(new_hsv)));
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging {
                    state.is_dragging = false;
                    return Some(Action::publish((self.on_change)(self.hsv)));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_dragging {
                    let s = (cursor_position.x / bounds.width).clamp(0.0, 1.0);
                    let v = 1.0 - (cursor_position.y / bounds.height).clamp(0.0, 1.0);

                    let new_hsv = Hsv { s, v, ..self.hsv };
                    return Some(Action::publish((self.on_change)(new_hsv)));
                }
            }
            _ => {}
        }

        None
    }
}

/// HuePicker 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct HuePicker {
    pub hsv: Hsv,
    pub on_change: Box<dyn Fn(Hsv) -> Message>,
}

#[derive(Default)]
/// HueState 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct HueState {
    pub is_dragging: bool,
}

impl Program<Message> for HuePicker {
    type State = HueState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let _ = (renderer, bounds, self.hsv);
        vec![]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position = cursor_position_inclusive(cursor, bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.is_dragging = true;
                let h = (cursor_position.x / bounds.width).clamp(0.0, 1.0) * 360.0;

                let new_hsv = Hsv { h, ..self.hsv };
                return Some(Action::publish((self.on_change)(new_hsv)));
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging {
                    state.is_dragging = false;
                    return Some(Action::publish((self.on_change)(self.hsv)));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_dragging {
                    let h = (cursor_position.x / bounds.width).clamp(0.0, 1.0) * 360.0;

                    let new_hsv = Hsv { h, ..self.hsv };
                    return Some(Action::publish((self.on_change)(new_hsv)));
                }
            }
            _ => {}
        }

        None
    }
}

/// AlphaPicker 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct AlphaPicker {
    pub rgb: iced::Color,
    pub alpha: f32,
    pub on_change: Box<dyn Fn(f32) -> Message>,
}

#[derive(Default)]
/// AlphaState 状态结构，保存当前 UI 或导入流程需要跨消息传递的数据。
pub struct AlphaState {
    pub is_dragging: bool,
}

impl Program<Message> for AlphaPicker {
    type State = AlphaState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let _ = (renderer, bounds, self.alpha, self.rgb);
        vec![]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position = cursor_position_inclusive(cursor, bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.is_dragging = true;
                let v = (cursor_position.x / bounds.width).clamp(0.0, 1.0);
                Some(Action::publish((self.on_change)(v)))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging {
                    state.is_dragging = false;
                    Some(Action::publish((self.on_change)(self.alpha)))
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_dragging {
                    let v = (cursor_position.x / bounds.width).clamp(0.0, 1.0);
                    Some(Action::publish((self.on_change)(v)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "pickers_tests.rs"]
mod pickers_tests;
