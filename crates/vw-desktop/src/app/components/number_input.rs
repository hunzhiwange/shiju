//! 数字输入组件模块
//!
//! 本模块提供了一个可交互的数字输入控件，支持多种输入方式：
//! - **拖拽调节**：按住鼠标左键并拖动可按灵敏度调整数值
//! - **键盘输入**：点击后可通过键盘直接输入数值
//! - **方向键调节**：使用上下方向键按步进值调整
//! - **数值范围限制**：自动将数值限制在最小值和最大值之间
//! - **精度控制**：支持 0-4 位小数精度
//!
//! # 主要特性
//!
//! - 双模式交互（拖拽 vs 直接输入）
//! - 实时值变更回调
//! - 自动格式化显示
//! - 完整的光标和编辑支持
//!
//! # 使用示例
//!
//! ```rust,ignore
//! let input = NumberInput::new(
//!     50.0,           // 当前值
//!     0.0,            // 最小值
//!     100.0,          // 最大值
//!     1.0,            // 步进值
//!     2,              // 小数精度
//!     0.5,            // 拖拽灵敏度
//!     |v| Message::ValueChanged(v),
//! );
//! ```

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget};
use iced::widget::{container, mouse_area, row, text_input};
use iced::{Background, Border, Color, Element, Length, Rectangle, Size, Theme};

use crate::app::Message;

const PROP_INPUT_RADIUS: f32 = 8.0;

fn prop_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    let border_color = if focused { palette.primary } else { extended.background.strong.color };
    let bg = if focused { extended.background.weak.color } else { extended.background.base.color };
    text_input::Style {
        background: Background::Color(bg),
        border: iced::Border { width: 1.0, color: border_color, radius: PROP_INPUT_RADIUS.into() },
        icon: palette.text.scale_alpha(0.5),
        placeholder: palette.text.scale_alpha(0.55),
        value: palette.text,
        selection: palette.primary.scale_alpha(0.30),
    }
}

/// 数字输入组件
///
/// 一个支持拖拽和键盘输入的数值调节控件。该组件提供流畅的交互体验，
/// 允许用户通过鼠标拖拽或直接键盘输入来修改数值。
///
/// # 类型参数
///
/// * `'a` - 组件的生命周期，与回调函数的生命周期绑定
///
/// # 字段说明
///
/// * `value` - 当前数值
/// * `min` - 允许的最小值
/// * `max` - 允许的最大值
/// * `step` - 使用方向键调整时的步进值
/// * `precision` - 小数点后位数（0-4）
/// * `sensitivity` - 拖拽时的数值变化灵敏度
/// * `on_change` - 值变更时的回调函数
pub struct NumberInput<'a> {
    /// 当前数值
    value: f32,
    /// 允许的最小值
    min: f32,
    /// 允许的最大值
    max: f32,
    /// 方向键调整的步进值
    step: f32,
    /// 小数精度（0-4位）
    precision: u8,
    /// 拖拽灵敏度（每像素移动对应的数值变化）
    sensitivity: f32,
    /// 值变更回调函数
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
    /// 视觉样式变体
    style_variant: NumberInputStyleVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberInputStyleVariant {
    PropertyPanel,
    Settings,
}

impl NumberInputStyleVariant {
    fn height(self) -> f32 {
        match self {
            Self::PropertyPanel => 26.0,
            Self::Settings => 40.0,
        }
    }
}

impl<'a> NumberInput<'a> {
    /// 创建一个新的数字输入组件
    ///
    /// # 参数
    ///
    /// * `value` - 初始数值
    /// * `min` - 最小允许值
    /// * `max` - 最大允许值
    /// * `step` - 方向键调整时的步进值
    /// * `precision` - 小数点后位数（将被限制在 0-4 之间）
    /// * `sensitivity` - 拖拽灵敏度，值越大拖拽时数值变化越快
    /// * `on_change` - 数值变更时调用的回调函数
    ///
    /// # 返回值
    ///
    /// 返回一个配置好的 `NumberInput` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let input = NumberInput::new(
    ///     50.0,    // 初始值
    ///     0.0,     // 最小值
    ///     100.0,   // 最大值
    ///     1.0,     // 步进
    ///     2,       // 精度
    ///     0.5,     // 灵敏度
    ///     |v| Message::UpdateValue(v),
    /// );
    /// ```
    pub fn new(
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: u8,
        sensitivity: f32,
        on_change: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            value,
            min,
            max,
            step,
            // 将精度限制在 0-4 之间，防止过高的精度设置
            precision: precision.min(4),
            sensitivity,
            on_change: Box::new(on_change),
            style_variant: NumberInputStyleVariant::PropertyPanel,
        }
    }

    pub fn settings_style(mut self) -> Self {
        self.style_variant = NumberInputStyleVariant::Settings;
        self
    }
}

