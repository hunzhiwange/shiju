//! An independent updater survives shutdown and waits until application files are released.

use crate::app::installer_update::{PreparedDesktopUpdate, verify_ready};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(target_os = "macos")]
#[path = "desktop_update_macos.rs"]
mod macos;

#[cfg(windows)]
#[path = "desktop_update_windows.rs"]
mod windows;

#[cfg(any(test, target_os = "linux"))]
#[path = "desktop_update_linux.rs"]
mod linux;

const HELPER_ARG: &str = "--desktop-update-helper";

#[derive(Serialize, Deserialize)]
struct UpdateRequest {
    #[serde(deserialize_with = "crate::app::installer_update::deserialize_handoff")]
    ready: PreparedDesktopUpdate,
    executable: PathBuf,
    parent_pid: u32,
    macos_stage: Option<PathBuf>,
}

/// Start a copy outside the installation tree so Windows can release the old executable.
/// The original app exits only after the helper acknowledges a validated request.
pub(crate) fn start(ready: &PreparedDesktopUpdate) -> Result<()> {
    verify_ready(ready)?;
    let executable = std::env::current_exe()?.canonicalize()?;
    preflight(&executable, ready)?;
    let job = tempfile::Builder::new()
        .prefix("updater-")
        .tempdir_in(ready.path().parent().context("安装包目录缺失")?)?;
    #[cfg(target_os = "macos")]
    let stage = macos::prepare(ready.path(), &executable, ready.version())?;
    let request = UpdateRequest {
        ready: ready.clone(),
        executable: executable.clone(),
        parent_pid: std::process::id(),
        #[cfg(target_os = "macos")]
        macos_stage: Some(stage.path().to_owned()),
        #[cfg(not(target_os = "macos"))]
        macos_stage: None,
    };
    // Keep the helper unprivileged; system installers request authorization.
    let helper = job.path().join(if cfg!(windows) { "shi-ju-helper.exe" } else { "updater" });
    std::fs::copy(executable, &helper).context("复制独立更新进程失败")?;
    let log = std::fs::File::create(job.path().join("update.log"))?;
    let mut child = Command::new(&helper)
        .arg(HELPER_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .spawn()
        .context("启动独立更新进程失败")?;
    let handshake = (|| -> Result<()> {
        let mut input = child.stdin.take().context("更新进程输入不可用")?;
        serde_json::to_writer(&mut input, &request)?;
        drop(input);
        let output = child.stdout.take().context("更新进程输出不可用")?;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(output).read_line(&mut line).map(|_| line);
            let _ = tx.send(result);
        });
        let response = rx
            .recv_timeout(Duration::from_secs(30))
            .context("更新进程未就绪，当前应用保持运行")??;
        if response.trim() != "ready" {
            let status = child.try_wait().context("无法读取更新进程状态")?;
            let log = std::fs::read_to_string(job.path().join("update.log"))?;
            bail!("更新进程未确认就绪，进程状态：{status:?}，诊断：{}", log.trim());
        }
        Ok(())
    })();
    if let Err(error) = handshake {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    // Owned by the child from this point; do not delete its executable or staged bundle.
    let _ = job.keep();
    #[cfg(target_os = "macos")]
    let _ = stage.keep();
    Ok(())
}

