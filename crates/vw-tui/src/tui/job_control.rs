//! Unix 作业控制下的 TUI 挂起、终端让渡与恢复对齐。
//!
//! 挂起前离开替代屏幕并记录恢复意图；`fg` 返回后在同步绘制中恢复替代屏幕或校正内联 viewport。

use std::io::Result;
use std::io::stdout;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

use crossterm::cursor::MoveTo;
use crossterm::cursor::Show;
use crossterm::event::KeyCode;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use ratatui::layout::Size;

use crate::key_hint;

use super::DisableAlternateScroll;
use super::EnableAlternateScroll;
use super::Terminal;

pub const SUSPEND_KEY: key_hint::KeyBinding = key_hint::ctrl(KeyCode::Char('z'));

/// Coordinates suspend/resume handling so the TUI can restore terminal context after SIGTSTP.
///
/// On suspend, it records which resume path to take (realign inline viewport vs. restore alt
/// screen) and caches the inline cursor row so the cursor can be placed meaningfully before
/// yielding.
///
/// After resume, `prepare_resume_action` consumes the pending intent and returns a
/// `PreparedResumeAction` describing any viewport adjustments to apply inside the synchronized
/// draw.
///
/// Callers keep `suspend_cursor_y` up to date during normal drawing so the suspend step always
/// has the latest cursor position.
///
/// The type is `Clone`, using Arc/atomic internals so bookkeeping can be shared across tasks
/// and moved into the boxed `'static` event stream without borrowing `self`.
/// 协调 SIGTSTP 挂起/恢复，使 TUI 能重建终端上下文。挂起时记录恢复路径与内联光标行，
/// 恢复时消费该意图并在同步绘制中应用 viewport 调整。内部使用 Arc/原子值以便跨任务共享。
#[derive(Clone)]
pub struct SuspendContext {
    /// Resume intent captured at suspend time; cleared once applied after resume.
    /// 挂起时捕获、恢复应用后清除的意图。
    resume_pending: Arc<Mutex<Option<ResumeAction>>>,
    /// Inline viewport cursor row used to place the cursor before yielding during suspend.
    /// 让渡终端前用于安置光标的内联 viewport 行。
    suspend_cursor_y: Arc<AtomicU16>,
}

impl SuspendContext {
    pub(crate) fn new() -> Self {
        Self {
            resume_pending: Arc::new(Mutex::new(None)),
            suspend_cursor_y: Arc::new(AtomicU16::new(0)),
        }
    }

    /// Capture how to resume, stash cursor position, and temporarily yield during SIGTSTP.
    ///
    /// - If the alt screen is active, exit alt-scroll/alt-screen and record `RestoreAlt`;
    ///   otherwise record `RealignInline`.
    /// - Update the cached inline cursor row so suspend can place the cursor meaningfully.
    /// - Trigger SIGTSTP so the process can be resumed and continue drawing with the saved state.
    /// 捕获恢复方式与光标位置：替代屏幕退出后记录恢复替代屏幕，否则记录内联重对齐，随后触发 SIGTSTP。
    pub(crate) fn suspend(&self, alt_screen_active: &Arc<AtomicBool>) -> Result<()> {
        if alt_screen_active.load(Ordering::Relaxed) {
            // Leave alt-screen so the terminal returns to the normal buffer while suspended; also turn off alt-scroll.
            // 挂起期间退出替代屏幕并关闭替代滚动，让 shell 回到正常缓冲区。
            let _ = execute!(stdout(), DisableAlternateScroll);
            let _ = execute!(stdout(), LeaveAlternateScreen);
            self.set_resume_action(ResumeAction::RestoreAlt);
        } else {
            self.set_resume_action(ResumeAction::RealignInline);
        }
        let y = self.suspend_cursor_y.load(Ordering::Relaxed);
        let _ = execute!(stdout(), MoveTo(0, y), Show);
        suspend_process()?;
        super::reapply_raw_mode_after_resume()?;

        // The shell writes its job-control status and the resumed command after `fg`, so the
        // cursor may no longer be on the row cached before suspending. The event stream remains
        // paused until this method returns, which makes it safe for the probe to consume both an
        // interleaved focus report and the cursor-position response without racing the background
        // input reader.
        // `fg` 会输出作业状态和恢复命令，光标可能已移动；事件流保持暂停，使探测可独占消费焦点报告与光标响应。
        match crate::terminal_probe::cursor_position(crate::terminal_probe::DEFAULT_TIMEOUT) {
            Ok(Some(position)) => self.set_cursor_y(position.y),
            Ok(None) => tracing::debug!("terminal cursor position unavailable after resume"),
            Err(err) => tracing::debug!(
                error = %err,
                "failed to read terminal cursor position after resume"
            ),
        }
        super::flush_terminal_input_buffer();
        tracing::trace!(
            event = "tui_suspend_resumed",
            cursor_y = self.cursor_y(),
            "restored terminal state after resume"
        );

        Ok(())
    }

