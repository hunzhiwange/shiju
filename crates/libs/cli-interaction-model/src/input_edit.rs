//! 交互式 CLI 输入处理模块
//!
//! 本模块提供交互式命令行界面的文本输入编辑功能，支持光标移动、字符插入、
//! 删除等基础编辑操作。主要用于处理用户在终端中的键盘输入，并维护输入
//! 缓冲区与光标位置的一致性。
//!
//! # 功能特性
//!
//! - **光标导航**：支持左右方向键、Home/End 键进行光标定位
//! - **字符编辑**：支持字符插入、退格删除、Delete 删除
//! - **多行输入**：支持换行符插入，允许多行文本编辑
//! - **Unicode 支持**：正确处理 UTF-8 多字节字符的边界操作
//!
//! # 使用示例
//!
//! ```ignore
//! use crossterm::event::KeyCode;
//! use cli_interaction_model::input_edit::*;
//!
//! let mut input = String::from("hello");
//! let mut cursor = 5;
//!
//! // 在末尾插入字符
//! let result = apply_key_to_input(&mut input, &mut cursor, KeyCode::Char('!'));
//! assert_eq!(result, InputEditResult::Updated);
//! assert_eq!(input, "hello!");
//! ```

use crossterm::event::KeyCode;

/// 表示输入编辑操作的结果状态
///
/// 该枚举用于指示键盘事件处理后输入缓冲区的变化情况，
/// 便于调用方决定是否需要重绘界面或执行其他后续操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEditResult {
    /// 输入未发生变化
    ///
    /// 当键盘事件未被识别或不影响输入内容时返回此变体，
    /// 例如按下了功能键（F1-F12）、修饰键等非编辑类按键。
    NoChange,

    /// 输入已更新
    ///
    /// 当键盘事件成功修改了输入内容或光标位置时返回此变体，
    /// 调用方通常需要据此触发界面重绘。
    Updated,
}

