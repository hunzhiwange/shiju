//! 为终端中未使用括号粘贴（bracketed paste）提供粘贴突发检测。 在某些平台上（例如 Windows），粘贴通常以快速流的形式到达，即 `KeyCode::Char` 和 `KeyCode::Enter` 键事件，而不是作为一个单一的“粘贴”事件。 在这种模式下，输入编辑器需要： - 防止瞬态 UI 副作用（例如绑定到 `?` 的切换）触发在已粘贴文本上。 - 确保 Enter 被视为粘贴内的换行符，而非“提交消息”。 - 避免因插入前缀字符后立即重新分类为粘贴而导致的闪烁。 本模块提供 `PasteBurst` 状态机。`ChatComposer` 仅接收文本产生字符事件（普通、Shift 或 Windows AltGr），并使用完整的缓冲决策： - 短暂保持第一个 ASCII 字符以抑制闪烁， - 将突发序列作为单个粘贴字符串存储， - 或将输入流作为正常打字处理。 # 调用模式 `PasteBurst` 是一个纯状态机：它不会直接修改文本框。调用者向其发送事件后，应用所选动作： - 对于每个文本产生 `KeyCode::Char`，调用 [`PasteBurst::on_plain_char`]（ASCII）或 [`PasteBurst::on_plain_char_no_hold`]（非 ASCII/IME）。 - 如果决策指示需要缓冲，调用者通过 [`PasteBurst::append_char_to_buffer`] 将字符追加到 `PasteBurst.buffer`。 - 在 UI 更新时，调用 [`PasteBurst::flush_if_due`]。如果返回 [`FlushResult::Typed`]，将该字符作为正常打字插入；如果返回 [`FlushResult::Paste`]，则将返回的字符串视为显式粘贴。 - 在处理非文本输入（方向键；除 Windows AltGr 外的 Ctrl/Alt 快捷键；或 Super、Hyper 和 Meta 修饰符）之前，使用 [`PasteBurst::flush_before_modified_input`] 避免将缓冲中的文本“卡住”，然后调用 [`PasteBurst::clear_window_after_non_char`]，确保后续打字不会与之前的突发序列合并。 - 直接插入调用者可以跳过缓冲，在 Enter 处理中调用 [`PasteBurst::direct_insert_newline_should_insert`]，并在 Enter 或 [`PasteBurst::on_plain_char_no_hold`] 报告类似突发流时调用 [`PasteBurst::extend_window`]。 # 状态变量 该状态机编码在几个字段中，含义略有不同： - `active`：表示我们仍在积极接受字符进入当前突发序列。 - `buffer`：累积的突发文本，最终将作为单个 `Paste(String)` 插入。即使 `active` 已清除，只要缓冲区非空，就视为处于“突发上下文”中。 - `pending_first_char`：用于抑制闪烁的单个 ASCII 字符。调用者必须在它成为突发序列的一部分（通过 [`BeginBufferFromPending`]）或作为正常打字字符（通过 [`FlushResult::Typed`]）时才能渲染该字符。 - `last_plain_char_time`/`consecutive_plain_char_burst`：用于“粘贴流”的时机计数和阈值。 - `burst_window_until`：Enter 抑制窗口（“Enter 插入换行符”），该窗口在缓冲区之外持续。 # 计时模型 有两个超时时间： - `PASTE_BURST_CHAR_INTERVAL`：连续“普通”字符之间的最大延迟，使其被视为突发序列的一部分。它也限制了 [`pending_first_char`] 被持有的时长。 - `PASTE_BURST_ACTIVE_IDLE_TIMEOUT`：一旦处于缓冲状态，最后一次字符之后多久等待将累积的缓冲区作为粘贴插入。 `flush_if_due()` 故意使用 `>`（而非 `>=`）来比较 elapsed time，因此测试和 UI 更新应至少跨越 1ms 阈值（见 [`recommended_flush_delay()`]）。 # 回溯捕获细节 回溯捕获用于处理初始以“正常打字”方式插入字符的情况，但后来决定该序列为粘贴流。当这种情况发生时，我们回滚已插入文本的前缀，并将其移动到突发缓冲区，从而使最终 `handle_paste(...)` 看到连续的粘贴字符串。 回溯捕获主要影响那些不持有第一个字符的路径（非 ASCII/IME 输入和回溯抓取场景）。ASCII 路径通常偏好 [`RetainFirstChar -> BeginBufferFromPending`]，从而无需使用回溯捕获。 回溯捕获以字符而非字节表示： - `CharDecision::BeginBuffer { retro_chars }` 使用 `retro_chars` 作为字符计数。 - `decide_begin_buffer(now, before_cursor, retro_chars)` 将这一行为转换为 UTF-8 字节范围，通过调用 [`retro_start_index()`]。 - `RetroGrab.start_byte` 是字节索引，指向 `before_cursor` 切片中的起始位置；调用者必须在切分前将光标定位到字符边界，否则 `start_byte..cursor` 始终有效 UTF-8。 # 清空 vs 刷新 有几种调用者结束突发处理，它们不是可互换的： - `flush_before_modified_input()` 返回缓冲文本（以及/或一个待处理的第一个 ASCII 字符），以便在应用正常粘贴路径之前处理无关输入。 - `clear_window_after_non_char()` 清空分类窗口，因此后续打字不会与之前的突发序列合并。它假设调用者已经刷新了缓冲区，因为它清除了 `last_plain_char_time`，这意味着 [`flush_if_due`] 将在另一个普通字符更新时间戳时不刷新非空缓冲区。 # 状态（概念上） - **空闲**：无缓冲文本，无待处理字符。 - **等待第一个字符**：[`pending_first_char`] 持有一个 ASCII 字符，最多 [`PASTE_BURST_CHAR_INTERVAL`] 时间后等待是否跟随突发序列。 - **活跃缓冲区**：[`active`] 和 [`buffer`] 存储粘贴内容，直到超时并插入。即使 `active` 已清除，只要缓冲区非空，就视为处于“突发上下文”中。 - **Enter 抑制窗口**：[`burst_window_until`] 在突发活动后短暂保持 Enter 被视为换行符，使多行粘贴保持分组。 # ASCII vs Non-ASCII - [`PasteBurst::on_plain_char`] 可能返回 [`CharDecision::RetainFirstChar`] 以持有第一个 ASCII 字符并避免闪烁。 - [`PasteBurst::on_plain_char_no_hold`] 从不持有（用于非 ASCII/IME 路径），因为持有非 ASCII 字符感觉像是输入丢失。 # 与调用者的契约 `PasteBurst` 不会在其自己的文本框上修改 UI 文本。调用者必须解释决策并应用相应的 UI 编辑。`ChatComposer` 使用完整的缓冲契约： - 对于每个普通 ASCII `KeyCode::Char`，调用 [`PasteBurst::on_plain_char`]。 - [`CharDecision::RetainFirstChar`]：不立即将字符插入文本框。 - [`CharDecision::BeginBufferFromPending`]：调用 [`PasteBurst::append_char_to_buffer`] 处理当前字符（已持有的字符已在突发缓冲区中）。 - [`CharDecision::BeginBuffer { retro_chars }`]：考虑回滚已插入的前缀，通过调用 [`PasteBurst::decide_begin_buffer`]。如果返回 `Some`，移除返回的 `start_byte..cursor` 范围从文本框，然后调用 [`PasteBurst::append_char_to_buffer`] 处理当前字符。如果返回 `None`，则降级为正常插入。 - [`CharDecision::BufferAppend`]：调用 [`PasteBurst::append_char_to_buffer`]。 - 对于每个普通非 ASCII `KeyCode::Char`，调用 [`PasteBurst::on_plain_char_no_hold`] 然后： - 如果返回 `Some(CharDecision::BufferAppend)`，调用 [`PasteBurst::append_char_to_buffer`]。 - 如果返回 `Some(CharDecision::BeginBuffer { retro_chars })`，调用 [`PasteBurst::decide_begin_buffer`] 如上所述（如果开始缓冲，移除抓取的前缀从文本框然后追加当前字符到缓冲区）。 - 如果返回 `None`，则正常插入。 - 在处理非字符输入（或任何不应与突发序列合并的输入），调用 [`PasteBurst::flush_before_modified_input`] 并返回返回字符串（如有）通过正常粘贴路径。 - 定期（例如在 UI 更新时）调用 [`PasteBurst::flush_if_due`]。 - [`FlushResult::Typed`]：将该单个字符作为正常打字插入。 - [`FlushResult::Paste`]：将返回的字符串视为显式粘贴。 - 当非文本输入被按下（除 Windows AltGr 外的 Ctrl/Alt 快捷键；或 Super、Hyper 和 Meta 修饰符；方向键等），调用者应使用 [`PasteBurst::clear_window_after_non_char`] 以防止下一次按键错误地与其他突发序列合并。
//! Paste-burst detection for terminals without bracketed paste.
//!
//! On some platforms (notably Windows), pastes often arrive as a rapid stream of
//! `KeyCode::Char` and `KeyCode::Enter` key events rather than as a single "paste" event.
//! In that mode, the composer needs to:
//!
//! - Prevent transient UI side effects (e.g. toggles bound to `?`) from triggering on pasted text.
//! - Ensure Enter is treated as a newline *inside the paste*, not as "submit the message".
//! - Avoid flicker caused by inserting a typed prefix and then immediately reclassifying it as
//!   paste once enough chars have arrived.
//!
//! This module provides the `PasteBurst` state machine. `ChatComposer` feeds it only
//! text-producing character events (plain, Shift, or Windows AltGr) and uses the full buffering
//! decisions to either:
//!
//! - briefly hold a first ASCII char (flicker suppression),
//! - buffer a burst as a single pasted string, or
//! - let input flow through as normal typing.
//!
//! # Call Pattern
//!
//! `PasteBurst` is a pure state machine: it never mutates the textarea directly. The caller feeds
//! it events and then applies the chosen action:
//!
//! - For each text-producing `KeyCode::Char`, call [`PasteBurst::on_plain_char`] (ASCII) or
//!   [`PasteBurst::on_plain_char_no_hold`] (non-ASCII/IME).
//! - If the decision indicates buffering, the caller appends to `PasteBurst.buffer` via
//!   [`PasteBurst::append_char_to_buffer`].
//! - On a UI tick, call [`PasteBurst::flush_if_due`]. If it returns [`FlushResult::Typed`], insert
//!   that char as normal typing. If it returns [`FlushResult::Paste`], treat the returned string as
//!   an explicit paste.
//! - Before applying non-text input (arrow keys; Ctrl/Alt shortcuts other than Windows AltGr;
//!   or Super, Hyper, and Meta modifiers), use
//!   [`PasteBurst::flush_before_modified_input`] to avoid leaving buffered text "stuck", and then
//!   [`PasteBurst::clear_window_after_non_char`] so subsequent typing does not get grouped into a
//!   previous burst.
//! - Direct-insert callers can skip buffering, use
//!   [`PasteBurst::direct_insert_newline_should_insert`] in their Enter handler, and call
//!   [`PasteBurst::extend_window`] when Enter or [`PasteBurst::on_plain_char_no_hold`] reports a
//!   burst-like stream.
//!
//! # State Variables
//!
//! This state machine is encoded in a few fields with slightly different meanings:
//!
//! - `active`: true while we are still *actively* accepting characters into the current burst.
//! - `buffer`: accumulated burst text that will eventually flush as a single `Paste(String)`.
//!   A non-empty buffer is treated as "in burst context" even if `active` has been cleared.
//! - `pending_first_char`: a single held ASCII char used for flicker suppression. The caller must
//!   not render this char until it either becomes part of a burst (`BeginBufferFromPending`) or
//!   flushes as a normal typed char (`FlushResult::Typed`).
//! - `last_plain_char_time`/`consecutive_plain_char_burst`: the timing/count heuristic for
//!   "paste-like" streams.
//! - `burst_window_until`: the Enter suppression window ("Enter inserts newline") that outlives the
//!   buffer itself.
//!
//! # Timing Model
//!
//! There are two timeouts:
//!
//! - `PASTE_BURST_CHAR_INTERVAL`: maximum delay between consecutive "plain" chars for them to be
//!   considered part of a single burst. It also bounds how long `pending_first_char` is held.
//! - `PASTE_BURST_ACTIVE_IDLE_TIMEOUT`: once buffering is active, how long to wait after the last
//!   char before flushing the accumulated buffer as a paste.
//!
//! `flush_if_due()` intentionally uses `>` (not `>=`) when comparing elapsed time, so tests and UI
//! ticks should cross the threshold by at least 1ms (see `recommended_flush_delay()`).
//!
//! # Retro Capture Details
//!
//! Retro-capture exists to handle the case where we initially inserted characters as "normal
//! typing", but later decide that the stream is paste-like. When that happens, we retroactively
//! remove a prefix of already-inserted text from the textarea and move it into the burst buffer so
//! the eventual `handle_paste(...)` sees a contiguous pasted string.
//!
//! Retro-capture mostly matters on paths that do *not* hold the first character (non-ASCII/IME
//! input, and retro-grab scenarios). The ASCII path usually prefers
//! `RetainFirstChar -> BeginBufferFromPending`, which avoids needing retro-capture at all.
//!
//! Retro-capture is expressed in terms of characters, not bytes:
//!
//! - `CharDecision::BeginBuffer { retro_chars }` uses `retro_chars` as a character count.
//! - `decide_begin_buffer(now, before_cursor, retro_chars)` turns that into a UTF-8 byte range by
//!   calling `retro_start_index()`.
//! - `RetroGrab.start_byte` is a byte index into the `before_cursor` slice; callers must clamp the
//!   cursor to a char boundary before slicing so `start_byte..cursor` is always valid UTF-8.
//!
//! # Clearing vs Flushing
//!
//! There are two ways callers end burst handling, and they are not interchangeable:
//!
//! - `flush_before_modified_input()` returns the buffered text (and/or a pending first ASCII char)
//!   so the caller can apply it through the normal paste path before handling an unrelated input.
//! - `clear_window_after_non_char()` clears the *classification window* so subsequent typing does
//!   not get grouped into the previous burst. It assumes the caller has already flushed any buffer
//!   because it clears `last_plain_char_time`, which means `flush_if_due()` will not flush a
//!   non-empty buffer until another plain char updates the timestamp.
//!
//! # States (Conceptually)
//!
//! - **Idle**: no buffered text, no pending char.
//! - **Pending first char**: `pending_first_char` holds one ASCII char for up to
//!   `PASTE_BURST_CHAR_INTERVAL` while we wait to see if a burst follows.
//! - **Active buffer**: `active`/`buffer` holds paste-like content until it times out and flushes.
//! - **Enter suppress window**: `burst_window_until` keeps Enter treated as newline briefly after
//!   burst activity so multiline pastes stay grouped.
//!
//! # ASCII vs Non-ASCII
//!
//! - [`PasteBurst::on_plain_char`] may return [`CharDecision::RetainFirstChar`] to hold the first
//!   ASCII char and avoid flicker.
//! - [`PasteBurst::on_plain_char_no_hold`] never holds (used for IME/non-ASCII paths), since
//!   holding a non-ASCII character can feel like dropped input.
//!
//! # Contract With Callers
//!
//! `PasteBurst` does not mutate the UI text buffer on its own. Callers must interpret decisions
//! and apply the corresponding UI edits. `ChatComposer` uses the full buffering contract:
//!
//! - For each plain ASCII `KeyCode::Char`, call [`PasteBurst::on_plain_char`].
//!   - [`CharDecision::RetainFirstChar`]: do **not** insert the char into the textarea yet.
//!   - [`CharDecision::BeginBufferFromPending`]: call [`PasteBurst::append_char_to_buffer`] for the
//!     current char (the previously-held char is already in the burst buffer).
//!   - [`CharDecision::BeginBuffer { retro_chars }`]: consider retro-capturing the already-inserted
//!     prefix by calling [`PasteBurst::decide_begin_buffer`]. If it returns `Some`, remove the
//!     returned `start_byte..cursor` range from the textarea and then call
//!     [`PasteBurst::append_char_to_buffer`] for the current char. If it returns `None`, fall back
//!     to normal insertion.
//!   - [`CharDecision::BufferAppend`]: call [`PasteBurst::append_char_to_buffer`].
//!
//! - For each plain non-ASCII `KeyCode::Char`, call [`PasteBurst::on_plain_char_no_hold`] and then:
//!   - If it returns `Some(CharDecision::BufferAppend)`, call
//!     [`PasteBurst::append_char_to_buffer`].
//!   - If it returns `Some(CharDecision::BeginBuffer { retro_chars })`, call
//!     [`PasteBurst::decide_begin_buffer`] as above (and if buffering starts, remove the grabbed
//!     prefix from the textarea and then append the current char to the buffer).
//!   - If it returns `None`, insert normally.
//!
//! - Before applying non-char input (or any input that should not join a burst), call
//!   [`PasteBurst::flush_before_modified_input`] and pass the returned string (if any) through the
//!   normal paste path.
//!
//! - Periodically (e.g. on a UI tick), call [`PasteBurst::flush_if_due`].
//!   - [`FlushResult::Typed`]: insert that single char as normal typing.
//!   - [`FlushResult::Paste`]: treat the returned string as an explicit paste.
//!
//! - When non-text input is pressed (Ctrl/Alt shortcuts other than Windows AltGr; Super, Hyper,
//!   and Meta modifiers; arrows; etc.), callers should use
//!   [`PasteBurst::clear_window_after_non_char`] to prevent the next keystroke from being
//!   incorrectly grouped into a previous burst.

