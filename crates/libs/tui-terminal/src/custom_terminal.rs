// 本文件派生自 `ratatui::Terminal`，下方完整保留其 MIT 许可；VibeWindow 适配继续使用
// 双缓冲终端，并在不依赖 Codex 运行时的前提下桥接 Ratatui 颜色到固定 Crossterm 后端。
// This is derived from `ratatui::Terminal`, which is licensed under the following terms:
//
// The MIT License (MIT)
// Copyright (c) 2016-2022 Florian Dehau
// Copyright (c) 2023-2025 The Ratatui Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// VibeWindow adaptation: preserve the copied double-buffer terminal while bridging Ratatui
// colors to the pinned Crossterm backend without Codex runtime dependencies.
use std::io;
use std::io::Write;

use crossterm::cursor::MoveTo;
use crossterm::cursor::SetCursorStyle;
use crossterm::queue;
use crossterm::style::Colors;
use crossterm::style::Print;
use crossterm::style::SetAttribute;
use crossterm::style::SetBackgroundColor;
use crossterm::style::SetColors;
use crossterm::style::SetForegroundColor;
use crossterm::terminal::Clear;
use ratatui::backend::Backend;
use ratatui::backend::ClearType;
use ratatui::buffer::Buffer;
use ratatui::buffer::CellDiffOption;
use ratatui::buffer::CellWidth;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::widgets::WidgetRef;

pub fn crossterm_color(color: Color) -> crossterm::style::Color {
    use crossterm::style::Color as CrosstermColor;
    match color {
        Color::Reset => CrosstermColor::Reset,
        Color::Black => CrosstermColor::Black,
        Color::Red => CrosstermColor::DarkRed,
        Color::Green => CrosstermColor::DarkGreen,
        Color::Yellow => CrosstermColor::DarkYellow,
        Color::Blue => CrosstermColor::DarkBlue,
        Color::Magenta => CrosstermColor::DarkMagenta,
        Color::Cyan => CrosstermColor::DarkCyan,
        Color::Gray => CrosstermColor::Grey,
        Color::DarkGray => CrosstermColor::DarkGrey,
        Color::LightRed => CrosstermColor::Red,
        Color::LightGreen => CrosstermColor::Green,
        Color::LightYellow => CrosstermColor::Yellow,
        Color::LightBlue => CrosstermColor::Blue,
        Color::LightMagenta => CrosstermColor::Magenta,
        Color::LightCyan => CrosstermColor::Cyan,
        Color::White => CrosstermColor::White,
        Color::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
        Color::Indexed(index) => CrosstermColor::AnsiValue(index),
    }
}

fn osc8_hyperlink_parts(symbol: &str) -> Option<(&str, &str)> {
    let content = symbol.strip_prefix("\x1b]8;;")?;
    let destination_end = content.find('\x07')?;
    let destination = &content[..destination_end];
    if destination.is_empty() {
        return None;
    }
    let visible = content[destination_end + 1..].strip_suffix("\x1b]8;;\x07")?;
    Some((destination, visible))
}

pub struct Frame<'a> {
    /// 本帧绘制后光标所在位置；`None` 隐藏光标，`Some` 则显示并移动到指定坐标。
    /// Where should the cursor be after drawing this frame?
    ///
    /// If `None`, the cursor is hidden and its position is controlled by the backend. If `Some((x,
    /// y))`, the cursor is shown and placed at `(x, y)` after the call to `Terminal::draw()`.
    pub(crate) cursor_position: Option<Position>,

    /// 本帧绘制后应用的可见光标形状。
    /// Visible cursor shape to apply after drawing this frame.
    cursor_style: SetCursorStyle,

    /// 视口区域。
    /// The area of the viewport
    pub(crate) viewport_area: Rect,

    /// 用于绘制当前帧的缓冲区。
    /// The buffer that is used to draw the current frame
    pub(crate) buffer: &'a mut Buffer,
}

