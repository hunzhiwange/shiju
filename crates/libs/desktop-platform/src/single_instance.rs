//! Native launch requests and the file-open inbox shared by the desktop shells.
use std::io;
use std::path::Path;
use std::sync::Mutex;

#[cfg(unix)]
#[path = "single_instance_unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "single_instance_windows.rs"]
mod platform;

pub use platform::InstanceGuard;

const MAX_REQUEST: usize = 16 * 1024;
static OPEN_FILES: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn acquire_or_forward() -> io::Result<Option<InstanceGuard>> {
    let args = std::env::args_os().skip(1).map(|arg| arg.to_string_lossy().into_owned()).collect();
    platform::acquire_or_forward(args)
}

pub fn poll() -> Vec<Vec<String>> {
    platform::poll()
}

pub fn receive_arguments(args: Vec<String>) -> io::Result<bool> {
    let (files, other): (Vec<_>, Vec<_>) =
        args.into_iter().partition(|arg| Path::new(arg).is_absolute());
    queue_open_files(files)?;
    #[cfg(target_os = "macos")]
    return crate::deep_links::receive_arguments(&other);
    #[cfg(not(target_os = "macos"))]
    Ok(!other.is_empty())
}

pub(crate) fn queue_open_files(files: Vec<String>) -> io::Result<()> {
    OPEN_FILES.lock().map_err(|_| io::Error::other("文件打开队列不可用"))?.extend(files);
    Ok(())
}

pub fn take_open_files() -> Vec<String> {
    OPEN_FILES.lock().map(|mut files| std::mem::take(&mut *files)).unwrap_or_default()
}