/// 组件内部状态
///
/// 用于跟踪数字输入组件的交互状态，包括拖拽、焦点和文本编辑状态。
/// 该状态在组件的整个生命周期中持久化保存。
///
/// # 状态说明
///
/// * `is_dragging` - 是否正在进行拖拽操作
/// * `is_focused` - 组件是否获得焦点
/// * `has_dragged` - 是否已经发生实际的拖拽移动（用于区分拖拽和点击）
/// * `drag_anchor_x` - 拖拽起始点的 X 坐标
/// * `drag_start_value` - 拖拽开始时的数值
/// * `edit_buffer` - 文本编辑缓冲区，用于键盘输入
/// * `caret` - 光标位置（以字符为单位）
#[derive(Default, Clone)]
struct State {
    /// 是否正在拖拽
    is_dragging: bool,
    /// 是否获得焦点
    is_focused: bool,
    /// 是否已经拖拽移动（用于区分点击和拖拽）
    has_dragged: bool,
    /// 拖拽起始点的 X 坐标
    drag_anchor_x: f32,
    /// 拖拽开始时的数值
    drag_start_value: f32,
    /// 文本编辑缓冲区（Some 表示正在编辑，None 表示非编辑模式）
    edit_buffer: Option<String>,
    /// 光标位置（字符索引）
    caret: usize,
}

/// 将数值限制在指定范围内
///
/// # 参数
///
/// * `v` - 要限制的数值
/// * `min` - 最小值
/// * `max` - 最大值
///
/// # 返回值
///
/// 返回限制在 `[min, max]` 范围内的数值
#[inline]
fn clamp(v: f32, min: f32, max: f32) -> f32 {
    v.max(min).min(max)
}

/// 按指定精度对数值进行四舍五入
///
/// # 参数
///
/// * `v` - 要舍入的数值
/// * `p` - 小数点后位数
///
/// # 返回值
///
/// 返回舍入后的数值
///
/// # 示例
///
/// ```rust,ignore
/// assert_eq!(round_precision(3.14159, 2), 3.14);
/// assert_eq!(round_precision(3.14159, 0), 3.0);
/// ```
#[inline]
fn round_precision(v: f32, p: u8) -> f32 {
    let factor = 10f32.powi(p as i32);
    (v * factor).round() / factor
}