use std::time::Duration;
use std::time::Instant;

// 用于检测粘贴样式的阈值。
// Heuristic thresholds for detecting paste-like input bursts.
// Detect quickly to avoid showing typed prefix before paste is recognized
const PASTE_BURST_MIN_CHARS: u16 = 3;
const PASTE_ENTER_SUPPRESS_WINDOW: Duration = Duration::from_millis(120);

// 字符组内连续字符之间的最大延迟时间
// Maximum delay between consecutive chars to be considered part of a paste burst.
const PASTE_BURST_CHAR_INTERVAL: Duration = Duration::from_millis(8);

// 空闲超时后刷新已缓冲的粘贴内容。
// Idle timeout before flushing buffered paste content.
// Slower paste bursts have been observed in Windows environments.
#[cfg(not(windows))]
const PASTE_BURST_ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(8);
#[cfg(windows)]
const PASTE_BURST_ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(60);

#[derive(Default)]
pub(crate) struct PasteBurst {
    last_plain_char_time: Option<Instant>,
    consecutive_plain_char_burst: u16,
    burst_window_until: Option<Instant>,
    buffer: String,
    active: bool,
    // 短暂按住第一个快速字符以避免渲染闪烁
    // Hold first fast char briefly to avoid rendering flicker
    pending_first_char: Option<(char, Instant)>,
}