    /// Consume the pending resume intent and precompute any viewport changes needed post-resume.
    ///
    /// Returns a `PreparedResumeAction` describing how to realign the viewport once drawing
    /// resumes; returns `None` when there was no pending suspend intent.
    /// 消费待处理恢复意图并预计算 viewport 调整；没有挂起意图时返回 `None`。
    pub(crate) fn prepare_resume_action(
        &self,
        alt_saved_viewport: &mut Option<Rect>,
    ) -> Option<PreparedResumeAction> {
        let action = self.take_resume_action()?;
        match action {
            ResumeAction::RealignInline => {
                let viewport = Rect::new(
                    /*x*/ 0,
                    self.cursor_y(),
                    /*width*/ 0,
                    /*height*/ 0,
                );
                Some(PreparedResumeAction::RealignViewport(viewport))
            }
            ResumeAction::RestoreAlt => {
                if let Some(saved) = alt_saved_viewport.as_mut() {
                    saved.y = self.cursor_y();
                }
                Some(PreparedResumeAction::RestoreAltScreen)
            }
        }
    }

    /// Set the cached inline cursor row so suspend can place the cursor meaningfully.
    ///
    /// Call during normal drawing when the inline viewport moves so suspend has a fresh cursor
    /// position to restore before yielding.
    /// 正常绘制时更新内联光标行，使挂起前始终能使用最新位置。
    pub(crate) fn set_cursor_y(&self, value: u16) {
        self.suspend_cursor_y.store(value, Ordering::Relaxed);
    }

    fn cursor_y(&self) -> u16 {
        self.suspend_cursor_y.load(Ordering::Relaxed)
    }

    /// Record a pending resume action to apply after SIGTSTP returns control.
    /// 记录 SIGTSTP 返回后需要执行的恢复动作。
    fn set_resume_action(&self, value: ResumeAction) {
        *self
            .resume_pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(value);
    }

    /// Take and clear any pending resume action captured at suspend time.
    /// 取出并清除挂起时捕获的恢复动作。
    fn take_resume_action(&self) -> Option<ResumeAction> {
        self.resume_pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Captures what should happen when returning from suspend.
///
/// Either realign the inline viewport to keep the cursor position, or re-enter the alt screen
/// to restore the overlay UI.
/// 描述挂起返回后的意图：保持内联光标锚点，或重新进入替代屏幕恢复覆盖层。
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResumeAction {
    /// Shift the inline viewport to keep the cursor anchored after resume.
    /// 平移内联 viewport，使恢复后的光标保持锚定。
    RealignInline,
    /// Re-enter the alt screen and restore the overlay UI.
    /// 重新进入替代屏幕并恢复覆盖界面。
    RestoreAlt,
}

/// Describes the viewport change to apply when resuming from suspend during the synchronized draw.
///
/// Either restore the alt screen (with viewport reset) or realign the inline viewport.
/// 同步恢复绘制中要应用的具体 viewport 变化：重置替代屏幕或校正内联 viewport。
#[derive(Clone, Debug)]
pub(crate) enum PreparedResumeAction {
    /// Re-enter the alt screen and reset the viewport to the terminal dimensions.
    /// 重新进入替代屏幕并把 viewport 重置为终端尺寸。
    RestoreAltScreen,
    /// Apply a viewport shift to keep the inline cursor position stable.
    /// 应用 viewport 位移以保持内联光标位置稳定。
    RealignViewport(Rect),
}

impl PreparedResumeAction {
    pub(crate) fn apply(self, terminal: &mut Terminal, screen_size: Size) -> Result<()> {
        match self {
            PreparedResumeAction::RealignViewport(area) => {
                terminal.set_viewport_area(area);
            }
            PreparedResumeAction::RestoreAltScreen => {
                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                // Enable "alternate scroll" so terminals may translate wheel to arrows
                // 启用替代滚动，让终端可把滚轮事件转换为方向键。
                execute!(terminal.backend_mut(), EnableAlternateScroll)?;
                terminal.set_viewport_area(Rect::from(screen_size));
                terminal.clear()?;
            }
        }
        Ok(())
    }
}

/// Deliver SIGTSTP after restoring terminal state, then re-applies terminal modes once resumed.
/// 恢复终端状态后发送 SIGTSTP，并在进程恢复后重新应用 TUI 模式。
fn suspend_process() -> Result<()> {
    super::restore()?;
    super::terminal_stderr::pause()?;
    unsafe {
        libc::kill(/*pid*/ 0, libc::SIGTSTP)
    };
    // After the process resumes, reapply terminal modes so drawing can continue.
    // 进程恢复后重新启用终端模式，使绘制可以继续。
    super::terminal_stderr::resume()?;
    super::set_modes()?;
    Ok(())
}