/// 格式化数值为字符串显示
///
/// 根据指定的精度将数值格式化为字符串。该函数会移除尾随的零和小数点，
/// 以提供更简洁的显示。
///
/// # 参数
///
/// * `v` - 要格式化的数值
/// * `p` - 小数精度
///
/// # 返回值
///
/// 返回格式化后的字符串
///
/// # 示例
///
/// ```rust,ignore
/// assert_eq!(fmt_value(3.14, 2), "3.14");
/// assert_eq!(fmt_value(3.0, 2), "3");
/// assert_eq!(fmt_value(100.0, 0), "100");
/// ```
fn fmt_value(v: f32, p: u8) -> String {
    let r = round_precision(v, p);
    if p == 0 {
        // 精度为 0 时显示为整数
        format!("{}", r as i32)
    } else {
        // 先格式化为 4 位小数，然后移除尾随的零和小数点
        let s = format!("{:.4}", r);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

fn settings_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = palette.background.r + palette.background.g + palette.background.b < 1.5;
    let is_focused = matches!(status, text_input::Status::Focused { .. });
    let is_hovered = matches!(status, text_input::Status::Hovered)
        || matches!(status, text_input::Status::Focused { is_hovered: true });
    let is_disabled = matches!(status, text_input::Status::Disabled);

    let background = if is_disabled {
        extended.background.base.color.scale_alpha(if is_dark { 0.40 } else { 0.60 })
    } else if is_focused {
        if is_dark {
            extended.background.weak.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.98)
        }
    } else if is_hovered {
        if is_dark {
            extended.background.base.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_dark {
        extended.background.base.color.scale_alpha(0.84)
    } else {
        Color::WHITE.scale_alpha(0.90)
    };

    let border_color = if is_focused {
        palette.primary.scale_alpha(0.84)
    } else if is_hovered {
        extended.background.strong.color.scale_alpha(0.92)
    } else if is_dark {
        extended.background.strong.color.scale_alpha(0.82)
    } else {
        Color::from_rgba8(15, 23, 42, 0.10)
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.50),
        value: if is_disabled { palette.text.scale_alpha(0.55) } else { palette.text },
        selection: palette.primary.scale_alpha(0.20),
    }
}

impl<'a, ThemeT, RendererT> Widget<Message, ThemeT, RendererT> for NumberInput<'a>
where
    RendererT: iced::advanced::Renderer + iced::advanced::text::Renderer,
    ThemeT: iced::widget::text_input::Catalog,
{
    /// 返回状态类型标识
    ///
    /// 用于 iced 框架识别和管理工作组件的状态类型。
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    /// 创建并返回组件的初始状态
    ///
    /// 初始化一个新的 `State` 实例作为组件的内部状态。
    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    /// 返回组件的尺寸约束
    ///
    /// 定义组件的宽度和高度约束：
    /// - 宽度：填充可用空间
    /// - 高度：固定为 26.0 像素
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fixed(self.style_variant.height()) }
    }

    /// 计算组件的布局
    ///
    /// 根据父容器提供的限制条件计算组件的布局节点。
    ///
    /// # 参数
    ///
    /// * `_tree` - 组件树（未使用）
    /// * `_renderer` - 渲染器（未使用）
    /// * `limits` - 布局限制条件
    ///
    /// # 返回值
    ///
    /// 返回包含计算后尺寸的布局节点
    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = self.style_variant.height();
        let size = limits.resolve(Length::Fill, Length::Fixed(height), Size::new(0.0, height));
        layout::Node::new(size)
    }

    /// 绘制组件
    ///
    /// 该组件的渲染通过 `From<NumberInput>` trait 实现的 Element 组合处理，
    /// 因此此方法为空操作。
    fn draw(
        &self,
        _tree: &widget::Tree,
        _renderer: &mut RendererT,
        _theme: &ThemeT,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // 渲染通过 From<NumberInput> trait 实现的 Element 组合处理，因此此方法为空操作
    }

    /// 处理用户交互事件
    ///
    /// 该方法是组件的核心交互逻辑处理器，负责响应和处理所有用户输入事件，
    /// 包括鼠标点击、拖拽、移动以及键盘输入等。
    ///
    /// # 交互模式
    ///
    /// 1. **拖拽模式**：在组件区域按下鼠标并移动，根据水平移动距离调整数值
    /// 2. **编辑模式**：点击组件但不移动，进入文本编辑状态，可直接输入数值
    /// 3. **键盘快捷键**：
    ///    - Enter：确认输入
    ///    - Escape：取消编辑
    ///    - 上下方向键：按步进值调整
    ///    - 左右方向键：移动光标
    ///    - Home/End：移动到开头/结尾
    ///    - Backspace/Delete：删除字符
    ///
    /// # 参数
    ///
    /// * `tree` - 组件树，包含组件状态
    /// * `event` - 要处理的事件
    /// * `layout` - 组件的布局信息
    /// * `cursor` - 鼠标光标状态
    /// * `_renderer` - 渲染器（未使用）
    /// * `_clipboard` - 剪贴板（未使用）
    /// * `shell` - 消息外壳，用于发布消息
    /// * `_viewport` - 视口区域（未使用）
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &RendererT,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            // 处理鼠标左键按下事件
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(pos) = cursor.position() {
                    if bounds.contains(pos) {
                        // 鼠标在组件区域内
                        state.is_focused = true;
                        if state.edit_buffer.is_none() {
                            // 不在编辑模式时，进入拖拽准备状态
                            state.is_dragging = true;
                            state.has_dragged = false;
                            state.drag_anchor_x = pos.x;
                            state.drag_start_value = self.value;
                        }
                    } else {
                        // 鼠标在组件区域外，失去焦点
                        state.is_focused = false;
                        state.is_dragging = false;
                        // 如果有未提交的编辑内容，解析并提交
                        if let Some(buffer) = state.edit_buffer.take()
                            && let Ok(v) = buffer.parse::<f32>()
                        {
                            let nv = clamp(v, self.min, self.max);
                            shell.publish((self.on_change)(nv));
                        }
                    }
                } else {
                    // 无法获取鼠标位置，清除焦点和编辑状态
                    state.is_focused = false;
                    state.edit_buffer = None;
                }
            }

            // 处理鼠标按钮释放事件
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                let was_dragging = state.is_dragging;
                state.is_dragging = false;

                // 如果之前在拖拽状态但没有实际移动，则切换到编辑模式
                if was_dragging && !state.has_dragged {
                    let s = fmt_value(self.value, self.precision);
                    state.caret = s.len();
                    state.edit_buffer = Some(s);
                }
            }

            // 处理鼠标移动事件
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                if state.is_dragging {
                    let dx = position.x - state.drag_anchor_x;
                    // 只有移动距离超过阈值才开始拖拽（避免误触发）
                    if dx.abs() > 2.0 {
                        state.has_dragged = true;
                        // 根据移动距离和灵敏度计算新值
                        let mut nv = state.drag_start_value + dx * self.sensitivity;
                        nv = clamp(nv, self.min, self.max);
                        nv = round_precision(nv, self.precision);

                        // 清除编辑缓冲区
                        state.edit_buffer = None;

                        // 只有值实际改变时才发布消息
                        if (nv - self.value).abs() > f32::EPSILON {
                            shell.publish((self.on_change)(nv));
                        }
                    }
                }
            }

            // 处理键盘按键事件
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, text, .. }) => {
                if state.is_focused {
                    match key {
                        // Enter 键：确认编辑并提交值
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                            if let Some(buffer) = state.edit_buffer.take()
                                && let Ok(v) = buffer.parse::<f32>()
                            {
                                let nv = clamp(v, self.min, self.max);
                                shell.publish((self.on_change)(nv));
                            }
                            state.is_focused = false;
                        }

                        // Escape 键：取消编辑
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                            state.edit_buffer = None;
                            state.is_focused = false;
                        }

                        // 上方向键：增加值
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            let mut nv = self.value + self.step;
                            nv = clamp(nv, self.min, self.max);
                            nv = round_precision(nv, self.precision);
                            shell.publish((self.on_change)(nv));
                            state.edit_buffer = None;
                        }

                        // 下方向键：减少值
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            let mut nv = self.value - self.step;
                            nv = clamp(nv, self.min, self.max);
                            nv = round_precision(nv, self.precision);
                            shell.publish((self.on_change)(nv));
                            state.edit_buffer = None;
                        }

                        // Backspace 键：删除光标前的字符
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace) => {
                            if let Some(buffer) = &mut state.edit_buffer {
                                // 在编辑缓冲区中删除
                                if state.caret > 0 && state.caret <= buffer.len() {
                                    buffer.remove(state.caret - 1);
                                    state.caret -= 1;
                                }
                            } else {
                                // 不在编辑模式时，从当前值开始编辑并删除最后一个字符
                                let mut s = fmt_value(self.value, self.precision);
                                state.caret = s.len();
                                if state.caret > 0 {
                                    s.pop();
                                    state.caret -= 1;
                                }
                                state.edit_buffer = Some(s);
                            }
                        }

                        // Delete 键：删除光标处的字符
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete) => {
                            if let Some(buffer) = &mut state.edit_buffer {
                                // 在编辑缓冲区中删除光标处的字符
                                if state.caret < buffer.len() {
                                    buffer.remove(state.caret);
                                }
                            } else {
                                // 不在编辑模式时，进入编辑模式并清空内容
                                let s = fmt_value(self.value, self.precision);
                                state.caret = 0;
                                state.edit_buffer = Some(s);
                            }
                        }

                        // 左方向键：向左移动光标
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                            if state.caret > 0 {
                                state.caret -= 1;
                            }
                        }

                        // 右方向键：向右移动光标
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                            let len = state
                                .edit_buffer
                                .as_ref()
                                .map(|s| s.len())
                                .unwrap_or_else(|| fmt_value(self.value, self.precision).len());
                            if state.caret < len {
                                state.caret += 1;
                            }
                        }

                        // Home 键：移动光标到开头
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Home) => {
                            state.caret = 0;
                        }

                        // End 键：移动光标到结尾
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::End) => {
                            state.caret = state
                                .edit_buffer
                                .as_ref()
                                .map(|s| s.len())
                                .unwrap_or_else(|| fmt_value(self.value, self.precision).len());
                        }

                        // 字符输入：插入数字、小数点或负号
                        iced::keyboard::Key::Character(s) => {
                            for c in s.chars() {
                                // 只接受数字、小数点和负号
                                if c.is_ascii_digit() || c == '.' || c == '-' {
                                    if state.edit_buffer.is_none() {
                                        // 首次输入时创建新的编辑缓冲区
                                        state.edit_buffer = Some(c.to_string());
                                        state.caret = 1;
                                    } else {
                                        // 在光标位置插入字符
                                        let buffer = state.edit_buffer.as_mut().unwrap();
                                        let pos = state.caret.min(buffer.len());
                                        buffer.insert(pos, c);
                                        state.caret = pos + 1;
                                    }
                                }
                            }
                        }

                        // 处理其他键盘输入（如输入法输入）
                        _ => {
                            if let Some(t) = text {
                                for c in t.chars() {
                                    // 只接受数字、小数点和负号
                                    if c.is_ascii_digit() || c == '.' || c == '-' {
                                        if state.edit_buffer.is_none() {
                                            state.edit_buffer = Some(c.to_string());
                                            state.caret = 1;
                                        } else {
                                            let buffer = state.edit_buffer.as_mut().unwrap();
                                            let pos = state.caret.min(buffer.len());
                                            buffer.insert(pos, c);
                                            state.caret = pos + 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// 返回鼠标交互状态
    ///
    /// 定义鼠标悬停在组件上时的交互样式。当前返回空闲状态，
    /// 因为组件主要通过 From trait 实现来处理视觉反馈。
    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &RendererT,
    ) -> mouse::Interaction {
        mouse::Interaction::Idle
    }
}

/// 将 NumberInput 转换为 Element
///
/// 该实现将 NumberInput 组件转换为一个可渲染的 Element。它通过组合
/// iced 的基础组件（text_input、container、mouse_area）来构建最终的 UI。
///
/// # 转换逻辑
///
/// 1. 创建一个文本输入框显示当前值
/// 2. 用鼠标区域包装输入框以拦截鼠标事件
/// 3. 用容器包装以提供背景和边框样式
///
/// # 参数
///
/// * `widget` - 要转换的 NumberInput 组件
///
/// # 返回值
///
/// 返回一个组合了文本输入、鼠标交互和容器样式的 Element
impl<'a> From<NumberInput<'a>> for Element<'a, Message> {
    fn from(widget: NumberInput<'a>) -> Self {
        // 提取组件字段
        let value = widget.value;
        let precision = widget.precision;
        let min = widget.min;
        let max = widget.max;
        let _step = widget.step;
        let _sensitivity = widget.sensitivity;
        let on_change = widget.on_change;
        let style_variant = widget.style_variant;

        // 格式化显示值
        let display = fmt_value(value, precision);

        // 创建文本输入框
        let input_padding = match style_variant {
            NumberInputStyleVariant::PropertyPanel => iced::Padding::from(0),
            NumberInputStyleVariant::Settings => {
                iced::Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 14.0 }
            }
        };

        let input = text_input("", &display)
            .on_input(move |s| {
                // 解析用户输入并限制在有效范围内
                let v = s.parse::<f32>().unwrap_or(value);
                let v = clamp(v, min, max);
                (on_change)(round_precision(v, precision))
            })
            .padding(input_padding)
            .size(match style_variant {
                NumberInputStyleVariant::PropertyPanel => 12,
                NumberInputStyleVariant::Settings => 14,
            })
            .width(Length::Fill)
            .style(match style_variant {
                NumberInputStyleVariant::PropertyPanel => prop_text_input_style,
                NumberInputStyleVariant::Settings => settings_text_input_style,
            });

        // 用鼠标区域包装输入框以拦截鼠标事件
        let overlay = mouse_area(
            container(row![input].spacing(6))
                .width(Length::Fill)
                .height(Length::Fixed(style_variant.height()))
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_move(move |_pos| Message::None)
        .on_press(Message::None)
        .on_release(Message::None);

        let wrapper =
            container(overlay).width(Length::Fill).height(Length::Fixed(style_variant.height()));

        match style_variant {
            NumberInputStyleVariant::PropertyPanel => wrapper
                .style(|theme: &Theme| {
                    let p = theme.palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.background)),
                        border: Border {
                            width: 1.0,
                            color: p.background,
                            radius: PROP_INPUT_RADIUS.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            NumberInputStyleVariant::Settings => wrapper.into(),
        }
    }
}

#[cfg(test)]
#[path = "number_input_tests.rs"]
mod number_input_tests;
