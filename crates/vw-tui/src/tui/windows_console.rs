//! Windows 控制台输入模式的保存、强制与恢复逻辑。
//!
//! Crossterm 需要 Win32 输入记录而非 VT 输入字节；本模块只切换该位，并按嵌套会话栈恢复原值。

/// Windows 控制台的虚拟终端输入标志位。
const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;

/// 进入 TUI 前原始 VT 输入位的状态。
#[derive(Clone, Copy)]
pub(super) enum VirtualTerminalInput {
    Enabled,
    Disabled,
}

/// 清除 VT 输入位，使 Crossterm 接收 Win32 输入记录。
pub(super) fn input_record_mode(mode: u32) -> u32 {
    mode & !ENABLE_VIRTUAL_TERMINAL_INPUT
}

/// 将当前模式中的 VT 输入位恢复为进入时记录的状态。
pub(super) fn restored_input_mode(mode: u32, original: VirtualTerminalInput) -> u32 {
    match original {
        VirtualTerminalInput::Enabled => mode | ENABLE_VIRTUAL_TERMINAL_INPUT,
        VirtualTerminalInput::Disabled => input_record_mode(mode),
    }
}

#[cfg(windows)]
// 以栈保存嵌套终端会话的原始 VT 输入位；锁中毒时仍取回内部值完成恢复。
static ORIGINAL_VT_INPUT: std::sync::Mutex<Vec<VirtualTerminalInput>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(windows)]
/// 获取标准输入句柄及当前控制台模式；非控制台句柄安全返回空。
fn current_input_mode() -> Option<(windows_sys::Win32::Foundation::HANDLE, u32)> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::GetConsoleMode;
    use windows_sys::Win32::System::Console::GetStdHandle;
    use windows_sys::Win32::System::Console::STD_INPUT_HANDLE;

    // 安全性：只读取进程标准输入句柄，随后验证空值和 INVALID_HANDLE_VALUE。
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE || handle == 0 {
        return None;
    }

    let mut mode = 0;
    // 安全性：句柄已验证，`mode` 在调用期间提供有效可写指针。
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return None;
    }

    Some((handle, mode))
}

#[cfg(windows)]
/// 首次取得终端所有权时清除 VT 输入位，并压栈记录原始状态。
pub(super) fn set_input_record_mode() -> std::io::Result<()> {
    use windows_sys::Win32::System::Console::SetConsoleMode;

    let Some((handle, mode)) = current_input_mode() else {
        return Ok(());
    };
    let requested_mode = input_record_mode(mode);
    if requested_mode != mode && unsafe { SetConsoleMode(handle, requested_mode) } == 0 {
        return Err(std::io::Error::last_os_error());
    }

    let original = if mode & ENABLE_VIRTUAL_TERMINAL_INPUT != 0 {
        VirtualTerminalInput::Enabled
    } else {
        VirtualTerminalInput::Disabled
    };
    ORIGINAL_VT_INPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(original);
    Ok(())
}

#[cfg(windows)]
/// 事件流启动或外部客户端改动后再次确保 Win32 输入记录模式。
pub(super) fn ensure_input_record_mode() -> std::io::Result<()> {
    use windows_sys::Win32::System::Console::SetConsoleMode;

    let Some((handle, mode)) = current_input_mode() else {
        return Ok(());
    };
    let requested_mode = input_record_mode(mode);
    if requested_mode != mode && unsafe { SetConsoleMode(handle, requested_mode) } == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(windows)]
/// 弹出最近一次原始状态并恢复 VT 输入位；没有保存状态时幂等返回。
pub(super) fn restore_input_mode() -> std::io::Result<()> {
    use windows_sys::Win32::System::Console::SetConsoleMode;

    let mut original_modes = ORIGINAL_VT_INPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(original) = original_modes.last().copied() else {
        return Ok(());
    };
    let Some((handle, mode)) = current_input_mode() else {
        original_modes.pop();
        return Ok(());
    };
    let requested_mode = restored_input_mode(mode, original);
    if requested_mode != mode && unsafe { SetConsoleMode(handle, requested_mode) } == 0 {
        return Err(std::io::Error::last_os_error());
    }

    original_modes.pop();
    Ok(())
}