pub(crate) enum CharDecision {
    /// 开始缓冲并回滚已插入的字符。
    /// Start buffering and retroactively capture some already-inserted chars.
    BeginBuffer { retro_chars: u16 },
    /// 正在缓冲，将当前字符添加到缓冲区。
    /// We are currently buffering; append the current char into the buffer.
    BufferAppend,
    /// 暂存第一个快速字符，待后续出现粘贴式爆发后再插入渲染。
    /// Do not insert/render this char yet; temporarily save the first fast
    /// char while we wait to see if a paste-like burst follows.
    RetainFirstChar,
    /// 使用之前保存的第一字符开始缓冲，无需进行回滚操作。
    /// Begin buffering using the previously saved first char (no retro grab needed).
    BeginBufferFromPending,
}

pub(crate) struct RetroGrab {
    pub start_byte: usize,
    pub grabbed: String,
}

pub(crate) enum FlushResult {
    Paste(String),
    Typed(char),
    None,
}

impl PasteBurst {
    /// 在模拟按键（或调度 UI 刷新）之间设置推荐的延迟，以便将正在缓冲的快速按键内容正常写入缓冲区检测器。主要用于测试和 TUI 以可靠地跨越粘贴 - 缓冲区时间阈值。
    /// Recommended delay to wait between simulated keypresses (or before
    /// scheduling a UI tick) so that a pending fast keystroke is flushed
    /// out of the burst detector as normal typed input.
    ///
    /// Primarily used by tests and by the TUI to reliably cross the
    /// paste-burst timing threshold.
    pub fn recommended_flush_delay() -> Duration {
        PASTE_BURST_CHAR_INTERVAL + Duration::from_millis(1)
    }

