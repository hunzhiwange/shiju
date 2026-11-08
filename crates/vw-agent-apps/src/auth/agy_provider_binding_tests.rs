use super::*;
use crate::auth::agy_account::{checkout_account_in, with_provider_account_in};
use std::os::fd::AsRawFd;

#[test]
fn provider_agy_binding_checks_native_reference_and_holds_the_slot() {
    let mut f = Fixture::new();
    let saved = credential("official", 1);
    let account = f.managed("official", &saved, "ready");
    let scope = scope(&f.tenant);
    let native = f.target(Some(scope));
    let check_locked = || {
        let probe = std::fs::File::open(f.home.join("Library/Keychains")).unwrap();
        // SAFETY: probe owns a live file descriptor and LOCK_NB never waits.
        assert_eq!(unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, -1);
    };
    assert_eq!(
        with_provider_account_in(scope, Some(&account), native, &f.entry(), true, |active| {
            check_locked();
            Ok(active)
        }),
        Ok(false)
    );
    assert!(matches!(
        checkout_account_in(scope, &account, native, &f.entry(), || Err(AuthErrorCode::Conflict)),
        Err(AuthErrorCode::Conflict)
    ));
    assert_eq!(f.entry().read().unwrap(), None);
    let lease = checkout_account_in(scope, &account, native, &f.entry(), || Ok(())).unwrap();
    check_locked();
    assert_eq!(f.entry().read().unwrap(), Some(encode(&saved).into_bytes()));
    drop(lease);
    assert_eq!(
        with_provider_account_in(scope, Some(&account), native, &f.entry(), true, |active| {
            check_locked();
            Ok(active)
        }),
        Ok(true)
    );
    assert_eq!(
        with_provider_account_in(scope, Some(&account), f.target(None), &f.entry(), true, |_| Ok(
            ()
        )),
        Err(AuthErrorCode::Forbidden)
    );
    f.remove_entries();
}
