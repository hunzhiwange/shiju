//! Native package updates. Only the package manager runs under system authorization.
use crate::app::installer_update::{PreparedDesktopUpdate, verify_ready};
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};
use vw_shared::update::InstallerKind;

pub(super) fn preflight(executable: &Path, ready: &PreparedDesktopUpdate) -> Result<()> {
    if executable != Path::new("/usr/bin/shi-ju") {
        bail!("自动更新仅支持系统包安装的 /usr/bin/shi-ju");
    }
    let kind = vw_shared::update::desktop_release::linux_installer_kind()?;
    if ready.kind() != kind {
        bail!("安装包格式与当前 Linux 发行版不一致");
    }
    for program in match kind {
        InstallerKind::Deb => {
            &["/usr/bin/pkexec", "/usr/bin/apt-get", "/usr/bin/dpkg-query", "/usr/bin/dpkg-deb"][..]
        }
        InstallerKind::Rpm => &["/usr/bin/pkexec", "/usr/bin/rpm"][..],
        _ => bail!("Linux 更新只支持 DEB/RPM"),
    } {
        if !Path::new(program).is_file() {
            bail!("系统自动更新缺少必需程序：{program}");
        }
    }
    inspect(kind, executable, ready.path(), ready.version(), &mut output)?;
    Ok(())
}

fn output(command: &mut Command) -> Result<String> {
    let output = command.env("LC_ALL", "C").output().context("无法查询系统安装包")?;
    if !output.status.success() {
        bail!("安装包查询失败：{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn architecture(kind: InstallerKind, arch: &str) -> Result<&str> {
    match (kind, arch) {
        (InstallerKind::Deb, "x86_64") => Ok("amd64"),
        (InstallerKind::Deb, "aarch64") => Ok("arm64"),
        (InstallerKind::Rpm, "x86_64" | "aarch64") => Ok(arch),
        _ => bail!("不支持的 Linux 安装架构：{arch}"),
    }
}

fn inspect(
    kind: InstallerKind,
    executable: &Path,
    package: &Path,
    version: &str,
    run: &mut impl FnMut(&mut Command) -> Result<String>,
) -> Result<()> {
    let arch = architecture(kind, std::env::consts::ARCH)?;
    let (owner, metadata, expected_owner) = match kind {
        InstallerKind::Deb => (
            run(Command::new("/usr/bin/dpkg-query").arg("-S").arg(executable))?,
            run(Command::new("/usr/bin/dpkg-deb")
                .args(["-W", "--showformat=${Package}\n${Version}\n${Architecture}"])
                .arg(package))?,
            format!("shiju: {}", executable.display()),
        ),
        InstallerKind::Rpm => (
            run(Command::new("/usr/bin/rpm")
                .args(["-qf", "--queryformat", "%{NAME}"])
                .arg(executable))?,
            run(Command::new("/usr/bin/rpm")
                .args(["-qp", "--queryformat", "%{NAME}\n%{VERSION}\n%{ARCH}"])
                .arg(package))?,
            "shiju".into(),
        ),
        _ => bail!("Linux 更新只支持 DEB/RPM"),
    };
    if owner != expected_owner {
        bail!("当前应用不属于 shiju 系统包，拒绝覆盖其它安装");
    }
    if metadata != format!("shiju\n{version}\n{arch}") {
        bail!("安装包名称、版本或架构与发布快照不一致");
    }
    Ok(())
}

fn install_command(kind: InstallerKind, package: &Path) -> Result<Command> {
    let mut command = Command::new("/usr/bin/pkexec");
    // Do not fall back to terminal password entry in a detached GUI helper.
    command.arg("--disable-internal-agent");
    match kind {
        InstallerKind::Deb => {
            command.args(["/usr/bin/apt-get", "--yes", "--no-remove", "install", "--"]);
        }
        InstallerKind::Rpm => {
            command.args(["/usr/bin/rpm", "-U", "--"]);
        }
        _ => bail!("Linux 更新只支持 DEB/RPM"),
    }
    command.arg(package).stdin(Stdio::null());
    Ok(command)
}

fn installation_result(code: Option<i32>) -> Result<()> {
    match code {
        Some(0) => Ok(()),
        Some(126) => bail!("已取消系统安装授权；安装包已保留，可重新启动应用后重试"),
        Some(127) => bail!("无法获得系统安装授权，请检查桌面的 polkit 授权服务"),
        _ => bail!("系统安装失败，退出码：{code:?}；请检查更新日志、依赖或软件包管理器锁后重试"),
    }
}

pub(super) fn install(executable: &Path, ready: &PreparedDesktopUpdate) -> Result<()> {
    preflight(executable, ready)?;
    verify_ready(ready)?;
    let log_path = ready.path().with_file_name("installer.log");
    let log = std::fs::File::create(&log_path)?;
    let status = install_command(ready.kind(), ready.path())?
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status()
        .context("无法启动系统安装授权")?;
    installation_result(status.code())
        .with_context(|| format!("安装日志：{}", log_path.display()))?;
    let installed = match ready.kind() {
        InstallerKind::Deb => output(Command::new("/usr/bin/dpkg-query").args([
            "-W",
            "--showformat=${Status}\n${Version}",
            "shiju",
        ]))?,
        InstallerKind::Rpm => output(Command::new("/usr/bin/rpm").args([
            "-q",
            "--queryformat",
            "%{VERSION}",
            "shiju",
        ]))?,
        _ => bail!("Linux 更新只支持 DEB/RPM"),
    };
    let expected = if ready.kind() == InstallerKind::Deb {
        format!("install ok installed\n{}", ready.version())
    } else {
        ready.version().to_owned()
    };
    if installed != expected || !executable.is_file() {
        bail!("安装后的版本或安装状态与目标不一致，拒绝报告升级成功");
    }
    Ok(())
}

#[cfg(test)]
#[path = "desktop_update_linux_tests.rs"]
mod tests;