/// 将键盘事件应用到输入缓冲区
///
/// 根据传入的按键类型对输入字符串和光标位置执行相应的编辑操作。
/// 该函数是交互式输入处理的核心，统一处理所有文本编辑相关的键盘事件。
///
/// # 参数
///
/// * `input` - 可变引用，指向待编辑的输入字符串缓冲区
/// * `cursor_idx` - 可变引用，指向当前光标位置（以字符为单位，非字节）
/// * `key` - 待处理的键盘事件代码
///
/// # 返回值
///
/// 返回 [`InputEditResult`] 枚举，指示此次操作是否改变了输入状态：
/// - [`InputEditResult::Updated`] - 输入内容或光标位置已修改
/// - [`InputEditResult::NoChange`] - 按键未被识别，输入未改变
///
/// # 支持的按键操作
///
/// | 按键 | 行为 |
/// |------|------|
/// | `Left` | 光标左移一位（最小为 0） |
/// | `Right` | 光标右移一位（最大为字符串末尾） |
/// | `Home` | 光标移动到行首（位置 0） |
/// | `End` | 光标移动到行尾（字符串末尾） |
/// | `Backspace` | 删除光标前一个字符，光标左移 |
/// | `Delete` | 删除光标位置的字符，光标不动 |
/// | `Char('\n')` / `Char('\r')` | 在光标处插入换行符 |
/// | `Char(c)` | 在光标处插入普通字符 |
///
/// # 示例
///
/// ```ignore
/// use crossterm::event::KeyCode;
///
/// let mut text = String::from("abc");
/// let mut pos = 1;
///
/// // 插入字符
/// assert_eq!(
///     apply_key_to_input(&mut text, &mut pos, KeyCode::Char('X')),
///     InputEditResult::Updated
/// );
/// assert_eq!(text, "aXbc");
/// assert_eq!(pos, 2);
///
/// // 删除字符
/// assert_eq!(
///     apply_key_to_input(&mut text, &mut pos, KeyCode::Backspace),
///     InputEditResult::Updated
/// );
/// assert_eq!(text, "abc");
/// assert_eq!(pos, 1);
/// ```
///
/// # 注意事项
///
/// - 光标位置以 Unicode 字符为单位计算，而非字节偏移
/// - 对于无效按键（如功能键、Alt 组合键等），返回 `NoChange`
/// - 所有编辑操作都会重建字符串，在极端高频输入下可能有性能影响
pub fn apply_key_to_input(
    input: &mut String,
    cursor_idx: &mut usize,
    key: KeyCode,
) -> InputEditResult {
    match key {
        // 处理光标左移：使用 saturating_sub 确保不会下溢至负数
        KeyCode::Left => {
            *cursor_idx = cursor_idx.saturating_sub(1);
            InputEditResult::Updated
        }

        // 处理光标右移：仅在未到达字符串末尾时允许移动
        KeyCode::Right => {
            if *cursor_idx < input.chars().count() {
                *cursor_idx += 1;
            }
            InputEditResult::Updated
        }

        // 处理 Home 键：将光标移动到输入起始位置
        KeyCode::Home => {
            *cursor_idx = 0;
            InputEditResult::Updated
        }

        // 处理 End 键：将光标移动到输入末尾
        KeyCode::End => {
            *cursor_idx = input.chars().count();
            InputEditResult::Updated
        }

        // 处理退格键：删除光标前一个字符
        // 仅在光标不在起始位置时执行删除操作
        KeyCode::Backspace => {
            if *cursor_idx > 0 {
                // 将字符串转换为字符向量以便按索引操作
                // 注意：这是为了正确处理多字节 UTF-8 字符
                let mut chars: Vec<char> = input.chars().collect();
                chars.remove(*cursor_idx - 1);
                *input = chars.into_iter().collect();
                // 光标跟随左移
                *cursor_idx = cursor_idx.saturating_sub(1);
            }
            InputEditResult::Updated
        }

        // 处理 Delete 键：删除光标位置的字符
        // 仅在光标不在末尾时执行删除操作
        KeyCode::Delete => {
            if *cursor_idx < input.chars().count() {
                let mut chars: Vec<char> = input.chars().collect();
                chars.remove(*cursor_idx);
                *input = chars.into_iter().collect();
            }
            InputEditResult::Updated
        }

        // 处理换行符输入：支持多行文本编辑
        // 同时处理 \n 和 \r 以兼容不同平台的换行约定
        KeyCode::Char('\n' | '\r') => {
            let mut chars: Vec<char> = input.chars().collect();
            chars.insert(*cursor_idx, '\n');
            *input = chars.into_iter().collect();
            *cursor_idx += 1;
            InputEditResult::Updated
        }

        // 处理普通字符输入：在光标位置插入字符
        KeyCode::Char(ch) => {
            let mut chars: Vec<char> = input.chars().collect();
            chars.insert(*cursor_idx, ch);
            *input = chars.into_iter().collect();
            // 插入后光标右移一位
            *cursor_idx += 1;
            InputEditResult::Updated
        }

        // 未识别的按键：不做任何处理
        _ => InputEditResult::NoChange,
    }
}

/// 在输入缓冲区中插入换行符
///
/// 这是一个便捷函数，用于在当前光标位置插入换行符。
/// 功能上等同于调用 `apply_key_to_input(..., KeyCode::Char('\n'))`，
/// 但提供了更明确的语义化接口。
///
/// # 参数
///
/// * `input` - 可变引用，指向待编辑的输入字符串缓冲区
/// * `cursor_idx` - 可变引用，指向当前光标位置
///
/// # 示例
///
/// ```ignore
/// let mut text = String::from("hello world");
/// let mut pos = 5;
///
/// insert_newline(&mut text, &mut pos);
/// assert_eq!(text, "hello\n world");
/// assert_eq!(pos, 6);
/// ```
///
/// # 实现说明
///
/// 该函数通过将字符串转换为字符向量来实现插入操作，
/// 确保正确处理 UTF-8 多字节字符的边界情况。
/// 插入后光标位置会自动加一，指向新插入的换行符之后。
pub fn insert_newline(input: &mut String, cursor_idx: &mut usize) {
    // 将字符串转换为字符向量以支持按索引插入
    let mut chars: Vec<char> = input.chars().collect();
    // 在光标位置插入换行符
    chars.insert(*cursor_idx, '\n');
    // 将字符向量重新组装为字符串
    *input = chars.into_iter().collect();
    // 更新光标位置到换行符之后
    *cursor_idx += 1;
}
