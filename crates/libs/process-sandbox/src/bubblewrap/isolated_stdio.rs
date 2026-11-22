//! Bubblewrap policy for a single reviewed GNU ELF communicating over stdio.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

const BWRAP: &str = "/usr/bin/bwrap";
const ENTRY: &str = "/app/entry";
#[cfg(target_arch = "x86_64")]
const LOADER: &str = "/lib64/ld-linux-x86-64.so.2";
#[cfg(target_arch = "aarch64")]
const LOADER: &str = "/lib/ld-linux-aarch64.so.1";
#[cfg(target_arch = "x86_64")]
const LIB_DIRS: &[&str] = &[
    "/lib/x86_64-linux-gnu",
    "/usr/lib/x86_64-linux-gnu",
    "/lib64",
    "/usr/lib64",
    "/lib",
    "/usr/lib",
];
#[cfg(target_arch = "aarch64")]
const LIB_DIRS: &[&str] = &[
    "/lib/aarch64-linux-gnu",
    "/usr/lib/aarch64-linux-gnu",
    "/lib64",
    "/usr/lib64",
    "/lib",
    "/usr/lib",
];

/// Prepare a fail-closed command for an absolute, caller-selected native ELF.
/// The caller owns the executable's provenance. The sandbox receives a read-only
/// copy of the entry and individual system runtime files. It gets no host paths,
/// environment overlay or file descriptors beyond the three private stdio pipes.
pub fn isolated_stdio_command(executable: &Path, args: &[OsString]) -> io::Result<Command> {
    if !executable.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "isolated stdio requires an absolute executable",
        ));
    }
    let mut entry = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(executable.canonicalize()?)?;
    let metadata = entry.metadata()?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "isolated stdio requires a regular executable",
        ));
    }
    let mut magic = [0; 4];
    entry.read_exact(&mut magic)?;
    if magic != *b"\x7fELF" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "isolated stdio requires a native ELF",
        ));
    }
    entry.rewind()?;

    let bwrap = fs::metadata(BWRAP)?;
    if !bwrap.is_file() || bwrap.uid() != 0 || bwrap.mode() & 0o6022 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "isolated stdio requires a trusted non-setuid Bubblewrap",
        ));
    }
    let filter = stdio_filter()?;
    let entry_fd = entry.as_raw_fd();
    let filter_fd = filter.as_raw_fd();
    if entry_fd < 3 || filter_fd < 3 {
        return Err(io::Error::other(
            "isolated stdio requires descriptors 0 through 2 on the host",
        ));
    }

    let mut command = Command::new(BWRAP);
    command.current_dir("/").env_clear().args([
        "--unshare-user",
        "--disable-userns",
        "--unshare-pid",
        "--unshare-net",
        "--unshare-ipc",
        "--unshare-uts",
        "--die-with-parent",
        "--cap-drop",
        "ALL",
        "--clearenv",
        "--tmpfs",
        "/tmp",
        "--perms",
        "0700",
        "--dir",
        "/tmp/work",
        "--perms",
        "0700",
        "--dir",
        "/tmp/home",
        "--chdir",
        "/tmp/work",
        "--setenv",
        "HOME",
        "/tmp/home",
        "--setenv",
        "TMPDIR",
        "/tmp",
        "--setenv",
        "LD_LIBRARY_PATH",
        "/runtime",
        "--perms",
        "0555",
        "--ro-bind-data",
    ]);
    command.arg(entry_fd.to_string()).arg(ENTRY);
    // Bind individual system files only. Extra package libraries and interpreters
    // must not be taken from the host search path or from the host's environment.
    if trusted_library(Path::new(LOADER)) {
        command.args(["--ro-bind", LOADER, LOADER]);
    }
    for name in
        ["libc.so.6", "libm.so.6", "libgcc_s.so.1", "libpthread.so.0", "libdl.so.2", "librt.so.1"]
    {
        if let Some(source) =
            LIB_DIRS.iter().map(|dir| Path::new(dir).join(name)).find(|path| trusted_library(path))
        {
            command.arg("--ro-bind").arg(source).arg(format!("/runtime/{name}"));
        }
    }
    command
        .arg("--seccomp")
        .arg(filter_fd.to_string())
        .args(["--", ENTRY])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // CLOEXEC preserves Rust's exec-error pipe until exec, then prevents leaking any
    // inherited descriptor into Bubblewrap except its two explicit policy inputs.
    let parent = std::process::id() as libc::pid_t;
    unsafe {
        command.pre_exec(move || {
            // Put setup in its own group before Bubblewrap can fork. The host
            // can cancel it even before the namespace's parent-death hook.
            if libc::setsid() < 0
                || libc::prctl(
                    libc::PR_SET_PDEATHSIG,
                    libc::SIGKILL as libc::c_ulong,
                    0 as libc::c_ulong,
                    0 as libc::c_ulong,
                    0 as libc::c_ulong,
                ) < 0
            {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(libc::ECHILD));
            }
            let inputs = [entry.as_raw_fd(), filter.as_raw_fd()];
            if libc::syscall(libc::SYS_close_range, 3_u32, u32::MAX, libc::CLOSE_RANGE_CLOEXEC) < 0
            {
                return Err(io::Error::last_os_error());
            }
            for fd in inputs {
                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(command)
}

fn trusted_library(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.uid() == 0 && metadata.mode() & 0o6022 == 0
    })
}

fn stdio_filter() -> io::Result<File> {
    // Linux audit ABI identifiers for the two supported little-endian 64-bit targets.
    #[cfg(target_arch = "x86_64")]
    const AUDIT_ARCH: u32 = 0xc000_003e;
    #[cfg(target_arch = "aarch64")]
    const AUDIT_ARCH: u32 = 0xc000_00b7;

    let load = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
    let equal = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
    let ret = (libc::BPF_RET | libc::BPF_K) as u16;
    let kill = libc::SECCOMP_RET_KILL_PROCESS;
    let deny = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
    // Classic BPF instructions use native byte order. seccomp_data starts
    // with the 32-bit syscall number followed by its 32-bit audit ABI.
    let mut bytes = Vec::new();
    let mut emit = |code: u16, jt: u8, jf: u8, k: u32| {
        bytes.extend(code.to_ne_bytes());
        bytes.extend([jt, jf]);
        bytes.extend(k.to_ne_bytes());
    };
    emit(load, 0, 0, 4);
    emit(equal, 1, 0, AUDIT_ARCH);
    emit(ret, 0, 0, kill);
    emit(load, 0, 0, 0);
    #[cfg(target_arch = "x86_64")]
    {
        // x32 uses the same audit architecture with bit 30 set on syscalls.
        emit((libc::BPF_JMP | libc::BPF_JSET | libc::BPF_K) as u16, 0, 1, 1 << 30);
        emit(ret, 0, 0, kill);
    }
    for syscall in [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_io_uring_setup,
        libc::SYS_io_uring_enter,
        libc::SYS_io_uring_register,
        libc::SYS_keyctl,
        libc::SYS_add_key,
        libc::SYS_request_key,
    ] {
        emit(equal, 0, 1, syscall as u32);
        emit(ret, 0, 0, deny);
    }
    emit(ret, 0, 0, libc::SECCOMP_RET_ALLOW);

    let fd = unsafe { libc::memfd_create(c"acp-stdio-seccomp".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut filter = unsafe { File::from_raw_fd(fd) };
    filter.write_all(&bytes)?;
    filter.rewind()?;
    Ok(filter)
}
