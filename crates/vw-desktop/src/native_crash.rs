//! Windows 原生异常的最小诊断记录；不修改异常处理结果或尝试继续执行。

use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static LOG: OnceLock<File> = OnceLock::new();
static RECORDING: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct ExceptionRecord {
    code: u32,
    flags: u32,
    nested: *const ExceptionRecord,
    address: *const c_void,
    parameter_count: u32,
    parameters: [usize; 15],
}

#[repr(C)]
struct ExceptionPointers {
    record: *const ExceptionRecord,
    context: *const c_void,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AddVectoredExceptionHandler(
        first: u32,
        handler: unsafe extern "system" fn(*const ExceptionPointers) -> i32,
    ) -> *mut c_void;
    fn RtlCaptureStackBackTrace(
        skip: u32,
        count: u32,
        frames: *mut *mut c_void,
        hash: *mut u32,
    ) -> u16;
    fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, path: *mut u16, length: u32) -> u32;
    fn SetThreadStackGuarantee(size: *mut u32) -> i32;
}

pub(super) fn install(directory: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let file =
        OpenOptions::new().create(true).append(true).open(directory.join("native-crash.log"))?;
    if LOG.set(file).is_err() {
        return Ok(());
    }
    // 为主线程栈溢出后的异常记录保留少量栈空间。
    let mut reserved_stack = 64 * 1024;
    // SAFETY: 系统在当前线程保存保证大小；处理器在进程退出前始终有效。
    unsafe {
        SetThreadStackGuarantee(&mut reserved_stack);
        if AddVectoredExceptionHandler(1, record_exception).is_null() {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

unsafe extern "system" fn record_exception(pointers: *const ExceptionPointers) -> i32 {
    // SAFETY: Windows 在异常分发期间提供有效指针；同时防止空指针和递归故障。
    let Some(pointers) = (unsafe { pointers.as_ref() }) else { return 0 };
    let Some(record) = (unsafe { pointers.record.as_ref() }) else { return 0 };
    if !matches!(
        record.code,
        0xc000_0005 | 0xc000_001d | 0xc000_00fd | 0xc000_0409 | 0xc000_041d | 0xe06d_7363
    ) || RECORDING.swap(true, Ordering::AcqRel)
    {
        return 0;
    }
    if let Some(mut file) = LOG.get() {
        let _ = writeln!(file, "native exception (first chance): code=0x{:08x}", record.code);
        record_address(&mut file, "fault", record.address);
        if record.code != 0xc000_00fd {
            let mut frames = [std::ptr::null_mut(); 32];
            // SAFETY: 输出缓冲区可容纳指定数量的调用地址。
            let count = unsafe {
                RtlCaptureStackBackTrace(
                    0,
                    frames.len() as u32,
                    frames.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            };
            for frame in &frames[..usize::from(count).min(frames.len())] {
                record_address(&mut file, "frame", *frame);
            }
        }
        let _ = file.flush();
    }
    RECORDING.store(false, Ordering::Release);
    // EXCEPTION_CONTINUE_SEARCH：保留 Windows 和现有运行时的异常处理行为。
    0
}

fn record_address(file: &mut &File, label: &str, address: *const c_void) {
    let mut module = std::ptr::null_mut();
    // FROM_ADDRESS | UNCHANGED_REFCOUNT：仅查找所属模块，不改变加载引用计数。
    // SAFETY: FROM_ADDRESS 将参数解释为地址而非字符串；输出指针有效。
    let found = unsafe { GetModuleHandleExW(0x0000_0006, address.cast(), &mut module) };
    if found == 0 {
        let _ = writeln!(file, "{label}: {address:p} (unknown module)");
        return;
    }
    let mut path = [0_u16; 1024];
    // SAFETY: module 来自系统查询；路径缓冲区与长度一致。
    let length = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) };
    let path = String::from_utf16_lossy(&path[..(length as usize).min(path.len())]);
    let offset = (address as usize).wrapping_sub(module as usize);
    let _ = writeln!(file, "{label}: {path} + 0x{offset:x}");
}

#[cfg(test)]
#[path = "native_crash_tests.rs"]
mod tests;