impl Frame<'_> {
    /// 当前帧的固定区域；渲染期间不会变化，收到后端缩放事件时也应使用此值计算本帧布局。
    /// The area of the current frame
    ///
    /// This is guaranteed not to change during rendering, so may be called multiple times.
    ///
    /// If your app listens for a resize event from the backend, it should ignore the values from
    /// the event for any calculations that are used to render the current frame and use this value
    /// instead as this is the area of the buffer that is used to render the current frame.
    pub const fn area(&self) -> Rect {
        self.viewport_area
    }

    /// 使用 `WidgetRef::render_ref` 把 `WidgetRef` 渲染到当前缓冲区。
    /// Render a [`WidgetRef`] to the current buffer using [`WidgetRef::render_ref`].
    ///
    /// Usually the area argument is the size of the current frame or a sub-area of the current
    /// frame (which can be obtained using [`Layout`] to split the total area).
    #[allow(clippy::needless_pass_by_value)]
    pub fn render_widget_ref<W: WidgetRef>(&mut self, widget: W, area: Rect) {
        widget.render_ref(area, self.buffer);
    }

    /// 本帧后显示光标并放到指定坐标；未调用则隐藏。不要与 `Terminal` 的光标控制 API 混用。
    /// After drawing this frame, make the cursor visible and put it at the specified (x, y)
    /// coordinates. If this method is not called, the cursor will be hidden.
    ///
    /// Note that this will interfere with calls to [`Terminal::hide_cursor`],
    /// [`Terminal::show_cursor`], and [`Terminal::set_cursor_position`]. Pick one of the APIs and
    /// stick with it.
    ///
    /// [`Terminal::hide_cursor`]: crate::Terminal::hide_cursor
    /// [`Terminal::show_cursor`]: crate::Terminal::show_cursor
    /// [`Terminal::set_cursor_position`]: crate::Terminal::set_cursor_position
    pub fn set_cursor_position<P: Into<Position>>(&mut self, position: P) {
        self.cursor_position = Some(position.into());
    }

    /// 本帧绘制后设置终端可见光标样式。
    /// After drawing this frame, set the terminal's visible cursor style.
    pub fn set_cursor_style(&mut self, style: SetCursorStyle) {
        self.cursor_style = style;
    }

    /// 以可变引用取得本 `Frame` 的绘制缓冲区。
    /// Gets the buffer that this `Frame` draws into as a mutable reference.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.buffer
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Terminal<B>
where
    B: Backend<Error = io::Error> + Write,
{
    /// 与终端交互的后端。
    /// The backend used to interface with the terminal
    backend: B,
    /// 保存当前和上一次绘制结果，帧末比较两者，只把必要更新输出到终端。
    /// Holds the results of the current and previous draw calls. The two are compared at the end
    /// of each draw pass to output the necessary updates to the terminal
    buffers: [Buffer; 2],
    /// 双缓冲数组中当前缓冲区的索引。
    /// Index of the current buffer in the previous array
    current: usize,
    /// 光标当前是否隐藏。
    /// Whether the cursor is currently hidden
    pub hidden_cursor: bool,
    /// 当前视口区域。
    /// Area of the viewport
    pub viewport_area: Rect,
    /// 最近一次已知终端尺寸，用于判断内部缓冲区是否需要缩放。
    /// Last known size of the terminal. Used to detect if the internal buffers have to be resized.
    pub last_known_screen_size: Size,
    /// 最近一次已知光标位置，内联视口随终端缩放时用于确定新区域。
    /// Last known position of the cursor. Used to find the new area when the viewport is inlined
    /// and the terminal resized.
    pub last_known_cursor_pos: Position,
    /// 内联模式下渲染在视口上方的可见历史行数。
    /// Count of visible history rows rendered above the viewport in inline mode.
    visible_history_rows: u16,
    #[cfg(test)]
    screen_size_override: Option<Size>,
}

impl<B> Drop for Terminal<B>
where
    B: Backend<Error = io::Error>,
    B: Write,
{
    #[allow(clippy::print_stderr)]
    fn drop(&mut self) {
        // 尝试恢复用户的光标状态。
        // Attempt to restore the cursor state
        if let Err(err) = self.reset_cursor_style() {
            eprintln!("Failed to reset the cursor style: {err}");
        }

        if self.hidden_cursor
            && let Err(err) = self.show_cursor()
        {
            eprintln!("Failed to show the cursor: {err}");
        }
    }
}

