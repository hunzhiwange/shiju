//! 在独立子进程中触发原生异常，验证记录后仍由系统终止进程。

use super::*;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetErrorMode(mode: u32) -> u32;
    fn RaiseException(code: u32, flags: u32, count: u32, arguments: *const usize);
}

#[test]
fn native_exception_is_recorded_without_swallowing_it() {
    const CHILD_DIRECTORY: &str = "VW_NATIVE_CRASH_TEST_DIRECTORY";
    if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
        // SAFETY: 仅对测试子进程关闭系统错误对话框，然后抛出不可继续的测试异常。
        unsafe { SetErrorMode(3) };
        install(Path::new(&directory)).expect("install native diagnostics");
        unsafe { RaiseException(0xc000_001d, 1, 0, std::ptr::null()) };
        panic!("native exception must not be swallowed");
    }
    let directory = tempfile::tempdir().expect("temporary native crash directory");
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "native_crash::tests::native_exception_is_recorded_without_swallowing_it",
        ])
        .env(CHILD_DIRECTORY, directory.path())
        .output()
        .expect("run native crash child");
    assert!(!output.status.success());
    let log = std::fs::read_to_string(directory.path().join("native-crash.log"))
        .expect("native exception log");
    assert!(log.contains("code=0xc000001d"));
    assert!(log.contains("fault:"));
    assert!(log.contains("frame:"));
}