    #[cfg(test)]
    pub(crate) fn recommended_active_flush_delay() -> Duration {
        PASTE_BURST_ACTIVE_IDLE_TIMEOUT + Duration::from_millis(1)
    }

    /// 入口点：决定当前时间对普通字符的处理方式。
    /// Entry point: decide how to treat a plain char with current timing.
    pub fn on_plain_char(&mut self, ch: char, now: Instant) -> CharDecision {
        self.note_plain_char(now);

        if self.active {
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return CharDecision::BufferAppend;
        }

        // 如果已持有第一个字符且接收到第二个快速字符，则无需回溯（即从未渲染过第一个）。
        // If we already held a first char and receive a second fast char,
        // start buffering without retro-grabbing (we never rendered the first).
        if let Some((held, held_at)) = self.pending_first_char
            && now.duration_since(held_at) <= PASTE_BURST_CHAR_INTERVAL
        {
            self.active = true;
            // 取以清除待处理；已在上述捕获的字符已被持有
            // take() to clear pending; we already captured the held char above
            let _ = self.pending_first_char.take();
            self.buffer.push(held);
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return CharDecision::BeginBufferFromPending;
        }

        if self.consecutive_plain_char_burst >= PASTE_BURST_MIN_CHARS {
            return CharDecision::BeginBuffer {
                retro_chars: self.consecutive_plain_char_burst.saturating_sub(1),
            };
        }

        // 保存第一个快速字符，以便观察是否随后出现爆发。
        // Save the first fast char very briefly to see if a burst follows.
        self.pending_first_char = Some((ch, now));
        CharDecision::RetainFirstChar
    }

