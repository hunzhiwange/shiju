//! Opt-in local dictation archive. Directory entries are the recoverable index.
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};
const MAX_COUNT: usize = 20;
const MAX_AUDIO: u64 = 44 + 24_000 * 2 * 180;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub id: String,
    pub bytes: u64,
    pub modified: SystemTime,
}
/// Holding the directory descriptor prevents replacement/symlink races. flock serializes
/// cooperating application instances, including crash recovery of abandoned temporary files.
pub(crate) struct Store {
    _directory: File,
}
impl Store {
    pub(crate) fn path() -> Result<PathBuf, String> {
        crate::app::project_dirs()
            .map(|d| d.data_local_dir().join("voice-recordings"))
            .ok_or_else(|| "无法定位本机录音目录。".into())
    }
    pub(crate) fn local() -> Result<Self, String> {
        Self::open(&Self::path()?)
    }
    #[cfg(unix)]
    pub(crate) fn open(root: &Path) -> Result<Self, String> {
        use std::os::{
            fd::AsRawFd,
            unix::fs::{DirBuilderExt, OpenOptionsExt},
        };
        // The parent is the existing project_dirs application directory. Never traverse a
        // supplied archive symlink; all subsequent IO is relative to the held descriptor.
        if !root.exists() {
            std::fs::create_dir_all(root.parent().ok_or("录音目录无效。")?).map_err(io_error)?;
            match std::fs::DirBuilder::new().mode(0o700).create(root) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(io_error(e)),
            }
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(root)
            .map_err(io_error)?;
        // SAFETY: the descriptor is live and owned for the lifetime of Store.
        if unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("本机录音正在由另一窗口使用，请稍后重试。".into());
        }
        use std::os::unix::fs::PermissionsExt;
        directory.set_permissions(std::fs::Permissions::from_mode(0o700)).map_err(io_error)?;
        let store = Self { _directory: directory };
        for name in store.names()? {
            if name.starts_with(".voice-") {
                store.remove(&name)?;
            }
        }
        store.trim(0, 0)?;
        Ok(store)
    }
    #[cfg(not(unix))]
    pub(crate) fn open(_root: &Path) -> Result<Self, String> {
        Err("当前平台尚未支持安全的本机录音留存。".into())
    }
    pub(crate) fn list(&self) -> Result<Vec<Entry>, String> {
        let mut entries = Vec::new();
        for id in self.names()? {
            if valid_id(&id) {
                let metadata = self.child(&id, false)?.metadata().map_err(io_error)?;
                if !metadata.is_file() || metadata.nlink_exceeds_one() {
                    return Err("录音目录包含不安全文件，请检查本机目录。".into());
                }
                entries.push(Entry {
                    id,
                    bytes: metadata.len(),
                    modified: metadata.modified().map_err(io_error)?,
                });
            }
        }
        entries.sort_by(|a, b| a.modified.cmp(&b.modified).then(a.id.cmp(&b.id)));
        Ok(entries)
    }
    fn trim(&self, incoming: u64, count: usize) -> Result<(), String> {
        let entries = self.list()?;
        let mut bytes = entries.iter().map(|e| e.bytes).sum::<u64>();
        let mut len = entries.len();
        for entry in entries {
            if len + count <= MAX_COUNT && bytes.saturating_add(incoming) <= MAX_TOTAL {
                break;
            }
            self.delete(&entry.id)?;
            bytes = bytes.saturating_sub(entry.bytes);
            len -= 1;
        }
        Ok(())
    }
    pub(crate) fn save(&self, enabled: bool, wav: &[u8]) -> Result<(), String> {
        if !enabled {
            return Ok(());
        }
        pcm(wav)?;
        self.trim(wav.len() as u64, 1)?;
        let temporary = format!(".voice-{:032x}", rand::random::<u128>());
        let result: Result<(), String> = (|| {
            let mut file = self.child(&temporary, true)?;
            file.write_all(wav).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
            self.publish(&temporary, &format!("{:032x}.wav", rand::random::<u128>()))?;
            Ok(())
        })();
        let cleanup = self.remove(&temporary);
        result?;
        cleanup?;
        self._directory.sync_all().map_err(io_error)
    }
    pub(crate) fn read(&self, id: &str) -> Result<Vec<u8>, String> {
        if !valid_id(id) {
            return Err("录音标识无效。".into());
        }
        let file =
            self.child(id, false).map_err(|_| "录音已丢失或不可读取，请删除该条目。".to_owned())?;
        let meta = file.metadata().map_err(io_error)?;
        if !meta.is_file() || meta.nlink_exceeds_one() || meta.len() > MAX_AUDIO {
            return Err("录音文件无效或过大，请删除该条目。".into());
        }
        let mut wav = Vec::new();
        file.take(MAX_AUDIO + 1).read_to_end(&mut wav).map_err(io_error)?;
        pcm(&wav)?;
        Ok(wav)
    }
    pub(crate) fn delete(&self, id: &str) -> Result<(), String> {
        if !valid_id(id) {
            return Err("录音标识无效。".into());
        }
        self.remove(id)?;
        self._directory.sync_all().map_err(io_error)
    }
    pub(crate) fn clear(&self) -> Result<(), String> {
        for entry in self.list()? {
            self.delete(&entry.id)?;
        }
        Ok(())
    }
    #[cfg(unix)]
    fn child(&self, name: &str, create: bool) -> Result<File, String> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let name = std::ffi::CString::new(name).map_err(|_| "录音标识无效。")?;
        let flags = libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if create { libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL } else { libc::O_RDONLY };
        // SAFETY: live directory FD and NUL-terminated child name; no borrowed FDs escape.
        let fd = unsafe { libc::openat(self._directory.as_raw_fd(), name.as_ptr(), flags, 0o600) };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // SAFETY: openat returned a newly owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    #[cfg(unix)]
    fn names(&self) -> Result<Vec<String>, String> {
        use std::os::fd::AsRawFd;
        // SAFETY: open a distinct directory description so enumeration does not share offset.
        let fd = unsafe {
            libc::openat(
                self._directory.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // SAFETY: fdopendir takes ownership on success; close on failure.
        let dir = unsafe { libc::fdopendir(fd) };
        if dir.is_null() {
            unsafe { libc::close(fd) };
            return Err("无法读取录音目录。".into());
        }
        let mut names = Vec::new();
        loop {
            // SAFETY: errno is thread-local; clear it to distinguish EOF from IO failure.
            #[cfg(target_os = "macos")]
            unsafe {
                *libc::__error() = 0;
            }
            #[cfg(target_os = "linux")]
            unsafe {
                *libc::__errno_location() = 0;
            }
            // SAFETY: dir is live, used on this thread only; name is copied before next call.
            let entry = unsafe { libc::readdir(dir) };
            if entry.is_null() {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error() != Some(0) {
                        // SAFETY: dir is still owned here; this error path returns immediately.
                        unsafe {
                            libc::closedir(dir);
                        }
                        return Err(io_error(error));
                    }
                }
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            if name != "." && name != ".." {
                names.push(name);
            }
        }
        // SAFETY: close the owned DIR and its descriptor exactly once.
        unsafe { libc::closedir(dir) };
        Ok(names)
    }
    #[cfg(unix)]
    fn remove(&self, name: &str) -> Result<(), String> {
        use std::os::fd::AsRawFd;
        let name = std::ffi::CString::new(name).map_err(|_| "录音标识无效。")?;
        // SAFETY: unlinkat removes only the named child, never follows a symlink.
        if unsafe { libc::unlinkat(self._directory.as_raw_fd(), name.as_ptr(), 0) } == 0 {
            return Ok(());
        }
        let e = std::io::Error::last_os_error();
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(io_error(e)) }
    }
    #[cfg(unix)]
    fn publish(&self, temporary: &str, id: &str) -> Result<(), String> {
        use std::os::fd::AsRawFd;
        let from = std::ffi::CString::new(temporary).map_err(|_| "录音标识无效。")?;
        let to = std::ffi::CString::new(id).map_err(|_| "录音标识无效。")?;
        // SAFETY: both paths are single generated names inside the same owned directory.
        // linkat atomically publishes the completed file and refuses to replace any entry.
        if unsafe {
            libc::linkat(
                self._directory.as_raw_fd(),
                from.as_ptr(),
                self._directory.as_raw_fd(),
                to.as_ptr(),
                0,
            )
        } == 0
        {
            Ok(())
        } else {
            Err(io_error(std::io::Error::last_os_error()))
        }
    }
    #[cfg(not(unix))]
    fn child(&self, _name: &str, _create: bool) -> Result<File, String> {
        Err("当前平台不支持本机留存。".into())
    }
    #[cfg(not(unix))]
    fn names(&self) -> Result<Vec<String>, String> {
        Err("当前平台不支持本机留存。".into())
    }
    #[cfg(not(unix))]
    fn remove(&self, _name: &str) -> Result<(), String> {
        Err("当前平台不支持本机留存。".into())
    }
    #[cfg(not(unix))]
    fn publish(&self, _from: &str, _to: &str) -> Result<(), String> {
        Err("当前平台不支持本机留存。".into())
    }
}
// Strictly accept the WAV shape produced by Capture::Recording, without parsing arbitrary codecs.
pub(crate) fn pcm(wav: &[u8]) -> Result<&[u8], String> {
    if wav.len() < 46
        || wav.len() as u64 > MAX_AUDIO
        || !(wav.len() - 44).is_multiple_of(2)
        || &wav[..4] != b"RIFF"
        || &wav[8..24] != b"WAVEfmt \x10\0\0\0\x01\0\x01\0"
        || wav[24..28] != 24_000u32.to_le_bytes()
        || wav[28..32] != 48_000u32.to_le_bytes()
        || &wav[32..40] != b"\x02\0\x10\0data"
        || wav[4..8] != ((wav.len() - 8) as u32).to_le_bytes()
        || wav[40..44] != ((wav.len() - 44) as u32).to_le_bytes()
    {
        return Err("录音损坏或格式不受支持，请删除后重新录音。".into());
    }
    Ok(&wav[44..])
}
fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.ends_with(".wav") && id.as_bytes()[..32].iter().all(u8::is_ascii_hexdigit)
}
fn io_error(_: std::io::Error) -> String {
    "无法访问本机录音目录，请检查权限与磁盘空间。".into()
}
trait LinkCount {
    fn nlink_exceeds_one(&self) -> bool;
}
impl LinkCount for std::fs::Metadata {
    fn nlink_exceeds_one(&self) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.nlink() > 1
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}
#[cfg(test)]
#[path = "recordings_tests.rs"]
mod tests;
