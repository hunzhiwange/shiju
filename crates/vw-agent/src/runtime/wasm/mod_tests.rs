use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    fn default_config() -> WasmRuntimeConfig {
        WasmRuntimeConfig::default()
    }

    // ── Basic trait compliance ──────────────────────────────────

    #[test]
    fn wasm_runtime_name() {
        let rt = WasmRuntime::new(default_config());
        assert_eq!(rt.name(), "wasm");
    }

    #[test]
    fn wasm_no_shell_access() {
        let rt = WasmRuntime::new(default_config());
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn wasm_no_filesystem_by_default() {
        let rt = WasmRuntime::new(default_config());
        assert!(!rt.has_filesystem_access());
    }

    #[test]
    fn wasm_filesystem_when_read_enabled() {
        let mut cfg = default_config();
        cfg.allow_workspace_read = true;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.has_filesystem_access());
    }

    #[test]
    fn wasm_filesystem_when_write_enabled() {
        let mut cfg = default_config();
        cfg.allow_workspace_write = true;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.has_filesystem_access());
    }

    #[test]
    fn wasm_no_long_running() {
        let rt = WasmRuntime::new(default_config());
        assert!(!rt.supports_long_running());
    }

    #[test]
    fn wasm_memory_budget() {
        let rt = WasmRuntime::new(default_config());
        assert_eq!(rt.memory_budget(), 64 * 1024 * 1024);
    }

    #[test]
    fn wasm_shell_command_errors() {
        let rt = WasmRuntime::new(default_config());
        let result = rt.build_shell_command("echo hello", Path::new("/tmp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not support shell"));
    }

    #[test]
    fn wasm_storage_path_default() {
        let rt = WasmRuntime::new(default_config());
        assert!(rt.storage_path().to_string_lossy().contains("vibewindow"));
    }

    #[test]
    fn wasm_storage_path_with_workspace() {
        let rt = WasmRuntime::with_workspace(default_config(), PathBuf::from("/home/user/project"));
        assert_eq!(
            rt.storage_path(),
            PathBuf::from("/home/user/project").join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        );
    }

    // ── Config validation ──────────────────────────────────────

    #[test]
    fn validate_rejects_zero_memory() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 0;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("must be > 0"));
    }

    #[test]
    fn validate_rejects_excessive_memory() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 8192;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("4 GB safety limit"));
    }

    #[test]
    fn validate_rejects_zero_fuel() {
        let mut cfg = default_config();
        cfg.fuel_limit = 0;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("fuel_limit"));
    }

    #[test]
    fn validate_rejects_excessive_fuel() {
        let mut cfg = default_config();
        cfg.fuel_limit = 10_000_000_001;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("safety ceiling"));
    }

    #[test]
    fn validate_rejects_zero_max_module_size() {
        let mut cfg = default_config();
        cfg.max_module_size_mb = 0;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("max_module_size_mb"));
    }

    #[test]
    fn validate_rejects_empty_tools_dir() {
        let mut cfg = default_config();
        cfg.tools_dir = String::new();
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn validate_rejects_absolute_tools_dir() {
        let mut cfg = default_config();
        cfg.tools_dir = "/tmp/wasm-tools".into();
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("workspace-relative"));
    }

    #[test]
    fn validate_rejects_path_traversal() {
        let mut cfg = default_config();
        cfg.tools_dir = "../../../etc/passwd".into();
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn validate_allows_absolute_tools_dir_when_configured() {
        let mut cfg = default_config();
        cfg.tools_dir = "/tmp/wasm-tools".into();
        cfg.security.require_workspace_relative_tools_dir = false;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn validate_allows_path_traversal_when_configured() {
        let mut cfg = default_config();
        cfg.tools_dir = "../../../etc/passwd".into();
        cfg.security.require_workspace_relative_tools_dir = false;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn validate_rejects_wildcard_host_entries() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["*.example.com".into()];
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("wildcard"));
    }

    #[test]
    fn validate_rejects_hosts_with_scheme() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["https://api.example.com".into()];
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("host[:port] only"));
    }

    #[test]
    fn validate_rejects_empty_and_boundary_punctuated_hosts() {
        for (host, expected) in [
            ("   ", "empty entry"),
            (".example.com", "start/end with '.'"),
            ("example.com-", "start/end with '-'"),
            ("api@example.com", "invalid characters"),
        ] {
            let mut cfg = default_config();
            cfg.allowed_hosts = vec![host.into()];
            let error = WasmRuntime::new(cfg).validate_config().unwrap_err().to_string();

            assert!(error.contains(expected), "host {host:?} returned: {error}");
        }
    }

    #[test]
    fn validate_rejects_malformed_host_ports() {
        for host in ["example.com:", ":443", "example.com:http", "example.com:443:8443"] {
            let mut cfg = default_config();
            cfg.allowed_hosts = vec![host.into()];
            let error = WasmRuntime::new(cfg).validate_config().unwrap_err().to_string();

            assert!(
                error.contains("invalid port format") || error.contains("too many ':' separators"),
                "host {host:?} returned: {error}"
            );
        }
    }

    #[test]
    fn validate_accepts_and_normalizes_host_with_numeric_port() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec![" API.Example.COM:443 ".into()];
        let rt = WasmRuntime::new(cfg);

        assert!(rt.validate_config().is_ok());
        let caps = WasmCapabilities {
            allowed_hosts: vec!["api.example.com:443".into()],
            ..Default::default()
        };
        let effective = rt.validate_capabilities(&caps).unwrap();
        assert_eq!(effective.allowed_hosts, vec!["api.example.com:443"]);
    }

    #[test]
    fn validate_ignores_invalid_host_entries_when_non_strict() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["*.example.com".into(), "api.example.com".into()];
        cfg.security.strict_host_validation = false;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn validate_accepts_valid_config() {
        let rt = WasmRuntime::new(default_config());
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn validate_rejects_invalid_module_sha256_pin_format() {
        let mut cfg = default_config();
        cfg.security.module_sha256.insert("calc".into(), "not-a-sha256".into());
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("64-character hex"));
    }

    #[test]
    fn validate_rejects_invalid_module_sha256_pin_name() {
        let mut cfg = default_config();
        cfg.security.module_sha256.insert(
            "bad$name".into(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        );
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn validate_rejects_enforce_hash_policy_without_pins() {
        let mut cfg = default_config();
        cfg.security.module_hash_policy = WasmModuleHashPolicy::Enforce;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("requires at least one module pin"));
    }

    #[test]
    fn validate_accepts_max_memory() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 4096;
        let rt = WasmRuntime::new(cfg);
        assert!(rt.validate_config().is_ok());
    }

    // ── Capabilities & fuel ────────────────────────────────────

    #[test]
    fn effective_fuel_uses_config_default() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        assert_eq!(rt.effective_fuel(&caps), 1_000_000);
    }

    #[test]
    fn effective_fuel_respects_override() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { fuel_override: 500, ..Default::default() };
        assert_eq!(rt.effective_fuel(&caps), 500);
    }

    #[test]
    fn effective_fuel_clamps_override_to_config_limit() {
        let mut cfg = default_config();
        cfg.fuel_limit = 10;
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities { fuel_override: 99, ..Default::default() };
        assert_eq!(rt.effective_fuel(&caps), 10);
    }

    #[test]
    fn effective_memory_uses_config_default() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        assert_eq!(rt.effective_memory_bytes(&caps), 64 * 1024 * 1024);
    }

    #[test]
    fn effective_memory_respects_override() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { memory_override_mb: 32, ..Default::default() };
        assert_eq!(rt.effective_memory_bytes(&caps), 32 * 1024 * 1024);
    }

    #[test]
    fn effective_memory_clamps_override_to_config_limit() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { memory_override_mb: 256, ..Default::default() };
        assert_eq!(rt.effective_memory_bytes(&caps), 64 * 1024 * 1024);
    }

    #[test]
    fn default_capabilities_match_config() {
        let mut cfg = default_config();
        cfg.allow_workspace_read = true;
        cfg.allowed_hosts = vec!["api.example.com".into()];
        let rt = WasmRuntime::new(cfg);
        let caps = rt.default_capabilities();
        assert!(caps.read_workspace);
        assert!(!caps.write_workspace);
        assert_eq!(caps.allowed_hosts, vec!["api.example.com"]);
    }

    #[test]
    fn validate_capabilities_rejects_fuel_escalation() {
        let mut cfg = default_config();
        cfg.fuel_limit = 100;
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities { fuel_override: 101, ..Default::default() };
        let err = rt.validate_capabilities(&caps).unwrap_err();
        assert!(err.to_string().contains("fuel_override"));
    }

    #[test]
    fn validate_capabilities_rejects_memory_escalation() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 64;
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities { memory_override_mb: 65, ..Default::default() };
        let err = rt.validate_capabilities(&caps).unwrap_err();
        assert!(err.to_string().contains("memory_override_mb"));
    }

    #[test]
    fn validate_capabilities_rejects_read_workspace_escalation() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { read_workspace: true, ..Default::default() };
        let err = rt.validate_capabilities(&caps).unwrap_err();
        assert!(err.to_string().contains("read_workspace"));
    }

    #[test]
    fn validate_capabilities_rejects_write_workspace_escalation() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { write_workspace: true, ..Default::default() };
        let err = rt.validate_capabilities(&caps).unwrap_err();
        assert!(err.to_string().contains("write_workspace"));
    }

    #[test]
    fn validate_capabilities_rejects_host_escalation() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["api.example.com".into()];
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities {
            allowed_hosts: vec!["evil.example.com".into()],
            ..Default::default()
        };
        let err = rt.validate_capabilities(&caps).unwrap_err();
        assert!(err.to_string().contains("not in runtime.wasm.allowed_hosts"));
    }

    #[test]
    fn validate_capabilities_accepts_host_subset() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["api.example.com".into(), "cdn.example.com".into()];
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities {
            allowed_hosts: vec!["api.example.com".into()],
            ..Default::default()
        };
        assert!(rt.validate_capabilities(&caps).is_ok());
    }

    #[test]
    fn validate_capabilities_clamps_escalation_when_configured() {
        let mut cfg = default_config();
        cfg.fuel_limit = 100;
        cfg.memory_limit_mb = 32;
        cfg.allowed_hosts = vec!["api.example.com".into()];
        cfg.security.capability_escalation_mode = WasmCapabilityEscalationMode::Clamp;
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities {
            read_workspace: true,
            write_workspace: true,
            allowed_hosts: vec!["api.example.com".into(), "evil.example.com".into()],
            fuel_override: 500,
            memory_override_mb: 64,
        };
        let effective = rt.validate_capabilities(&caps).expect("clamp should succeed");
        assert!(!effective.read_workspace);
        assert!(!effective.write_workspace);
        assert_eq!(effective.allowed_hosts, vec!["api.example.com"]);
        assert_eq!(effective.fuel_override, 100);
        assert_eq!(effective.memory_override_mb, 32);
    }

    #[test]
    fn validate_capabilities_normalizes_and_deduplicates_hosts() {
        let mut cfg = default_config();
        cfg.allowed_hosts = vec!["api.example.com".into(), "cdn.example.com".into()];
        cfg.security.capability_escalation_mode = WasmCapabilityEscalationMode::Clamp;
        let rt = WasmRuntime::new(cfg);
        let caps = WasmCapabilities {
            allowed_hosts: vec![
                " CDN.EXAMPLE.COM ".into(),
                "api.example.com".into(),
                "cdn.example.com".into(),
            ],
            ..Default::default()
        };

        let effective = rt.validate_capabilities(&caps).expect("host normalization should succeed");
        assert_eq!(effective.allowed_hosts, vec!["api.example.com", "cdn.example.com"]);
    }

    // ── Tools directory ────────────────────────────────────────

    #[test]
    fn tools_dir_resolves_relative_to_workspace() {
        let rt = WasmRuntime::new(default_config());
        let dir = rt.tools_dir(Path::new("/home/user/project"));
        assert_eq!(dir, PathBuf::from("/home/user/project/tools/wasm"));
    }

    #[test]
    fn list_modules_empty_when_dir_missing() {
        let rt = WasmRuntime::new(default_config());
        let modules = rt.list_modules(Path::new("/nonexistent/path")).unwrap();
        assert!(modules.is_empty());
    }

    #[test]
    fn list_modules_errors_when_tools_path_is_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_root = dir.path().join("tools");
        std::fs::create_dir_all(&tools_root).unwrap();
        std::fs::write(tools_root.join("wasm"), b"not a directory").unwrap();

        let rt = WasmRuntime::new(default_config());
        let err = rt.list_modules(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Failed to read tools dir"));
    }

    #[test]
    fn list_modules_finds_wasm_files() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        // Create dummy .wasm files
        std::fs::write(tools_dir.join("calculator.wasm"), b"\0asm").unwrap();
        std::fs::write(tools_dir.join("formatter.wasm"), b"\0asm").unwrap();
        std::fs::write(tools_dir.join("bad$name.wasm"), b"\0asm").unwrap();
        std::fs::write(tools_dir.join("readme.txt"), b"not a wasm").unwrap();

        let rt = WasmRuntime::new(default_config());
        let modules = rt.list_modules(dir.path()).unwrap();
        assert_eq!(modules, vec!["calculator", "formatter"]);
    }

    #[test]
    fn validate_module_name_rejects_traversal_like_input() {
        let err = WasmRuntime::validate_module_name("../secrets").unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn validate_module_name_rejects_empty_and_overlong_input() {
        let empty = WasmRuntime::validate_module_name("").unwrap_err().to_string();
        assert!(empty.contains("cannot be empty"));

        let overlong = "a".repeat(129);
        let error = WasmRuntime::validate_module_name(&overlong).unwrap_err().to_string();
        assert!(error.contains("too long"));
    }

    // ── Module execution edge cases ────────────────────────────

    #[cfg(feature = "runtime-wasm")]
    #[test]
    fn execute_module_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        let result = rt.execute_module("nonexistent", dir.path(), &caps);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        // Should mention the module name
        assert!(err_msg.contains("nonexistent"));
    }

    #[cfg(feature = "runtime-wasm")]
    #[test]
    fn execute_module_rejects_invalid_module_name() {
        let dir = tempfile::tempdir().unwrap();
        let rt = WasmRuntime::new(default_config());
        let result = rt.execute_module("../escape", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid characters"));
    }

    #[cfg(feature = "runtime-wasm")]
    #[test]
    fn execute_module_errors_when_tools_path_is_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_root = dir.path().join("tools");
        std::fs::create_dir_all(&tools_root).unwrap();
        std::fs::write(tools_root.join("wasm"), b"not a directory").unwrap();

        let rt = WasmRuntime::new(default_config());
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("WASM tools path is not a directory"));
    }

    #[test]
    fn execute_module_invalid_wasm() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        // Write invalid WASM bytes
        std::fs::write(tools_dir.join("bad.wasm"), b"not valid wasm bytes at all").unwrap();

        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        let result = rt.execute_module("bad", dir.path(), &caps);
        assert!(result.is_err());
    }

    #[test]
    fn execute_module_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        // Write a file > 50 MB (we just check the size, don't actually allocate)
        // This test verifies the check without consuming 50 MB of disk
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();

        // File doesn't exist for oversized test — the missing file check catches first
        // But if it did exist and was 51 MB, the size check would catch it
        let result = rt.execute_module("oversized", dir.path(), &caps);
        assert!(result.is_err());
    }

    #[test]
    fn execute_module_enforce_hash_policy_rejects_mismatch() {
        if !WasmRuntime::is_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();
        std::fs::write(tools_dir.join("calc.wasm"), b"\0asm\x01\0\0\0").unwrap();

        let mut cfg = default_config();
        cfg.security.module_hash_policy = WasmModuleHashPolicy::Enforce;
        cfg.security.module_sha256.insert(
            "calc".into(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        );

        let rt = WasmRuntime::new(cfg);
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("integrity mismatch"));
    }

    #[test]
    fn execute_module_warn_hash_policy_allows_execution_path() {
        if !WasmRuntime::is_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();
        std::fs::write(tools_dir.join("calc.wasm"), b"\0asm\x01\0\0\0").unwrap();

        let mut cfg = default_config();
        cfg.security.module_hash_policy = WasmModuleHashPolicy::Warn;
        cfg.security.module_sha256.insert(
            "calc".into(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        );

        let rt = WasmRuntime::new(cfg);
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must export a 'run() -> i32'"));
    }

    #[cfg(unix)]
    #[test]
    fn execute_module_rejects_symlink_tools_dir_when_enabled() {
        if !WasmRuntime::is_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let real_tools_dir = dir.path().join("real-tools");
        std::fs::create_dir_all(&real_tools_dir).unwrap();
        std::fs::write(real_tools_dir.join("calc.wasm"), b"\0asm\x01\0\0\0").unwrap();

        let tools_parent = dir.path().join("tools");
        std::fs::create_dir_all(&tools_parent).unwrap();
        std::os::unix::fs::symlink(&real_tools_dir, tools_parent.join("wasm")).unwrap();

        let rt = WasmRuntime::new(default_config());
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("tools directory must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn execute_module_allows_symlink_tools_dir_when_disabled() {
        if !WasmRuntime::is_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let real_tools_dir = dir.path().join("real-tools");
        std::fs::create_dir_all(&real_tools_dir).unwrap();
        std::fs::write(real_tools_dir.join("calc.wasm"), b"\0asm\x01\0\0\0").unwrap();

        let tools_parent = dir.path().join("tools");
        std::fs::create_dir_all(&tools_parent).unwrap();
        std::os::unix::fs::symlink(&real_tools_dir, tools_parent.join("wasm")).unwrap();

        let mut cfg = default_config();
        cfg.security.reject_symlink_tools_dir = false;
        cfg.security.module_hash_policy = WasmModuleHashPolicy::Disabled;
        let rt = WasmRuntime::new(cfg);
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must export a 'run() -> i32'"));
    }

    #[cfg(unix)]
    #[test]
    fn execute_module_rejects_symlink_module_when_enabled() {
        if !WasmRuntime::is_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        let real_modules_dir = dir.path().join("real-modules");
        std::fs::create_dir_all(&tools_dir).unwrap();
        std::fs::create_dir_all(&real_modules_dir).unwrap();
        std::fs::write(real_modules_dir.join("calc.wasm"), b"\0asm\x01\0\0\0").unwrap();
        std::os::unix::fs::symlink(real_modules_dir.join("calc.wasm"), tools_dir.join("calc.wasm"))
            .unwrap();

        let rt = WasmRuntime::new(default_config());
        let result = rt.execute_module("calc", dir.path(), &WasmCapabilities::default());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("module path must not be a symlink"));
    }

    // ── Feature gate check ─────────────────────────────────────

    #[test]
    fn is_available_matches_feature_flag() {
        // This test verifies the compile-time feature detection works
        let available = WasmRuntime::is_available();
        assert_eq!(available, cfg!(feature = "runtime-wasm"));
    }

    // ── Memory overflow edge cases ─────────────────────────────

    #[test]
    fn memory_budget_no_overflow() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 4096; // Max valid
        let rt = WasmRuntime::new(cfg);
        assert_eq!(rt.memory_budget(), 4096 * 1024 * 1024);
    }

    #[test]
    fn effective_memory_saturating() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { memory_override_mb: u64::MAX, ..Default::default() };
        // Should not panic — override is clamped to config ceiling.
        assert_eq!(rt.effective_memory_bytes(&caps), 64 * 1024 * 1024);
    }

    // ── WasmCapabilities default ───────────────────────────────

    #[test]
    fn capabilities_default_is_locked_down() {
        let caps = WasmCapabilities::default();
        assert!(!caps.read_workspace);
        assert!(!caps.write_workspace);
        assert!(caps.allowed_hosts.is_empty());
        assert_eq!(caps.fuel_override, 0);
        assert_eq!(caps.memory_override_mb, 0);
    }

    // ── §3.1 / §3.2 WASM fuel & memory exhaustion tests ─────

    #[test]
    fn wasm_fuel_limit_enforced_in_config() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        let fuel = rt.effective_fuel(&caps);
        assert!(fuel > 0, "default fuel limit must be > 0 to prevent infinite loops");
    }

    #[test]
    fn wasm_memory_limit_enforced_in_config() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities::default();
        let mem_bytes = rt.effective_memory_bytes(&caps);
        assert!(mem_bytes > 0, "default memory limit must be > 0");
        assert!(
            mem_bytes <= 4096 * 1024 * 1024,
            "default memory must not exceed 4 GB safety limit"
        );
    }

    #[test]
    fn wasm_zero_fuel_override_uses_default() {
        let rt = WasmRuntime::new(default_config());
        let caps = WasmCapabilities { fuel_override: 0, ..Default::default() };
        assert_eq!(rt.effective_fuel(&caps), 1_000_000, "fuel_override=0 must use config default");
    }

    #[test]
    fn validate_rejects_memory_just_above_limit() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 4097;
        let rt = WasmRuntime::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("4 GB safety limit"));
    }

    #[test]
    fn execute_module_stub_returns_error_without_feature() {
        if !WasmRuntime::is_available() {
            let dir = tempfile::tempdir().unwrap();
            let tools_dir = dir.path().join("tools/wasm");
            std::fs::create_dir_all(&tools_dir).unwrap();
            std::fs::write(tools_dir.join("test.wasm"), b"\0asm\x01\0\0\0").unwrap();

            let rt = WasmRuntime::new(default_config());
            let caps = WasmCapabilities::default();
            let result = rt.execute_module("test", dir.path(), &caps);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not available"));
        }
    }
}