impl<B> Terminal<B>
where
    B: Backend<Error = io::Error>,
    B: Write,
{
    /// 使用给定后端和终端选项创建 `Terminal`。
    /// Creates a new [`Terminal`] with the given [`Backend`] and [`TerminalOptions`].
    pub fn with_options(mut backend: B) -> io::Result<Self> {
        let screen_size = backend.size()?;
        let cursor_pos = backend.get_cursor_position().unwrap_or_else(|err| {
            // 部分 PTY 不响应 CPR（`ESC[6n`），此时使用安全默认位置继续启动 TUI。
            // Some PTYs do not answer CPR (`ESC[6n`); continue with a safe default instead
            // of failing TUI startup.
            tracing::warn!("failed to read initial cursor position; defaulting to origin: {err}");
            Position { x: 0, y: 0 }
        });
        Ok(Self::with_screen_size_and_cursor_position(backend, screen_size, cursor_pos))
    }

    /// 使用调用方提供的初始光标位置创建终端；仅当外部已完成有界探测并选择同一回退时使用，
    /// 因为过期或合成位置会改变内联视口锚点。
    /// Creates a new [`Terminal`] from a caller-provided initial cursor position.
    ///
    /// Startup code uses this when cursor probing has already happened outside the backend, for
    /// example through a bounded terminal probe. Supplying a stale or synthetic position changes
    /// the inline viewport anchor, so callers should only use this after they have chosen the same
    /// fallback they want the first render to honor.
    pub fn with_options_and_cursor_position(backend: B, cursor_pos: Position) -> io::Result<Self> {
        let screen_size = backend.size()?;
        Ok(Self::with_screen_size_and_cursor_position(backend, screen_size, cursor_pos))
    }

    fn with_screen_size_and_cursor_position(
        backend: B,
        screen_size: Size,
        cursor_pos: Position,
    ) -> Self {
        Self {
            backend,
            buffers: [Buffer::empty(Rect::ZERO), Buffer::empty(Rect::ZERO)],
            current: 0,
            hidden_cursor: false,
            viewport_area: Rect::new(
                /*x*/ 0,
                cursor_pos.y,
                /*width*/ 0,
                /*height*/ 0,
            ),
            last_known_screen_size: screen_size,
            last_known_cursor_pos: cursor_pos,
            visible_history_rows: 0,
            #[cfg(test)]
            screen_size_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_screen_size_and_cursor_position_for_test(
        backend: B,
        screen_size: Size,
        cursor_pos: Position,
    ) -> Self {
        let mut terminal =
            Self::with_screen_size_and_cursor_position(backend, screen_size, cursor_pos);
        terminal.screen_size_override = Some(screen_size);
        terminal
    }

    /// 获取为本次渲染提供一致终端状态视图的 `Frame`。
    /// Get a Frame object which provides a consistent view into the terminal state for rendering.
    pub fn get_frame(&mut self) -> Frame<'_> {
        Frame {
            cursor_position: None,
            cursor_style: SetCursorStyle::DefaultUserShape,
            viewport_area: self.viewport_area,
            buffer: self.current_buffer_mut(),
        }
    }

    /// 获取当前缓冲区的只读引用。
    /// Gets the current buffer as a reference.
    fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.current]
    }

    /// 获取当前缓冲区的可变引用。
    /// Gets the current buffer as a mutable reference.
    fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    /// 获取上一缓冲区的只读引用。
    /// Gets the previous buffer as a reference.
    fn previous_buffer(&self) -> &Buffer {
        &self.buffers[1 - self.current]
    }

    /// 获取上一缓冲区的可变引用。
    /// Gets the previous buffer as a mutable reference.
    fn previous_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[1 - self.current]
    }

    /// 获取终端后端的只读引用。
    /// Gets the backend
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// 获取终端后端的可变引用。
    /// Gets the backend as a mutable reference
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// 计算上一缓冲区与当前缓冲区的差异，并交给后端绘制。
    /// Obtains a difference between the previous and the current buffer and passes it to the
    /// current backend for drawing.
    pub fn flush(&mut self) -> io::Result<()> {
        let updates = diff_buffers(self.previous_buffer(), self.current_buffer());
        let last_put_command =
            updates.iter().rfind(|command| matches!(command, DrawCommand::Put { .. }));
        if let Some(&DrawCommand::Put { x, y, .. }) = last_put_command {
            self.last_known_cursor_pos = Position { x, y };
        }
        draw(&mut self.backend, updates.into_iter())
    }

    /// 调整终端内部缓冲区以匹配请求区域，并保存区域保证渲染一致；该操作会完整清屏。
    /// Updates the Terminal so that internal buffers match the requested area.
    ///
    /// Requested area will be saved to remain consistent when rendering. This leads to a full clear
    /// of the screen.
    pub fn resize(&mut self, screen_size: Size) -> io::Result<()> {
        self.last_known_screen_size = screen_size;
        Ok(())
    }

    /// 设置视口区域。
    /// Sets the viewport area.
    pub fn set_viewport_area(&mut self, area: Rect) {
        self.current_buffer_mut().resize(area);
        self.previous_buffer_mut().resize(area);
        self.viewport_area = area;
        self.visible_history_rows = self.visible_history_rows.min(area.top());
    }

    /// 查询后端尺寸，与上次尺寸不同时执行缩放。
    /// Queries the backend for size and resizes if it doesn't match the previous size.
    pub fn autoresize(&mut self) -> io::Result<()> {
        let screen_size = self.size()?;
        if screen_size != self.last_known_screen_size {
            self.resize(screen_size)?;
        }
        Ok(())
    }

    /// 向终端绘制单帧：必要时自动缩放、调用完整帧渲染回调、刷新差异并恢复回调指定的光标。
    /// 回调可能失败时应使用 `try_draw`；回调必须完整渲染整帧以维持双缓冲一致性。
    /// Draws a single frame to the terminal.
    ///
    /// Returns a [`CompletedFrame`] if successful, otherwise a [`std::io::Error`].
    ///
    /// If the render callback passed to this method can fail, use [`try_draw`] instead.
    ///
    /// Applications should call `draw` or [`try_draw`] in a loop to continuously render the
    /// terminal. These methods are the main entry points for drawing to the terminal.
    ///
    /// [`try_draw`]: Terminal::try_draw
    ///
    /// This method will:
    ///
    /// - autoresize the terminal if necessary
    /// - call the render callback, passing it a [`Frame`] reference to render to
    /// - flush the current internal state by copying the current buffer to the backend
    /// - move the cursor to the last known position if it was set during the rendering closure
    ///
    /// The render callback should fully render the entire frame when called, including areas that
    /// are unchanged from the previous frame. This is because each frame is compared to the
    /// previous frame to determine what has changed, and only the changes are written to the
    /// terminal. If the render callback does not fully render the frame, the terminal will not be
    /// in a consistent state.
    pub fn draw<F>(&mut self, render_callback: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        let screen_size = self.size()?;
        self.draw_with_size(screen_size, render_callback)
    }

    /// 使用调用方已取得的屏幕尺寸绘制单帧。
    /// Draws a single frame using a screen size already obtained by the caller.
    pub fn draw_with_size<F>(&mut self, screen_size: Size, render: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.try_draw_with_size(screen_size, |frame| {
            render(frame);
            io::Result::Ok(())
        })
    }

    /// 尝试绘制单帧，流程与 `draw` 相同，但渲染回调可返回能转为 I/O 错误的结果；
    /// 回调失败时终端不更新，成功时完成差异刷新和光标处理。
    /// Tries to draw a single frame to the terminal.
    ///
    /// Returns [`Result::Ok`] containing a [`CompletedFrame`] if successful, otherwise
    /// [`Result::Err`] containing the [`std::io::Error`] that caused the failure.
    ///
    /// This is the equivalent of [`Terminal::draw`] but the render callback is a function or
    /// closure that returns a `Result` instead of nothing.
    ///
    /// Applications should call `try_draw` or [`draw`] in a loop to continuously render the
    /// terminal. These methods are the main entry points for drawing to the terminal.
    ///
    /// [`draw`]: Terminal::draw
    ///
    /// This method will:
    ///
    /// - autoresize the terminal if necessary
    /// - call the render callback, passing it a [`Frame`] reference to render to
    /// - flush the current internal state by copying the current buffer to the backend
    /// - move the cursor to the last known position if it was set during the rendering closure
    /// - return a [`CompletedFrame`] with the current buffer and the area of the terminal
    ///
    /// The render callback passed to `try_draw` can return any [`Result`] with an error type that
    /// can be converted into an [`std::io::Error`] using the [`Into`] trait. This makes it possible
    /// to use the `?` operator to propagate errors that occur during rendering. If the render
    /// callback returns an error, the error will be returned from `try_draw` as an
    /// [`std::io::Error`] and the terminal will not be updated.
    ///
    /// The [`CompletedFrame`] returned by this method can be useful for debugging or testing
    /// purposes, but it is often not used in regular applicationss.
    ///
    /// The render callback should fully render the entire frame when called, including areas that
    /// are unchanged from the previous frame. This is because each frame is compared to the
    /// previous frame to determine what has changed, and only the changes are written to the
    /// terminal. If the render function does not fully render the frame, the terminal will not be
    /// in a consistent state.
    pub fn try_draw<F, E>(&mut self, render_callback: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame) -> Result<(), E>,
        E: Into<io::Error>,
    {
        let screen_size = self.size()?;
        self.try_draw_with_size(screen_size, render_callback)
    }

    fn try_draw_with_size<F, E>(&mut self, screen_size: Size, render_callback: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame) -> Result<(), E>,
        E: Into<io::Error>,
    {
        if screen_size != self.last_known_screen_size {
            self.resize(screen_size)?;
        }
        let mut frame = self.get_frame();

        render_callback(&mut frame).map_err(Into::into)?;

        // 必须先把帧刷新到 stdout 才能移动光标，同时 `Frame` 借用了缓冲区，
        // 因此先取出必要字段再丢弃 `Frame`。
        // We can't change the cursor position right away because we have to flush the frame to
        // stdout first. But we also can't keep the frame around, since it holds a &mut to
        // Buffer. Thus, we're taking the important data out of the Frame and dropping it.
        let cursor_position = frame.cursor_position;
        let cursor_style = frame.cursor_style;

        // 把差异绘制到标准输出后端。
        // Draw to stdout
        self.flush()?;

        match cursor_position {
            None => self.hide_cursor()?,
            Some(position) => {
                self.set_cursor_style(cursor_style)?;
                self.show_cursor()?;
                self.set_cursor_position(position)?;
            }
        }

        self.swap_buffers();

        Backend::flush(&mut self.backend)?;

        Ok(())
    }

    /// 隐藏光标。
    /// Hides the cursor.
    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.backend.hide_cursor()?;
        self.hidden_cursor = true;
        Ok(())
    }

    /// 显示光标。
    /// Shows the cursor.
    pub fn show_cursor(&mut self) -> io::Result<()> {
        self.backend.show_cursor()?;
        self.hidden_cursor = false;
        Ok(())
    }

    /// 设置可见终端光标样式。
    /// Sets the visible terminal cursor style.
    pub fn set_cursor_style(&mut self, style: SetCursorStyle) -> io::Result<()> {
        queue!(self.backend, style)
    }

    /// 恢复用户配置的终端光标样式。
    /// Restores the user-configured terminal cursor style.
    pub fn reset_cursor_style(&mut self) -> io::Result<()> {
        self.set_cursor_style(SetCursorStyle::DefaultUserShape)
    }

    /// 获取最近一次绘制完成后的当前光标位置。
    /// Gets the current cursor position.
    ///
    /// This is the position of the cursor after the last draw call.
    #[allow(dead_code)]
    pub fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.backend.get_cursor_position()
    }

    /// 设置光标位置。
    /// Sets the cursor position.
    pub fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        let position = position.into();
        self.backend.set_cursor_position(position)?;
        self.last_known_cursor_pos = position;
        Ok(())
    }

    /// 清除终端，并强制下一次绘制完整重画。
    /// Clear the terminal and force a full redraw on the next draw call.
    pub fn clear(&mut self) -> io::Result<()> {
        if self.viewport_area.is_empty() {
            return Ok(());
        }
        self.clear_after_position(self.viewport_area.as_position())
    }

    /// 从 `position` 清到可见屏幕末尾，并强制完整重画。
    /// Clear from `position` through the end of the visible screen and force a full redraw.
    pub fn clear_after_position(&mut self, position: Position) -> io::Result<()> {
        self.backend.set_cursor_position(position)?;
        self.backend.clear_region(ClearType::AfterCursor)?;
        // 重置后备缓冲区，确保下一次更新重画所有内容。
        // Reset the back buffer to make sure the next update will redraw everything.
        self.previous_buffer_mut().reset();
        Ok(())
    }

    /// 原始终端操作在 Ratatui 不知情时移动屏幕内容后，强制下一帧重画整个视口；
    /// 仅重置差异缓冲会让默认空格与旧单元相等，从而泄漏陈旧终端内容。
    /// Force the next draw pass to repaint the entire viewport after raw terminal
    /// operations move screen content outside ratatui's knowledge. Resetting the
    /// diff buffer alone would leave default-style spaces equal to their previous
    /// cells, allowing stale terminal content to show through those spaces.
    pub fn invalidate_viewport(&mut self) {
        let previous_buffer = self.previous_buffer_mut();
        previous_buffer.reset();
        for cell in &mut previous_buffer.content {
            cell.set_diff_option(CellDiffOption::AlwaysUpdate);
        }
    }

    /// 清除整个可见屏幕而不只是视口，并强制完整重画。
    /// Clear the entire visible screen (not just the viewport) and force a full redraw.
    pub fn clear_visible_screen(&mut self) -> io::Result<()> {
        let home = Position { x: 0, y: 0 };
        // 某些终端（尤其 Terminal.app）在 ED2 前后显式归位光标时更可靠，
        // 与常见的 `CSI 2J` 加 `CSI H` 清屏序列一致。
        // Some terminals (notably Terminal.app) behave more reliably if we pair ED2
        // with an explicit cursor-home before/after, matching the common `clear`
        // sequence (`CSI 2J` + `CSI H`).
        self.set_cursor_position(home)?;
        self.backend.clear_region(ClearType::All)?;
        self.set_cursor_position(home)?;
        std::io::Write::flush(&mut self.backend)?;
        self.visible_history_rows = 0;
        self.previous_buffer_mut().reset();
        Ok(())
    }

    /// 用显式 ANSI 序列硬重置回滚区和可见屏幕；部分终端将清除和清屏合并发送更可靠。
    /// Hard-reset scrollback + visible screen using an explicit ANSI sequence.
    ///
    /// Some terminals behave more reliably when purge + clear are emitted as a
    /// single ANSI sequence instead of separate backend commands.
    pub fn clear_scrollback_and_visible_screen_ansi(&mut self) -> io::Result<()> {
        if self.viewport_area.is_empty() {
            return Ok(());
        }

        // 依次重置滚动区域和样式、归位光标、清屏并清除回滚区，匹配常见 shell 行为。
        // Reset scroll region + style state, home cursor, clear screen, purge scrollback.
        // The order matches the common shell `clear && printf '\\e[3J'` behavior.
        write!(self.backend, "\x1b[r\x1b[0m\x1b[H\x1b[2J\x1b[3J\x1b[H")?;
        std::io::Write::flush(&mut self.backend)?;
        self.last_known_cursor_pos = Position { x: 0, y: 0 };
        self.visible_history_rows = 0;
        self.previous_buffer_mut().reset();
        Ok(())
    }

    pub fn note_history_rows_inserted(&mut self, inserted_rows: u16) {
        self.visible_history_rows =
            self.visible_history_rows.saturating_add(inserted_rows).min(self.viewport_area.top());
    }

    /// 清空非活动缓冲区并与当前缓冲区交换。
    /// Clears the inactive buffer and swaps it with the current buffer
    pub fn swap_buffers(&mut self) {
        self.previous_buffer_mut().reset();
        self.current = 1 - self.current;
    }

    /// 查询后端的真实尺寸。
    /// Queries the real size of the backend.
    pub fn size(&self) -> io::Result<Size> {
        #[cfg(test)]
        if let Some(size) = self.screen_size_override {
            return Ok(size);
        }
        self.backend.size()
    }
}