    /// 与 PlainChar 类似，但不会持有第一个字符。 用于非 ASCII 输入路径（例如 IMEs），其中持有字符可能感觉像丢弃了输入，但仍允许基于突发的粘贴检测。 注意：该方法将永远只返回 BufferAppend 或 BeginBuffer。
    /// Like on_plain_char(), but never holds the first char.
    ///
    /// Used for non-ASCII input paths (e.g., IMEs) where holding a character can
    /// feel like dropped input, while still allowing burst-based paste detection.
    ///
    /// Note: This method will only ever return BufferAppend or BeginBuffer.
    pub fn on_plain_char_no_hold(&mut self, now: Instant) -> Option<CharDecision> {
        self.note_plain_char(now);

        if self.active {
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            return Some(CharDecision::BufferAppend);
        }

        if self.consecutive_plain_char_burst >= PASTE_BURST_MIN_CHARS {
            return Some(CharDecision::BeginBuffer {
                retro_chars: self.consecutive_plain_char_burst.saturating_sub(1),
            });
        }

        None
    }

    fn note_plain_char(&mut self, now: Instant) {
        match self.last_plain_char_time {
            Some(prev) if now.duration_since(prev) <= PASTE_BURST_CHAR_INTERVAL => {
                self.consecutive_plain_char_burst =
                    self.consecutive_plain_char_burst.saturating_add(1)
            }
            _ => self.consecutive_plain_char_burst = 1,
        }
        self.last_plain_char_time = Some(now);
    }

