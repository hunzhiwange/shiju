//! Launch the installed helper as a macOS application with its own TCC identity.
use std::ffi::{CString, c_void};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::ptr;
use std::time::Duration;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;

type Cf = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Cf);
    fn CFDataCreate(allocator: Cf, bytes: *const u8, length: isize) -> Cf;
    fn CFStringCreateWithCString(allocator: Cf, text: *const libc::c_char, encoding: u32) -> Cf;
    fn CFDictionaryCreate(
        allocator: Cf,
        keys: *const Cf,
        values: *const Cf,
        count: isize,
        key_callbacks: Cf,
        value_callbacks: Cf,
    ) -> Cf;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
}

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecGuestAttributeAudit: Cf;
    fn SecCodeCopyGuestWithAttributes(host: Cf, attributes: Cf, flags: u32, guest: *mut Cf) -> i32;
    fn SecCodeCopySelf(flags: u32, code: *mut Cf) -> i32;
    fn SecRequirementCreateWithString(text: Cf, flags: u32, requirement: *mut Cf) -> i32;
    fn SecCodeCheckValidity(code: Cf, flags: u32, requirement: Cf) -> i32;
}

struct Owned(Cf);
impl Owned {
    fn new(value: Cf) -> io::Result<Self> {
        if value.is_null() {
            return Err(io::Error::other("macOS code identity unavailable"));
        }
        Ok(Self(value))
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

fn check_os_status(status: i32) -> io::Result<()> {
    if status == 0 {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("macOS code identity rejected ({status})"),
    ))
}

fn verify(code: &Owned, requirement: &str) -> io::Result<()> {
    let requirement = CString::new(requirement)?;
    // SAFETY: These are live Core Foundation objects and Security out-parameters.
    unsafe {
        let string =
            Owned::new(CFStringCreateWithCString(ptr::null(), requirement.as_ptr(), 0x0800_0100))?;
        let mut requirement = ptr::null();
        check_os_status(SecRequirementCreateWithString(string.0, 0, &mut requirement))?;
        let requirement = Owned::new(requirement)?;
        check_os_status(SecCodeCheckValidity(code.0, 0, requirement.0))
    }
}

fn code_hash(hash: &str) -> io::Result<&str> {
    let hash = hash.trim();
    if hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(hash);
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "invalid application code hash"))
}

fn verify_peer(channel: &UnixStream, hash: &str) -> io::Result<()> {
    if channel.peer_cred()?.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "control service peer UID mismatch",
        ));
    }
    // An audit token identifies the actual socket peer, even if its PID is recycled.
    let mut token = [0u32; 8];
    let mut length = std::mem::size_of_val(&token) as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            channel.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERTOKEN,
            token.as_mut_ptr().cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if length as usize != std::mem::size_of_val(&token) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid control service peer audit token",
        ));
    }
    // SAFETY: The kernel populated the audit token and its size was checked above.
    let code = unsafe {
        let data = Owned::new(CFDataCreate(ptr::null(), token.as_ptr().cast(), length as isize))?;
        let attributes = Owned::new(CFDictionaryCreate(
            ptr::null(),
            &raw const kSecGuestAttributeAudit,
            &data.0,
            1,
            (&raw const kCFTypeDictionaryKeyCallBacks).cast(),
            (&raw const kCFTypeDictionaryValueCallBacks).cast(),
        ))?;
        let mut code = ptr::null();
        check_os_status(SecCodeCopyGuestWithAttributes(ptr::null(), attributes.0, 0, &mut code))?;
        Owned::new(code)?
    };
    verify(&code, &format!("cdhash H\"{}\"", code_hash(hash)?))
}

fn app_bundle(executable: &Path) -> io::Result<&Path> {
    executable
        .ancestors()
        .nth(3)
        .filter(|bundle| bundle.extension().is_some_and(|ext| ext == "app"))
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing control application bundle")
        })
}

/// Only a one-use rendezvous path crosses Launch Services. Bootstrap data stays on the authenticated socket.
pub async fn launch(executable: &Path, mode: &str) -> io::Result<UnixStream> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let app = app_bundle(executable)?;
        let output = Command::new("/usr/bin/codesign").args(["-dv", "--verbose=4"]).arg(app)
            .kill_on_drop(true).output().await?;
        let signing = String::from_utf8_lossy(&output.stderr);
        let hash = signing.lines().find_map(|line| line.strip_prefix("CDHash="))
            .filter(|_| output.status.success()).ok_or_else(|| io::Error::other("missing control app signature"))?;
        let hash = code_hash(hash)?;
        let directory = tempfile::Builder::new().prefix("shiju-launch-").tempdir_in("/tmp")?;
        let rendezvous = directory.path().join("bootstrap.sock");
        let listener = UnixListener::bind(&rendezvous)?;
        let mut opener = Command::new("/usr/bin/open").args(["-n", "-g", "-a"]).arg(app)
            .args(["--args", mode]).arg(&rendezvous).kill_on_drop(true).spawn()?;
        let (channel, _) = tokio::select! {
            accepted = listener.accept() => accepted?,
            status = opener.wait() => {
                if !status?.success() { return Err(io::Error::other("Launch Services could not start the control app")); }
                listener.accept().await?
            }
        };
        verify_peer(&channel, hash)?;
        Ok(channel)
    }).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "control app launch timed out"))?
}

pub async fn connect_to_launcher(rendezvous: &Path) -> io::Result<UnixStream> {
    let executable = std::env::current_exe()?;
    let plist = app_bundle(&executable)?.join("Contents/Info.plist");
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :ShiJuGatewayCDHash"])
        .arg(plist)
        .kill_on_drop(true)
        .output()
        .await?;
    if !output.status.success() {
        return Err(io::Error::other("missing bundled gateway identity"));
    }
    let value = String::from_utf8_lossy(&output.stdout);
    let hash = code_hash(&value)?;
    // The pin must be part of this running app's signed Info.plist.
    let code = unsafe {
        let mut code = ptr::null();
        check_os_status(SecCodeCopySelf(0, &mut code))?;
        Owned::new(code)?
    };
    verify(&code, &format!("info[ShiJuGatewayCDHash] = \"{hash}\""))?;
    drop(code);
    let channel = UnixStream::connect(rendezvous).await?;
    verify_peer(&channel, hash)?;
    Ok(channel)
}