use ratatui::buffer::Cell;

#[derive(Debug)]
enum DrawCommand {
    Put { x: u16, y: u16, cell: Cell },
    ClearToEnd { x: u16, y: u16, bg: Color },
}

fn diff_buffers(a: &Buffer, b: &Buffer) -> Vec<DrawCommand> {
    let next_buffer = &b.content;

    let mut updates = vec![];
    let mut last_nonblank_columns = vec![0; a.area.height as usize];
    for y in 0..a.area.height {
        let row_start = y as usize * a.area.width as usize;
        let row_end = row_start + a.area.width as usize;
        let previous_row = &a.content[row_start..row_end];
        let row = &next_buffer[row_start..row_end];
        let bg = row.last().map(|cell| cell.bg).unwrap_or(Color::Reset);

        // 扫描行中最后一个仍需绘制的列：非空字形、背景不同、带修饰符、强制更新或宽字形覆盖均计入；
        // 其后尾部可用一次 `ClearToEnd` 清除，避免逐空格输出。
        // Scan the row to find the rightmost column that still matters: any non-space glyph,
        // any cell whose bg differs from the row’s trailing bg, any cell with modifiers,
        // or any cell explicitly marked for updating.
        // Multi-width glyphs extend that region through their full displayed width.
        // After that point the rest of the row can be cleared with a single ClearToEnd, a perf win
        // versus emitting multiple space Put commands.
        let mut last_nonblank_column = 0usize;
        let mut column = 0usize;
        while column < row.len() {
            let cell = &row[column];
            let width = usize::from(cell.cell_width());
            // 将 `AlwaysUpdate` 空白保留在可绘制前缀中，避免尾部过滤丢弃 Ratatui 明确请求的重画。
            // Keep AlwaysUpdate blanks in the drawable prefix; otherwise filtering the tail
            // would discard the repaint explicitly requested by Ratatui.
            if cell.symbol() != " "
                || cell.bg != bg
                || cell.modifier != Modifier::empty()
                || cell.diff_option == CellDiffOption::AlwaysUpdate
            {
                last_nonblank_column = column + (width.saturating_sub(1));
            }
            column += width.max(1); // treat zero-width symbols as width 1
        }

        let clear_start = last_nonblank_column + 1;
        if clear_start < row.len() {
            // 缓冲区反映真实终端时，相等缓存尾部无需清除；视口失效会标记旧单元，
            // 使带外写入强制产生差异。
            // Equal cached tails need no clear when the buffers reflect the terminal.
            // Viewport invalidation marks old cells, so out-of-band writes force inequality.
            let tail_changed = previous_row[clear_start..] != row[clear_start..];

            // 宽字形续单元看似空白，相等尾部仍可能与清除边界前的字形首单元重叠。
            // Wide-glyph continuation cells look blank, so an equal tail can still overlap
            // a glyph whose leader lies before the clear boundary.
            let wide_char_overlaps_tail = previous_row[..clear_start]
                .iter()
                .enumerate()
                .rev()
                .find(|(_, cell)| cell.symbol() != " " || cell.cell_width() > 1)
                .is_some_and(|(column, cell)| {
                    column + usize::from(cell.cell_width()) > clear_start
                });
            if tail_changed || wide_char_overlaps_tail {
                let (x, y) = a.pos_of(row_start + clear_start);
                updates.push(DrawCommand::ClearToEnd { x, y, bg });
            }
        }

        last_nonblank_columns[y as usize] = last_nonblank_column as u16;
    }

    // 保留 Ratatui 原生的 `Skip`、`AlwaysUpdate` 和多宽度差异语义。
    // Preserve Ratatui's native Skip, AlwaysUpdate, and multi-width diff semantics.
    let mut cell_updates = a.diff_iter(b).collect::<Vec<_>>();
    // Ratatui 的 `ForcedWidth` 路径在带样式宽单元收缩时会跳过尾随单元失效处理。
    // Ratatui's ForcedWidth path skips trailing-cell invalidation when a styled wide cell shrinks.
    let visible_on_blank = Modifier::REVERSED
        .union(Modifier::UNDERLINED)
        .union(Modifier::SLOW_BLINK)
        .union(Modifier::RAPID_BLINK)
        .union(Modifier::CROSSED_OUT);
    for (i, (current, previous)) in next_buffer.iter().zip(a.content.iter()).enumerate() {
        let CellDiffOption::ForcedWidth(current_width) = current.diff_option else {
            continue;
        };
        let current_width = usize::from(current_width.get());
        let previous_width = usize::from(previous.cell_width());
        if previous_width <= current_width
            || (previous.bg == Color::Reset && !previous.modifier.intersects(visible_on_blank))
        {
            continue;
        }

        for (index, cell) in next_buffer
            .iter()
            .enumerate()
            .skip(i + current_width)
            .take(previous_width - current_width)
        {
            #[allow(deprecated)]
            let is_skip = cell.diff_option == CellDiffOption::Skip
                || (cell.skip && cell.diff_option == CellDiffOption::None);
            if !is_skip {
                let (x, y) = a.pos_of(index);
                cell_updates.push((x, y, cell));
            }
        }
    }
    cell_updates.sort_unstable_by_key(|(x, y, _)| (*y, *x));
    cell_updates.dedup_by_key(|(x, y, _)| (*y, *x));

    for (x, y, cell) in cell_updates {
        let row = usize::from(y - a.area.y);
        if x <= last_nonblank_columns[row] {
            updates.push(DrawCommand::Put { x, y, cell: cell.clone() });
        }
    }
    updates
}