    /// 若超时时间已过，则清除任何已缓冲的突发数据。
    /// Flushes any buffered burst if the inter-key timeout has elapsed.
    ///
    /// Returns:
    ///
    /// - [`FlushResult::Paste`] when a paste burst was active and buffered text is emitted as one
    ///   pasted string.
    /// - [`FlushResult::Typed`] when a single fast first ASCII char was being held (flicker
    ///   suppression) and no burst followed before the timeout elapsed.
    /// - [`FlushResult::None`] when the timeout has not elapsed, or there is nothing to flush.
    pub fn flush_if_due(&mut self, now: Instant) -> FlushResult {
        let timeout = if self.is_active_internal() {
            PASTE_BURST_ACTIVE_IDLE_TIMEOUT
        } else {
            PASTE_BURST_CHAR_INTERVAL
        };
        let timed_out = self.last_plain_char_time.is_some_and(|t| now.duration_since(t) > timeout);
        if timed_out && self.is_active_internal() {
            self.active = false;
            let out = std::mem::take(&mut self.buffer);
            FlushResult::Paste(out)
        } else if timed_out {
            // 如果只保存一个快速字符且没有后续爆发，则将其作为正常输入的流处理。
            // If we were saving a single fast char and no burst followed,
            // flush it as normal typed input.
            if let Some((ch, _at)) = self.pending_first_char.take() {
                FlushResult::Typed(ch)
            } else {
                FlushResult::None
            }
        } else {
            FlushResult::None
        }
    }

    /// 在爆发性模式下，将换行符累积到缓冲区中，而非直接提交文本域。
    /// While bursting: accumulate a newline into the buffer instead of
    /// submitting the textarea.
    ///
    /// Returns true if a newline was appended (we are in a burst context),
    /// false otherwise.
    pub fn append_newline_if_active(&mut self, now: Instant) -> bool {
        if self.is_active() {
            self.buffer.push('\n');
            self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
            true
        } else {
            false
        }
    }

    /// 决定是否将回车符插入换行（burst context）还是提交。
    /// Decide if Enter should insert a newline (burst context) vs submit.
    pub fn newline_should_insert_instead_of_submit(&self, now: Instant) -> bool {
        let in_burst_window = self.burst_window_until.is_some_and(|until| now <= until);
        self.is_active() || in_burst_window
    }

    /// 决定调用者是否需要在插入字符时添加换行符
    /// Decide if Enter should insert a newline for callers that insert chars immediately.
    pub fn direct_insert_newline_should_insert(&self, now: Instant) -> bool {
        self.newline_should_insert_instead_of_submit(now)
            || self
                .last_plain_char_time
                .is_some_and(|t| now.duration_since(t) <= PASTE_BURST_CHAR_INTERVAL)
    }

