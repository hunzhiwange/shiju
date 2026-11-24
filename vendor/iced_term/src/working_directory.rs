use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct WorkingDirectory {
    last_known: Arc<Mutex<Option<PathBuf>>>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pid: Arc<std::sync::atomic::AtomicU32>,
}

impl WorkingDirectory {
    pub(super) fn new(
        initial: Option<PathBuf>,
        #[cfg(any(target_os = "linux", target_os = "macos"))] pid: u32,
    ) -> Self {
        Self {
            last_known: Arc::new(Mutex::new(initial)),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            pid: Arc::new(std::sync::atomic::AtomicU32::new(pid)),
        }
    }

    pub(super) fn mark_exited(&self) {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        self.pid.store(0, std::sync::atomic::Ordering::Release);
    }

    pub(crate) fn get(&self) -> Option<PathBuf> {
        let directory = self.last_known.lock().ok()?;
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            use std::sync::atomic::Ordering;
            let mut directory = directory;
            let pid = self.pid.load(Ordering::Acquire);
            let current = if pid == 0 {
                None
            } else {
                process_directory(pid)
            };
            if self.pid.load(Ordering::Acquire) == pid {
                if let Some(current) = current {
                    *directory = Some(current);
                }
            }
            return directory.clone();
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        directory.clone()
    }
}

#[cfg(target_os = "linux")]
fn process_directory(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .filter(|path| path.is_absolute())
}

#[cfg(target_os = "macos")]
fn process_directory(pid: u32) -> Option<PathBuf> {
    use std::ffi::OsStr;
    use std::mem::{size_of, MaybeUninit};
    use std::os::unix::ffi::OsStrExt;

    let mut info = MaybeUninit::<libc::proc_vnodepathinfo>::uninit();
    let length = size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    // SAFETY: The buffer has the size required by PROC_PIDVNODEPATHINFO.
    let read = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr().cast(),
            length,
        )
    };
    if read != length {
        return None;
    }

    // SAFETY: The kernel initialized the complete structure above.
    let info = unsafe { info.assume_init() };
    let bytes: Vec<u8> = info
        .pvi_cdir
        .vip_path
        .iter()
        .flatten()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != 0)
        .collect();
    let path = PathBuf::from(OsStr::from_bytes(&bytes));
    path.is_absolute().then_some(path)
}