fn draw<I>(writer: &mut impl Write, commands: I) -> io::Result<()>
where
    I: Iterator<Item = DrawCommand>,
{
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;
    let mut modifier = Modifier::empty();
    let mut last_pos: Option<Position> = None;
    let mut active_hyperlink: Option<String> = None;
    for command in commands {
        let (x, y) = match &command {
            DrawCommand::Put { x, y, .. } => (x, y),
            DrawCommand::ClearToEnd { x, y, .. } => (x, y),
        };
        let hyperlink = match &command {
            DrawCommand::Put { cell, .. } => osc8_hyperlink_parts(cell.symbol()),
            DrawCommand::ClearToEnd { .. } => None,
        };
        let destination = hyperlink.map(|(destination, _)| destination);
        let hyperlink_changed = active_hyperlink.as_deref() != destination;
        if hyperlink_changed && active_hyperlink.is_some() {
            queue!(writer, Print("\x1b]8;;\x07"))?;
        }
        // 上次位置不是 `(x - 1, y)` 时才显式移动光标。
        // Move the cursor if the previous location was not (x - 1, y)
        if !matches!(last_pos, Some(p) if *x == p.x + 1 && *y == p.y) {
            queue!(writer, MoveTo(*x, *y))?;
        }
        last_pos = Some(Position { x: *x, y: *y });
        match &command {
            DrawCommand::Put { cell, .. } => {
                if cell.modifier != modifier {
                    let diff = ModifierDiff { from: modifier, to: cell.modifier };
                    diff.queue(writer)?;
                    modifier = cell.modifier;
                }
                if cell.fg != fg || cell.bg != bg {
                    queue!(
                        writer,
                        SetColors(Colors::new(crossterm_color(cell.fg), crossterm_color(cell.bg)))
                    )?;
                    fg = cell.fg;
                    bg = cell.bg;
                }

                if hyperlink_changed && let Some(destination) = destination {
                    queue!(writer, Print(format!("\x1b]8;;{destination}\x07")))?;
                }
                let symbol = hyperlink.map_or_else(|| cell.symbol(), |(_, visible)| visible);
                queue!(writer, Print(symbol))?;
            }
            DrawCommand::ClearToEnd { bg: clear_bg, .. } => {
                queue!(writer, SetAttribute(crossterm::style::Attribute::Reset))?;
                modifier = Modifier::empty();
                queue!(writer, SetBackgroundColor(crossterm_color(*clear_bg)))?;
                bg = *clear_bg;
                queue!(writer, Clear(crossterm::terminal::ClearType::UntilNewLine))?;
            }
        }
        if hyperlink_changed {
            active_hyperlink = destination.map(str::to_owned);
        }
    }
    if active_hyperlink.is_some() {
        queue!(writer, Print("\x1b]8;;\x07"))?;
    }

    queue!(
        writer,
        SetForegroundColor(crossterm::style::Color::Reset),
        SetBackgroundColor(crossterm::style::Color::Reset),
        SetAttribute(crossterm::style::Attribute::Reset),
    )?;

    Ok(())
}

/// `ModifierDiff` 计算两个 `Modifier` 值的差异，只发送必要的终端样式变化以提高更新效率。
/// The `ModifierDiff` struct is used to calculate the difference between two `Modifier`
/// values. This is useful when updating the terminal display, as it allows for more
/// efficient updates by only sending the necessary changes.
struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W: io::Write>(self, w: &mut W) -> io::Result<()> {
        use crossterm::style::Attribute as CAttribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::NoReverse))?;
        }
        if removed.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
            if self.to.contains(Modifier::DIM) {
                queue!(w, SetAttribute(CAttribute::Dim))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::NoItalic))?;
        }
        if removed.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::NoUnderline))?;
        }
        if removed.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::NotCrossedOut))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::NoBlink))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::Reverse))?;
        }
        if added.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::Bold))?;
        }
        if added.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::Italic))?;
        }
        if added.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::Underlined))?;
        }
        if added.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::Dim))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::CrossedOut))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            queue!(w, SetAttribute(CAttribute::SlowBlink))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::RapidBlink))?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "custom_terminal_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "custom_terminal_cross_layer_tests.rs"]
mod cross_layer_tests;