/// Run before GUI/tracing initialization; ordinary launches never enter this path.
pub fn run_if_requested() -> Option<Result<()>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_none_or(|arg| arg != HELPER_ARG) {
        return None;
    }
    Some((|| {
        if args.len() != 1 {
            bail!("更新进程参数无效");
        }
        let mut input = String::new();
        std::io::stdin().take(64 * 1024).read_to_string(&mut input)?;
        let request: UpdateRequest = serde_json::from_str(&input)?;
        verify_ready(&request.ready)?;
        preflight(&request.executable, &request.ready)?;
        let parent = ParentProcess::open(request.parent_pid)?;
        println!("ready");
        std::io::stdout().flush()?;
        let result = parent.wait().and_then(|()| install(&request));
        match result {
            Ok(()) => {
                let answer = rfd::MessageDialog::new()
                    .set_title("ShiJu 更新完成")
                    .set_description("升级已完成，是否立即启动新版？")
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if answer == rfd::MessageDialogResult::Yes {
                    if let Err(error) = launch(&request.executable) {
                        #[cfg(target_os = "macos")]
                        macos::rollback(&request)?;
                        rfd::MessageDialog::new()
                            .set_title("ShiJu 启动失败")
                            .set_description(format!("{error:#}\n请从应用安装目录重新启动。"))
                            .set_level(rfd::MessageLevel::Error)
                            .show();
                        return Err(error.context("启动新版失败"));
                    }
                }
                #[cfg(target_os = "macos")]
                macos::cleanup(&request)?;
            }
            Err(error) => {
                eprintln!("更新失败：{error:#}");
                rfd::MessageDialog::new()
                    .set_title("ShiJu 更新失败")
                    .set_description(format!("{error:#}\n安装包已保留，可重新启动应用后重试。"))
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                return Err(error);
            }
        }
        Ok(())
    })())
}

fn launch(executable: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::launch(executable);
    #[cfg(not(target_os = "macos"))]
    {
        Command::new(executable).spawn().context("无法启动应用")?;
        Ok(())
    }
}

fn preflight(executable: &Path, ready: &PreparedDesktopUpdate) -> Result<()> {
    if executable.canonicalize()? != executable || !executable.is_file() {
        bail!("当前应用路径无效");
    }
    #[cfg(target_os = "macos")]
    {
        if ready.kind() != vw_shared::update::InstallerKind::Dmg {
            bail!("macOS 更新需要 DMG");
        }
        macos::bundle(executable)?;
    }
    #[cfg(target_os = "linux")]
    linux::preflight(executable, ready)?;
    #[cfg(target_os = "windows")]
    {
        if ready.kind() != vw_shared::update::InstallerKind::Msi {
            bail!("Windows 更新需要 MSI");
        }
        windows::Package::inspect(windows_installer_path(ready.path())?, ready.version())?;
        if !msiexec()?.is_file() {
            bail!("Windows Installer 不可用");
        }
        let supported = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
            .into_iter()
            .filter_map(std::env::var_os)
            .map(|root| PathBuf::from(root).join("ShiJu/shi-ju.exe"))
            .filter_map(|path| path.canonicalize().ok())
            .any(|path| path == executable);
        if !supported {
            bail!("此 MSI 只能更新 Program Files 中的 ShiJu，请先通过 MSI 安装后重试");
        }
    }
    Ok(())
}

