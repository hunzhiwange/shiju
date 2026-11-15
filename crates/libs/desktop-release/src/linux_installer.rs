//! Resolve native package formats without executing distribution-provided shell code.
use super::InstallerKind;
use anyhow::{Context, Result, bail};

pub fn linux_installer_kind() -> Result<InstallerKind> {
    let release = match std::fs::read_to_string("/etc/os-release") {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::read_to_string("/usr/lib/os-release").context("无法读取 Linux 发行版信息")?
        }
        Err(error) => return Err(error).context("无法读取 Linux 发行版信息"),
    };
    kind_from_os_release(&release)
}

fn kind_from_os_release(release: &str) -> Result<InstallerKind> {
    let field = |key: &str| {
        release
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once('=')?;
                (name == key).then(|| value.trim().trim_matches(['\'', '"']))
            })
            .unwrap_or_default()
    };
    let classify = |id| match id {
        "ubuntu" | "debian" => Some(InstallerKind::Deb),
        "fedora"
        | "rhel"
        | "centos"
        | "rocky"
        | "almalinux"
        | "suse"
        | "opensuse"
        | "opensuse-leap"
        | "opensuse-tumbleweed"
        | "sles"
        | "ol"
        | "amzn" => Some(InstallerKind::Rpm),
        _ => None,
    };
    if let Some(kind) = classify(field("ID")) {
        return Ok(kind);
    }
    let mut selected = None;
    for id in field("ID_LIKE").split_whitespace() {
        if let Some(kind) = classify(id) {
            if selected.is_some_and(|other| other != kind) {
                bail!("发行版包格式不明确，拒绝自动安装");
            }
            selected = Some(kind);
        }
    }
    selected.context("此 Linux 发行版不支持 DEB/RPM 自动更新")
}

#[cfg(test)]
#[path = "linux_installer_tests.rs"]
mod tests;
