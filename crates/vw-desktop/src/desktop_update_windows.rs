//! Read MSI identity before shutdown and verify the installed payload before reporting success.

use anyhow::{Context, Result, bail};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
    INSTALLSTATE_ABSENT, INSTALLSTATE_ADVERTISED, INSTALLSTATE_DEFAULT, INSTALLSTATE_UNKNOWN,
    MSIDBOPEN_READONLY, MSIFILEHASHINFO, MsiCloseHandle, MsiDatabaseOpenViewW, MsiGetFileHashW,
    MsiOpenDatabaseW, MsiQueryProductStateW, MsiRecordGetInteger, MsiRecordGetStringW,
    MsiRecordIsNull, MsiViewExecute, MsiViewFetch,
};

// File identifiers and installed names from release/windows/vibewindow.wxs.
const BUNDLED_FILES: [(&str, &str); 5] = [
    ("DesktopExecutableFile", "shi-ju.exe"),
    ("WebviewHelperFile", "vw-webview.exe"),
    ("CliExecutableFile", "shiju.exe"),
    ("AcpExecutableFile", "acp.exe"),
    ("ServiceLauncherFile", "Start ShiJu Service.cmd"),
];

pub(super) struct Package {
    installed: bool,
    hashes: [[u32; 4]; 5],
}

impl Package {
    pub(super) fn inspect(installer: &Path, expected_version: &str) -> Result<Self> {
        let path = wide_path(installer)?;
        let mut raw = 0;
        // SAFETY: path is NUL-terminated and output storage is valid; READONLY cannot install.
        check(unsafe { MsiOpenDatabaseW(path.as_ptr(), MSIDBOPEN_READONLY, &mut raw) })
            .context("Windows Installer 无法读取安装包数据库")?;
        let database = MsiHandle(raw);
        let version = property(&database, "ProductVersion")?;
        if version != expected_version {
            bail!("MSI 内部版本 {version} 与目标版本 {expected_version} 不一致，已取消更新");
        }
        if property(&database, "UpgradeCode")? != "{C8512FCC-FA63-4C65-9458-B5363A71E371}" {
            bail!("安装包不属于 ShiJu，已取消更新");
        }
        let product = property(&database, "ProductCode")?;
        let product: Vec<_> = product.encode_utf16().chain(Some(0)).collect();
        // SAFETY: product is a NUL-terminated MSI ProductCode read from the verified package.
        let installed = match unsafe { MsiQueryProductStateW(product.as_ptr()) } {
            INSTALLSTATE_DEFAULT => true,
            INSTALLSTATE_UNKNOWN | INSTALLSTATE_ADVERTISED => false,
            INSTALLSTATE_ABSENT => bail!("此 MSI 已为其他用户安装，请使用原安装账户更新"),
            state => bail!("无法确认 MSI 安装状态：{state}"),
        };
        let mut hashes = [[0; 4]; 5];
        for ((id, _), hash) in BUNDLED_FILES.iter().zip(&mut hashes) {
            let record = first_record(
                &database,
                &format!(
                    "SELECT `HashPart1`, `HashPart2`, `HashPart3`, `HashPart4` \
                     FROM `MsiFileHash` WHERE `File_` = '{id}'"
                ),
            )
            .with_context(|| format!("MSI 缺少 {id} 的文件校验信息"))?;
            for (index, part) in hash.iter_mut().enumerate() {
                let field = index as u32 + 1;
                // SAFETY: record remains open, and this query selects four integer fields.
                if unsafe { MsiRecordIsNull(record.0, field) } != 0 {
                    bail!("MSI 的 {id} 文件校验信息不完整");
                }
                // Preserve all 32 bits, including hash parts represented as signed MSI integers.
                *part = unsafe { MsiRecordGetInteger(record.0, field) } as u32;
            }
        }
        Ok(Self { installed, hashes })
    }

    pub(super) fn is_installed(&self) -> bool {
        self.installed
    }

    pub(super) fn verify_installed(&self, executable: &Path) -> Result<()> {
        let directory = executable.parent().context("应用安装目录缺失")?;
        for ((_, name), expected) in BUNDLED_FILES.iter().zip(&self.hashes) {
            let path = directory.join(name);
            if file_hash(&path)? != *expected {
                bail!(
                    "安装器返回成功，但 {} 未替换为安装包中的文件，升级尚未完成；请关闭仍在运行的 ShiJu 进程后重试",
                    path.display()
                );
            }
        }
        Ok(())
    }
}

struct MsiHandle(u32);

impl Drop for MsiHandle {
    fn drop(&mut self) {
        // SAFETY: this guard owns a handle obtained from a successful MSI call.
        unsafe { MsiCloseHandle(self.0) };
    }
}

fn check(code: u32) -> Result<()> {
    if code != 0 {
        bail!("Windows Installer 错误 {code}：{}", std::io::Error::from_raw_os_error(code as i32));
    }
    Ok(())
}

