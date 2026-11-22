use super::*;

#[test]
fn acp_platform_registry_contract_selects_fixed_target_artifacts() {
    for agent in ["omp", "stakpak", "vtcode"] {
        let x64 = contract(agent, "x86_64").unwrap().unwrap();
        let arm64 = contract(agent, "aarch64").unwrap().unwrap();
        assert_ne!(x64.url, arm64.url, "{agent}");
        assert_ne!(x64.executable_sha256, arm64.executable_sha256, "{agent}");
        assert_eq!(x64.args, ["acp"], "{agent}");
        assert_eq!(arm64.args, x64.args, "{agent}");
        assert!(contract(agent, "riscv64").unwrap().is_none(), "{agent}");
        assert!(config_acp_install::acp_install_supported(agent), "{agent}");
    }
    let corust = contract("corust-agent", "x86_64").unwrap().unwrap();
    assert_eq!(corust.bin, "corust-agent-acp");
    assert!(corust.args.is_empty());
    assert!(contract("corust-agent", "aarch64").unwrap().is_none());
    assert_eq!(
        config_acp_install::acp_install_supported("corust-agent"),
        cfg!(target_arch = "x86_64"),
    );
    assert!(contract("unknown-agent", "x86_64").unwrap().is_none());
}

#[test]
fn acp_platform_registry_contract_rejects_unreviewed_installed_entries() {
    for agent in ["omp", "stakpak", "vtcode", "corust-agent"] {
        let Some(artifact) = contract(agent, std::env::consts::ARCH).unwrap() else {
            continue;
        };
        let root = tempfile::tempdir().unwrap();
        assert!(discover(agent, root.path()).unwrap().is_none());
        let directory = root.path().join(artifact.directory);
        fs::create_dir(&directory).unwrap();
        let entry = directory.join(artifact.bin);
        fs::write(&entry, b"wrong artifact").unwrap();
        fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(discover(agent, root.path()).is_err(), "{agent}");
        assert!(verify_launch(agent, root.path(), &entry, &artifact.args).is_err(), "{agent}");
        assert!(
            crate::verify_reviewed_agent_executable(agent, entry.to_str().unwrap()).is_err(),
            "{agent}",
        );
    }
}