    /// 保持窗口持续活跃
    /// Keep the burst window alive.
    pub fn extend_window(&mut self, now: Instant) {
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// 使用回溯性抓取文本进行缓冲。
    /// Begin buffering with retroactively grabbed text.
    pub fn begin_with_retro_grabbed(&mut self, grabbed: String, now: Instant) {
        if !grabbed.is_empty() {
            self.buffer.push_str(&grabbed);
        }
        self.active = true;
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// 将字符追加到缓冲区中
    /// Append a char into the burst buffer.
    pub fn append_char_to_buffer(&mut self, ch: char, now: Instant) {
        self.buffer.push(ch);
        self.burst_window_until = Some(now + PASTE_ENTER_SUPPRESS_WINDOW);
    }

    /// 仅在已存在活跃 Burst 时尝试将字符追加到缓冲区。
    /// Try to append a char into the burst buffer only if a burst is already active.
    ///
    /// Returns true when the char was captured into the existing burst, false otherwise.
    pub fn try_append_char_if_active(&mut self, ch: char, now: Instant) -> bool {
        if self.active || !self.buffer.is_empty() {
            self.append_char_to_buffer(ch, now);
            true
        } else {
            false
        }
    }

    /// 在光标之前截取最近字符以决定是否开始缓冲。
    /// Decide whether to begin buffering by retroactively capturing recent
    /// chars from the slice before the cursor.
    ///
    /// Heuristic: if the retro-grabbed slice contains any whitespace or is
    /// sufficiently long (>= 16 characters), treat it as paste-like to avoid
    /// rendering the typed prefix momentarily before the paste is recognized.
    /// This favors responsiveness and prevents flicker for typical pastes
    /// (URLs, file paths, multiline text) while not triggering on short words.
    ///
    /// Returns Some(RetroGrab) with the start byte and grabbed text when we
    /// decide to buffer retroactively; otherwise None.
    pub fn decide_begin_buffer(
        &mut self,
        now: Instant,
        before: &str,
        retro_chars: usize,
    ) -> Option<RetroGrab> {
        let start_byte = retro_start_index(before, retro_chars);
        let grabbed = before[start_byte..].to_string();
        let looks_pastey =
            grabbed.chars().any(char::is_whitespace) || grabbed.chars().count() >= 16;
        if looks_pastey {
            // 注意：调用者需负责从 UI 文本中移除此切片。
            // Note: caller is responsible for removing this slice from UI text.
            self.begin_with_retro_grabbed(grabbed.clone(), now);
            Some(RetroGrab { start_byte, grabbed })
        } else {
            None
        }
    }

    /// 在应用修改或非字符输入前：立即刷新已缓冲的突发内容。
    /// Before applying modified/non-char input: flush buffered burst immediately.
    pub fn flush_before_modified_input(&mut self) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        self.active = false;
        let mut out = std::mem::take(&mut self.buffer);
        if let Some((ch, _at)) = self.pending_first_char.take() {
            out.push(ch);
        }
        Some(out)
    }

    /// 仅清除计时窗口及任何未处理的第一个字符。 不直接发送或清除缓冲文本；调用者应已通过上述任一 flush 方法已进行flush。
    /// Clear only the timing window and any pending first-char.
    ///
    /// Does not emit or clear the buffered text itself; callers should have
    /// already flushed (if needed) via one of the flush methods above.
    pub fn clear_window_after_non_char(&mut self) {
        self.consecutive_plain_char_burst = 0;
        self.last_plain_char_time = None;
        self.burst_window_until = None;
        self.active = false;
        self.pending_first_char = None;
    }

    /// 返回是否处于任何粘贴突发相关的瞬态状态（正在缓冲、缓冲区非空或等待潜在爆发时保存第一个字符）
    /// Returns true if we are in any paste-burst related transient state
    /// (actively buffering, have a non-empty buffer, or have saved the first
    /// fast char while waiting for a potential burst).
    pub fn is_active(&self) -> bool {
        self.is_active_internal() || self.pending_first_char.is_some()
    }

    fn is_active_internal(&self) -> bool {
        self.active || !self.buffer.is_empty()
    }

    pub fn clear_after_explicit_paste(&mut self) {
        self.last_plain_char_time = None;
        self.consecutive_plain_char_burst = 0;
        self.burst_window_until = None;
        self.active = false;
        self.buffer.clear();
        self.pending_first_char = None;
    }
}

pub(crate) fn retro_start_index(before: &str, retro_chars: usize) -> usize {
    if retro_chars == 0 {
        return before.len();
    }
    before.char_indices().rev().nth(retro_chars.saturating_sub(1)).map(|(idx, _)| idx).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// 行为：对 ASCII 输入，短暂地“保持”第一个快字符。如果之后没有爆发，该被持有的字符最终会像普通文本输入一样正常 flush（而非粘贴）。
    /// Behavior: for ASCII input we "hold" the first fast char briefly. If no burst follows,
    /// that held char should eventually flush as normal typed input (not as a paste).
    // 验证 `ascii_first_char_is_held_then_flushes_as_typed` 场景的触发条件与预期结果。
    #[test]
    fn ascii_first_char_is_held_then_flushes_as_typed() {
        let mut burst = PasteBurst::default();
        let t0 = Instant::now();
        assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

        let t1 = t0 + PasteBurst::recommended_flush_delay() + Duration::from_millis(1);
        assert!(matches!(burst.flush_if_due(t1), FlushResult::Typed('a')));
        assert!(!burst.is_active());
    }

    /// 行为：若两个 ASCII 字符快速到达，则不应立即渲染第一个字符，而是将缓冲后的完整 Payload 一次性发送。
    /// Behavior: if two ASCII chars arrive quickly, we should start buffering without ever
    /// rendering the first one, then flush the whole buffered payload as a paste.
    // 验证 `ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste` 场景的触发条件与预期结果。
    #[test]
    fn ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste() {
        let mut burst = PasteBurst::default();
        let t0 = Instant::now();
        assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

        let t1 = t0 + Duration::from_millis(1);
        assert!(matches!(burst.on_plain_char('b', t1), CharDecision::BeginBufferFromPending));
        burst.append_char_to_buffer('b', t1);

        let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
        assert!(matches!(
            burst.flush_if_due(t2),
            FlushResult::Paste(ref s) if s == "ab"
        ));
    }

    /// 当非字符输入即将被应用时，立即 flush 任何瞬态 burst 状态（包括一个待处理的单个 ASCII 字符），以确保状态不会跨输入泄漏。
    /// Behavior: when non-char input is about to be applied, we flush any transient burst state
    /// immediately (including a single pending ASCII char) so state doesn't leak across inputs.
    // 验证 `flush_before_modified_input_includes_pending_first_char` 场景的触发条件与预期结果。
    #[test]
    fn flush_before_modified_input_includes_pending_first_char() {
        let mut burst = PasteBurst::default();
        let t0 = Instant::now();
        assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

        assert_eq!(burst.flush_before_modified_input(), Some("a".to_string()));
        assert!(!burst.is_active());
    }

    /// 行为：仅当已插入的前缀看起来像粘贴时（包含空白字符或足够长度）启用回滚抓取缓冲。
    /// Behavior: retro-grab buffering is only enabled when the already-inserted prefix looks
    /// paste-like (whitespace or "long enough") so short IME bursts don't get misclassified.
    // 验证 `decide_begin_buffer_only_triggers_for_pastey_prefixes` 场景的触发条件与预期结果。
    #[test]
    fn decide_begin_buffer_only_triggers_for_pastey_prefixes() {
        let mut burst = PasteBurst::default();
        let now = Instant::now();

        assert!(burst.decide_begin_buffer(now, "ab", /*retro_chars*/ 2).is_none());
        assert!(!burst.is_active());

        let grab = burst
            .decide_begin_buffer(now, "a b", /*retro_chars*/ 2)
            .expect("whitespace should be considered paste-like");
        assert_eq!(grab.start_byte, 1);
        assert_eq!(grab.grabbed, " b");
        assert!(burst.is_active());
    }

    /// 行为：在粘贴式爆发后，短暂保持一个“输入抑制窗口”，以便稍晚的 Enter 键仍能插入换行符而非提交。
    /// Behavior: after a paste-like burst, we keep an "enter suppression window" alive briefly so
    /// a slightly-late Enter still inserts a newline instead of submitting.
    // 验证 `newline_suppression_window_outlives_buffer_flush` 场景的触发条件与预期结果。
    #[test]
    fn newline_suppression_window_outlives_buffer_flush() {
        let mut burst = PasteBurst::default();
        let t0 = Instant::now();
        assert!(matches!(burst.on_plain_char('a', t0), CharDecision::RetainFirstChar));

        let t1 = t0 + Duration::from_millis(1);
        assert!(matches!(burst.on_plain_char('b', t1), CharDecision::BeginBufferFromPending));
        burst.append_char_to_buffer('b', t1);

        let t2 = t1 + PasteBurst::recommended_active_flush_delay() + Duration::from_millis(1);
        assert!(matches!(burst.flush_if_due(t2), FlushResult::Paste(ref s) if s == "ab"));
        assert!(!burst.is_active());

        assert!(burst.newline_should_insert_instead_of_submit(t2));
        let t3 = t1 + PASTE_ENTER_SUPPRESS_WINDOW + Duration::from_millis(1);
        assert!(!burst.newline_should_insert_instead_of_submit(t3));
    }
}