fn wide_path(path: &Path) -> Result<Vec<u16>> {
    let mut wide: Vec<_> = dunce::simplified(path).as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        bail!("MSI 文件路径包含无效字符");
    }
    wide.push(0);
    Ok(wide)
}

fn first_record(database: &MsiHandle, query: &str) -> Result<MsiHandle> {
    let query: Vec<_> = query.encode_utf16().chain(Some(0)).collect();
    let mut raw = 0;
    // SAFETY: database stays open, query is NUL-terminated, and raw is writable.
    check(unsafe { MsiDatabaseOpenViewW(database.0, query.as_ptr(), &mut raw) })?;
    let view = MsiHandle(raw);
    // SAFETY: view stays open and this query has no parameter record.
    check(unsafe { MsiViewExecute(view.0, 0) })?;
    // SAFETY: raw is writable and the view is positioned at its first row.
    check(unsafe { MsiViewFetch(view.0, &mut raw) })?;
    Ok(MsiHandle(raw))
}

fn property(database: &MsiHandle, name: &str) -> Result<String> {
    let record = first_record(
        database,
        &format!("SELECT `Value` FROM `Property` WHERE `Property` = '{name}'"),
    )?;
    // ProductVersion and GUID properties are all shorter than this bounded buffer.
    let mut buffer = [0; 128];
    let mut length = buffer.len() as u32;
    // SAFETY: record is open and buffer/length refer to valid writable storage.
    check(unsafe { MsiRecordGetStringW(record.0, 1, buffer.as_mut_ptr(), &mut length) })?;
    Ok(String::from_utf16(&buffer[..length as usize])?)
}

fn file_hash(path: &Path) -> Result<[u32; 4]> {
    let wide = wide_path(path)?;
    let mut hash = MSIFILEHASHINFO {
        dwFileHashInfoSize: std::mem::size_of::<MSIFILEHASHINFO>() as u32,
        dwData: [0; 4],
    };
    // SAFETY: wide is NUL-terminated and hash has the size required by the MSI API.
    check(unsafe { MsiGetFileHashW(wide.as_ptr(), 0, &mut hash) })
        .with_context(|| format!("无法校验安装后的文件：{}", path.display()))?;
    Ok(hash.dwData)
}

pub(super) fn stop_installed_processes(executable: &Path) -> Result<()> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    let directory = executable.parent().context("应用安装目录缺失")?.canonicalize()?;
    // SAFETY: this snapshots process metadata without inheriting or modifying any process.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error()).context("无法检查仍在运行的 ShiJu 进程");
    }
    // SAFETY: snapshot creation succeeded and this guard takes ownership of its handle.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: entry has its required size and snapshot remains open during enumeration.
    let mut found = unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) };
    while found != 0 {
        let length =
            entry.szExeFile.iter().position(|&ch| ch == 0).unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
        if entry.th32ProcessID != std::process::id()
            && BUNDLED_FILES.iter().any(|(_, expected)| name.eq_ignore_ascii_case(expected))
        {
            stop_process_in_directory(entry.th32ProcessID, &directory)?;
        }
        // SAFETY: the same entry buffer and snapshot remain valid for the next result.
        found = unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) };
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
        return Err(error).context("枚举 ShiJu 进程失败");
    }
    Ok(())
}

fn stop_process_in_directory(pid: u32, directory: &Path) -> Result<()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
    };

    // Retain one handle for path verification, termination and waiting to avoid PID reuse.
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if raw.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(());
        }
        return Err(error).with_context(|| format!("无法检查进程 {pid}，请退出 ShiJu 后重试"));
    }
    // SAFETY: OpenProcess succeeded and this guard takes ownership of the handle.
    let process = unsafe { OwnedHandle::from_raw_handle(raw) };
    let handle = process.as_raw_handle();
    if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    let mut image = vec![0; 32768];
    let mut length = image.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle, 0, image.as_mut_ptr(), &mut length) } == 0 {
        return Err(std::io::Error::last_os_error()).context("无法确认 ShiJu 进程的完整路径");
    }
    let image = std::path::PathBuf::from(OsString::from_wide(&image[..length as usize]));
    let image = image.canonicalize().context("无法核对运行中程序的安装路径")?;
    let owned_name = image.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
        BUNDLED_FILES.iter().any(|(_, expected)| name.eq_ignore_ascii_case(expected))
    });
    if image.parent() != Some(directory) || !owned_name {
        return Ok(());
    }
    if unsafe { TerminateProcess(handle, 0) } == 0 {
        let error = std::io::Error::last_os_error();
        if unsafe { WaitForSingleObject(handle, 0) } != WAIT_OBJECT_0 {
            return Err(error).with_context(|| format!("无法退出 {}，已取消安装", image.display()));
        }
    }
    if unsafe { WaitForSingleObject(handle, 10_000) } != WAIT_OBJECT_0 {
        bail!("等待 {} 退出失败，已取消安装以避免文件占用", image.display());
    }
    Ok(())
}

#[cfg(test)]
#[path = "desktop_update_windows_tests.rs"]
mod tests;
