//! Windows equivalent of a user-owned 0700 ACP directory.
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const PRIVATE_DIRECTORY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$directory = [IO.DirectoryInfo]::new($env:SHIJU_ACP_PRIVATE_DIRECTORY)
for ($ancestor = $directory.Parent; $null -ne $ancestor; $ancestor = $ancestor.Parent) {
    if (-not $ancestor.Exists -or ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Invalid ACP directory ancestor'
    }
}
$user = [Security.Principal.WindowsIdentity]::GetCurrent().User
$inheritance = [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit'
$rule = [Security.AccessControl.FileSystemAccessRule]::new(
    $user, 'FullControl', $inheritance, 'None', 'Allow'
)
$acl = [Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($user)
$acl.SetAccessRuleProtection($true, $false)
$acl.AddAccessRule($rule)
if ($env:SHIJU_ACP_NEW_TEMPORARY_DIRECTORY -eq '1') {
    if (-not $directory.Exists -or ($directory.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Invalid ACP temporary directory'
    }
    $directory.SetAccessControl($acl)
} elseif (-not $directory.Exists) {
    # On Windows PowerShell/.NET Framework this applies the DACL during creation.
    $directory.Create($acl)
}
$directory.Refresh()
if (-not $directory.Exists -or ($directory.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw 'Invalid ACP private directory'
}
$actual = $directory.GetAccessControl()
$rules = @($actual.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
if (-not $actual.AreAccessRulesProtected -or
    $actual.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $user.Value -or
    $rules.Count -ne 1 -or
    $rules[0].IdentityReference.Value -ne $user.Value -or
    $rules[0].AccessControlType -ne 'Allow' -or
    $rules[0].FileSystemRights -ne 'FullControl' -or
    $rules[0].InheritanceFlags -ne $inheritance -or
    $rules[0].PropagationFlags -ne 'None') {
    throw 'Insecure ACP directory ACL'
}
"#;

pub(super) fn create_or_verify(path: &Path) -> anyhow::Result<()> {
    secure_directory(path, false)
}

pub(super) fn secure_new_temporary_directory(path: &Path) -> anyhow::Result<()> {
    secure_directory(path, true)
}

fn secure_directory(path: &Path, new_temporary_directory: bool) -> anyhow::Result<()> {
    anyhow::ensure!(path.is_absolute(), "ACP private directory must be absolute");
    let system_root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or_else(|| anyhow::anyhow!("Missing Windows system directory"))?;
    let status = Command::new(system_root.join("System32/WindowsPowerShell/v1.0/powershell.exe"))
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", PRIVATE_DIRECTORY_SCRIPT])
        .env("SHIJU_ACP_PRIVATE_DIRECTORY", path)
        .env("SHIJU_ACP_NEW_TEMPORARY_DIRECTORY", if new_temporary_directory { "1" } else { "0" })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    anyhow::ensure!(status.success(), "Unable to establish a private Windows ACP directory ACL");
    Ok(())
}
