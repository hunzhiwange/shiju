use super::*;

#[test]
fn linux_installer_resolves_native_families_and_derivatives() {
    for (release, expected) in [
        ("ID=ubuntu\nID_LIKE=debian", InstallerKind::Deb),
        ("ID=debian", InstallerKind::Deb),
        ("ID=linuxmint\nID_LIKE=\"ubuntu debian\"", InstallerKind::Deb),
        ("ID=fedora", InstallerKind::Rpm),
        ("ID=rocky\nID_LIKE=\"rhel centos fedora\"", InstallerKind::Rpm),
        ("ID=opensuse-tumbleweed", InstallerKind::Rpm),
        ("ID=custom\nID_LIKE='suse'", InstallerKind::Rpm),
    ] {
        assert_eq!(kind_from_os_release(release).unwrap(), expected);
    }
}

#[test]
fn linux_installer_rejects_unknown_and_ambiguous_systems() {
    for release in
        ["", "ID=arch", "ID=alpine", "ID=custom\nID_LIKE=\"debian fedora\"", "ID=$(echo ubuntu)"]
    {
        assert!(kind_from_os_release(release).is_err(), "{release}");
    }
}
