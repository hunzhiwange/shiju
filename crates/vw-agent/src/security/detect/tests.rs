//! 验证沙箱后端探测、显式禁用和不可用后端的安全回退。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::{SandboxConfig, SecurityConfig};

    #[test]
    fn detect_best_sandbox_returns_something() {
        let sandbox = detect_best_sandbox();
        // Should always return at least NoopSandbox
        assert!(sandbox.is_available());
    }

    #[test]
    fn explicit_none_returns_noop() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(false),
                backend: SandboxBackend::None,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config);
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    // 总开关优先于具体后端选择，关闭后必须统一退回空沙箱。
    fn disabled_flag_overrides_every_selectable_backend() {
        for backend in [
            SandboxBackend::Auto,
            SandboxBackend::Landlock,
            SandboxBackend::Firejail,
            SandboxBackend::Bubblewrap,
            SandboxBackend::Docker,
        ] {
            let config = SecurityConfig {
                sandbox: SandboxConfig { enabled: Some(false), backend, firejail_args: Vec::new() },
                ..Default::default()
            };

            let sandbox = create_sandbox(&config);
            assert_eq!(sandbox.name(), "none");
            assert!(sandbox.is_available());
        }
    }

    #[test]
    fn none_backend_returns_noop_regardless_of_enablement() {
        for enabled in [None, Some(true)] {
            let config = SecurityConfig {
                sandbox: SandboxConfig {
                    enabled,
                    backend: SandboxBackend::None,
                    firejail_args: Vec::new(),
                },
                ..Default::default()
            };

            let sandbox = create_sandbox(&config);
            assert_eq!(sandbox.name(), "none");
            assert!(sandbox.is_available());
        }
    }

    #[test]
    fn auto_mode_detects_something() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: None, // Auto-detect
                backend: SandboxBackend::Auto,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };
        let sandbox = create_sandbox(&config);
        // Should return some sandbox (at least NoopSandbox)
        assert!(sandbox.is_available());
    }

    #[test]
    fn explicitly_enabled_auto_mode_keeps_detection_active() {
        let config = SecurityConfig {
            sandbox: SandboxConfig {
                enabled: Some(true),
                backend: SandboxBackend::Auto,
                firejail_args: Vec::new(),
            },
            ..Default::default()
        };

        let sandbox = create_sandbox(&config);

        assert!(sandbox.is_available());
    }

    #[test]
    fn unavailable_backend_messages_cover_explicit_backends_only() {
        let cases = [
            (SandboxBackend::Landlock, "Landlock requested"),
            (SandboxBackend::Firejail, "Firejail requested"),
            (SandboxBackend::Bubblewrap, "Bubblewrap requested"),
            (SandboxBackend::Docker, "Docker requested"),
        ];

        for (backend, expected_prefix) in cases {
            let message = unavailable_backend_message(&backend).expect("explicit backend message");
            assert!(message.starts_with(expected_prefix));
            assert!(message.contains("falling back to application-layer"));
        }
        assert!(unavailable_backend_message(&SandboxBackend::Auto).is_none());
        assert!(unavailable_backend_message(&SandboxBackend::None).is_none());
    }

    #[test]
    fn fallback_helpers_return_available_noop_sandboxes() {
        let unavailable = unavailable_backend_fallback(&SandboxBackend::Docker);
        let application_layer = application_layer_fallback();

        assert_eq!(unavailable.name(), "none");
        assert!(unavailable.is_available());
        assert_eq!(application_layer.name(), "none");
        assert!(application_layer.is_available());
    }

    #[test]
    // 显式后端不可用时只允许明确回退或提示可用项，不能伪装为已启用。
    fn unavailable_explicit_backends_fall_back_to_noop_or_report_available_backend() {
        for backend in [
            SandboxBackend::Landlock,
            SandboxBackend::Firejail,
            SandboxBackend::Bubblewrap,
            SandboxBackend::Docker,
        ] {
            let config = SecurityConfig {
                sandbox: SandboxConfig { enabled: Some(true), backend, firejail_args: Vec::new() },
                ..Default::default()
            };
            let sandbox = create_sandbox(&config);
            assert!(sandbox.is_available(), "create_sandbox must return a usable sandbox object");
            assert!(
                ["none", "landlock", "firejail", "bubblewrap", "docker"].contains(&sandbox.name())
            );
        }
    }
}
