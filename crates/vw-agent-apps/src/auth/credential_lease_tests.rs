//! A concurrent fork must not extend a released owner's credential lease.
use super::credential_file::{AccountCredential, CredentialFileStore};
use std::{
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::PermissionsExt, net::UnixStream, process::CommandExt},
    },
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};
#[test]
fn cc_apps_credential_lease_release_does_not_wait_for_unrelated_exec() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    store
        .save(
            &tenant,
            "host",
            AppId::Claude,
            "selected",
            &AccountCredential::ClaudeToken("fixture-token".into()),
        )
        .unwrap();
    let lease = store.lease(&tenant, "host", AppId::Claude, "selected", false).unwrap();
    let (mut parent, child) = UnixStream::pair().unwrap();
    parent.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
    let process = std::thread::spawn(move || {
        let fd = child.as_raw_fd();
        let mut command = std::process::Command::new("/usr/bin/true");
        // SAFETY: only async-signal-safe read/write occur before exec, and fd remains live.
        unsafe {
            command.pre_exec(move || {
                let ready = b'R';
                let mut go = 0u8;
                if libc::write(fd, (&ready as *const u8).cast(), 1) != 1
                    || libc::read(fd, (&mut go as *mut u8).cast(), 1) != 1
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let result = command.status();
        drop(child);
        result
    });
    let mut ready = [0];
    parent.read_exact(&mut ready).unwrap();
    assert_eq!(ready, *b"R");
    drop(lease);
    let next = store.lease(&tenant, "host", AppId::Claude, "selected", false);
    // Release the child before asserting, including when running the red implementation.
    parent.write_all(b"G").unwrap();
    assert!(process.join().unwrap().unwrap().success());
    assert!(
        next.is_ok(),
        "the owner's release must unlock despite a concurrent pre-exec descriptor copy"
    );
}