fn install(request: &UpdateRequest) -> Result<()> {
    verify_ready(&request.ready)?;
    #[cfg(target_os = "macos")]
    return macos::install(request);
    #[cfg(target_os = "windows")]
    {
        let installer = windows_installer_path(request.ready.path())?;
        let package = windows::Package::inspect(installer, request.ready.version())?;
        let log = installer.with_file_name("installer.log");
        windows::stop_installed_processes(&request.executable)?;
        eprintln!(
            "Windows 更新器 {}，目标版本 {}，安装日志：{}",
            env!("CARGO_PKG_VERSION"),
            request.ready.version(),
            log.display()
        );
        let status =
            windows_installer_command(&msiexec()?, installer, &log, package.is_installed())
                .status()
                .context("Windows Installer 启动失败")?;
        windows_installation_result(status.code(), installer, &log)?;
        package
            .verify_installed(&request.executable)
            .with_context(|| format!("安装日志：{}", log.display()))
    }
    #[cfg(target_os = "linux")]
    {
        linux::install(&request.executable, &request.ready)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    bail!("当前平台不支持自动更新")
}

#[cfg(target_os = "windows")]
fn windows_installer_path(verified: &Path) -> Result<&Path> {
    // canonicalize() produces \\?\ paths, which Windows Installer cannot open.
    // Only simplify when DOS path semantics still refer to the verified file.
    let path = dunce::simplified(verified);
    if !path.is_absolute()
        || matches!(
            path.components().next(),
            Some(std::path::Component::Prefix(prefix)) if prefix.kind().is_verbatim()
        )
    {
        bail!("安装包路径无法转换为 Windows Installer 支持的格式，请使用较短的本地缓存路径后重试");
    }
    if path.canonicalize()? != verified.canonicalize()? {
        bail!("安装包路径转换后指向不同文件，已取消更新");
    }
    Ok(path)
}

#[cfg(any(test, target_os = "windows"))]
fn windows_installer_command(
    msiexec: &Path,
    installer: &Path,
    log: &Path,
    already_installed: bool,
) -> Command {
    let mut command = Command::new(msiexec);
    command
        .arg("/i")
        .arg(installer)
        // Let Windows Installer release daemon/helper file locks. Restarting the
        // updated application remains an explicit choice in our completion dialog.
        .args(["/passive", "/norestart", "REBOOT=ReallySuppress", "MSIDISABLERMRESTART=1"])
        .arg("/L*V")
        .arg(log);
    // /i on an already registered ProductCode enters maintenance mode and can
    // retain old unversioned executables. Re-cache this verified package and
    // replace its files, while leaving first installs/major upgrades untouched.
    if already_installed {
        command.args(["REINSTALL=ALL", "REINSTALLMODE=vamus"]);
    }
    command
}

#[cfg(any(test, target_os = "windows"))]
fn windows_installation_result(code: Option<i32>, installer: &Path, log: &Path) -> Result<()> {
    installation_result("windows", code)
        .with_context(|| format!("安装包：{}\n安装日志：{}", installer.display(), log.display()))
}

#[cfg(any(test, target_os = "windows"))]
fn installation_result(os: &str, code: Option<i32>) -> Result<()> {
    match (os, code) {
        (_, Some(0)) => Ok(()),
        ("windows", Some(3010 | 1641)) => {
            bail!("Windows 要求重启系统才能完成更新，请先保存工作并重启系统")
        }
        ("windows", Some(1603)) => {
            bail!("Windows Installer 安装失败（1603），请查看安装日志中的具体失败原因")
        }
        (_, code) => bail!("系统安装未成功完成，退出码：{code:?}"),
    }
}

#[cfg(target_os = "windows")]
fn msiexec() -> Result<PathBuf> {
    Ok(PathBuf::from(std::env::var_os("SystemRoot").context("SystemRoot 缺失")?)
        .join("System32/msiexec.exe"))
}

#[cfg(target_os = "macos")]
fn command_output(command: &mut Command) -> Result<String> {
    let output = command.output().context("系统更新命令启动失败")?;
    if !output.status.success() {
        bail!("系统更新命令失败：{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[cfg(unix)]
struct ParentProcess(u32);

#[cfg(unix)]
impl ParentProcess {
    fn open(pid: u32) -> Result<Self> {
        // getppid binds the request to the actual spawning application, avoiding PID reuse.
        if pid <= 1 || unsafe { libc::getppid() } as u32 != pid {
            bail!("更新请求不属于父应用进程");
        }
        Ok(Self(pid))
    }

    fn wait(self) -> Result<()> {
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while unsafe { libc::getppid() } as u32 == self.0 {
            if std::time::Instant::now() >= deadline {
                bail!("旧应用尚未退出，已取消更新以避免文件占用");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
}

#[cfg(windows)]
struct ParentProcess(*mut std::ffi::c_void);

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
impl ParentProcess {
    fn open(pid: u32) -> Result<Self> {
        let handle = unsafe { OpenProcess(0x00100000, 0, pid) }; // SYNCHRONIZE
        if handle.is_null() {
            return Err(std::io::Error::last_os_error()).context("无法等待旧应用退出");
        }
        Ok(Self(handle))
    }

    fn wait(self) -> Result<()> {
        if unsafe { WaitForSingleObject(self.0, 60_000) } != 0 {
            bail!("旧应用尚未退出，已取消更新以避免文件占用");
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for ParentProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
#[path = "desktop_update_helper_tests.rs"]
mod tests;
