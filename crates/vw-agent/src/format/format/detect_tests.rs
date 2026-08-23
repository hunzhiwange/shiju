// 覆盖格式化器对可执行文件、项目依赖与配置文件的探测。

use super::detect::{
    biome_enabled, clang_format_enabled, has_dep, is_enabled, ocamlformat_enabled, oxfmt_enabled,
    pint_enabled, prettier_enabled, rlang_air_enabled, ruff_enabled, uvformat_enabled, which,
};
use super::state::{EnabledCheck, FormatterInfo, State};
use crate::app::agent::{flag, project::instance};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

struct PathGuard {
    old: Option<std::ffi::OsString>,
}

impl PathGuard {
    fn set(path: &Path) -> Self {
        let old = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", path.as_os_str()) };
        Self { old }
    }

    fn remove() -> Self {
        let old = std::env::var_os("PATH");
        unsafe { std::env::remove_var("PATH") };
        Self { old }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var("PATH", value) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

fn write_executable(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod");
    }
}

async fn in_project<T: Send + 'static>(
    dir: &Path,
    f: impl FnOnce() -> crate::app::agent::project::BoxFuture<T> + Send + 'static,
) -> T {
    instance::provide(dir, None, f).await.expect("project context")
}

fn formatter(name: &str, enabled: EnabledCheck) -> FormatterInfo {
    FormatterInfo {
        name: name.to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled,
    }
}

#[test]
fn which_returns_none_for_missing_program() {
    assert!(which("vibewindow-format-command-that-should-not-exist").is_none());
}

#[test]
fn which_returns_none_when_path_is_unset() {
    let _lock = env_lock();
    let _path = PathGuard::remove();

    assert!(which("any-program").is_none());
}

#[test]
fn which_finds_program_on_path_and_ignores_directories() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(temp.path(), "tool", "#!/bin/sh\nexit 0\n");
    std::fs::create_dir(temp.path().join("folder-tool")).expect("create dir");
    let _path = PathGuard::set(temp.path());

    assert_eq!(which("tool").as_deref(), Some(temp.path().join("tool").as_path()));
    assert!(which("folder-tool").is_none());
}

#[cfg(windows)]
#[test]
fn which_finds_exe_suffix_on_windows() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    write_executable(temp.path(), "tool.exe", "");
    let _path = PathGuard::set(temp.path());

    assert_eq!(which("tool").as_deref(), Some(temp.path().join("tool.exe").as_path()));
}

#[test]
fn has_dep_checks_dependencies_and_dev_dependencies_only() {
    assert!(has_dep(&json!({"dependencies": {"prettier": "^3"}}), "prettier"));
    assert!(has_dep(&json!({"devDependencies": {"ruff": "*"}}), "ruff"));
    assert!(!has_dep(&json!({"peerDependencies": {"prettier": "^3"}}), "prettier"));
    assert!(!has_dep(&json!({"dependencies": []}), "prettier"));
}

#[tokio::test]
async fn is_enabled_uses_cache_before_recomputing() {
    let state = Arc::new(State {
        enabled: Mutex::new(HashMap::from([("cached".to_string(), true)])),
        formatters: HashMap::new(),
    });
    let cached = FormatterInfo {
        name: "cached".to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled: EnabledCheck::Which("definitely-not-present"),
    };
    assert!(is_enabled(&state, &cached).await);

    let always = FormatterInfo {
        name: "always".to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled: EnabledCheck::Always,
    };
    assert!(is_enabled(&state, &always).await);
    assert_eq!(state.enabled.lock().unwrap().get("always"), Some(&true));
}

#[tokio::test]
async fn is_enabled_computes_which_results_and_caches_both_values() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "present-tool", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let state = Arc::new(State::default());

    assert!(is_enabled(&state, &formatter("present", EnabledCheck::Which("present-tool"))).await);
    assert!(!is_enabled(&state, &formatter("missing", EnabledCheck::Which("missing-tool"))).await);

    let cache = state.enabled.lock().expect("enabled cache");
    assert_eq!(cache.get("present"), Some(&true));
    assert_eq!(cache.get("missing"), Some(&false));
}

#[tokio::test]
async fn is_enabled_returns_computed_value_when_cache_is_poisoned() {
    let state = Arc::new(State::default());
    let poisoned = Arc::clone(&state);
    assert!(
        std::thread::spawn(move || {
            let _guard = poisoned.enabled.lock().expect("lock enabled cache");
            panic!("poison enabled cache");
        })
        .join()
        .is_err()
    );

    assert!(is_enabled(&state, &formatter("always", EnabledCheck::Always)).await);
    assert!(state.enabled.is_poisoned());
}

#[tokio::test]
async fn enabled_check_dispatches_every_detector_variant() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    for program in ["bun", "clang-format", "ocamlformat", "ruff", "uv"] {
        write_executable(bin.path(), program, "#!/bin/sh\nexit 0\n");
    }
    write_executable(bin.path(), "air", "#!/bin/sh\nprintf 'R language formatter\\n'\n");
    let _path = PathGuard::set(bin.path());

    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(
        project.path().join("package.json"),
        r#"{"dependencies":{"prettier":"3","oxfmt":"1"}}"#,
    )
    .expect("write package");
    std::fs::write(project.path().join("biome.json"), "{}").expect("write biome config");
    std::fs::write(project.path().join(".clang-format"), "BasedOnStyle: LLVM\n")
        .expect("write clang config");
    std::fs::write(project.path().join("ruff.toml"), "").expect("write ruff config");
    std::fs::write(project.path().join(".ocamlformat"), "profile = default\n")
        .expect("write ocaml config");
    std::fs::write(project.path().join("composer.json"), r#"{"require":{"laravel/pint":"^1.0"}}"#)
        .expect("write composer");
    std::fs::write(project.path().join("marker.toml"), "").expect("write marker");
    let oxfmt_expected = *flag::VIBEWINDOW_EXPERIMENTAL_OXFMT;

    in_project(project.path(), move || {
        Box::pin(async move {
            let state = Arc::new(State::default());
            let checks = [
                ("always", EnabledCheck::Always, true),
                ("which", EnabledCheck::Which("bun"), true),
                ("file-up-any", EnabledCheck::FileUpAny(&["missing.toml", "marker.toml"]), true),
                ("prettier", EnabledCheck::Prettier, true),
                ("oxfmt", EnabledCheck::Oxfmt, oxfmt_expected),
                ("biome", EnabledCheck::Biome, true),
                ("clang-format", EnabledCheck::ClangFormat, true),
                ("ruff", EnabledCheck::Ruff, true),
                ("uv", EnabledCheck::UvFormat, false),
                ("pint", EnabledCheck::Pint, true),
                ("ocamlformat", EnabledCheck::Ocamlformat, true),
                ("air", EnabledCheck::RLangAir, true),
            ];

            for (name, check, expected) in checks {
                assert_eq!(is_enabled(&state, &formatter(name, check)).await, expected, "{name}");
            }
        })
    })
    .await;
}

#[tokio::test]
async fn project_scoped_detectors_are_disabled_without_project_context() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    for program in ["bun", "clang-format", "ocamlformat", "ruff"] {
        write_executable(bin.path(), program, "#!/bin/sh\nexit 0\n");
    }
    let _path = PathGuard::set(bin.path());

    assert!(!prettier_enabled().await);
    assert!(!oxfmt_enabled().await);
    assert!(!biome_enabled().await);
    assert!(!clang_format_enabled().await);
    assert!(!ocamlformat_enabled().await);
    assert!(!ruff_enabled().await);
    assert!(!pint_enabled().await);
}

#[tokio::test]
async fn prettier_requires_bun_package_json_and_dependency() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());

    let project = tempfile::tempdir().expect("project tempdir");
    assert!(!in_project(project.path(), || Box::pin(async { prettier_enabled().await })).await);

    std::fs::write(project.path().join("package.json"), r#"{"devDependencies":{"prettier":"3"}}"#)
        .expect("write package");
    assert!(in_project(project.path(), || Box::pin(async { prettier_enabled().await })).await);
}

#[tokio::test]
async fn prettier_ignores_invalid_json_and_missing_dependency() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");

    std::fs::write(project.path().join("package.json"), "{").expect("write package");
    assert!(!in_project(project.path(), || Box::pin(async { prettier_enabled().await })).await);

    std::fs::write(project.path().join("package.json"), r#"{"dependencies":{"eslint":"9"}}"#)
        .expect("write package");
    assert!(!in_project(project.path(), || Box::pin(async { prettier_enabled().await })).await);
}

#[tokio::test]
async fn prettier_searches_up_to_worktree_only() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let outer = tempfile::tempdir().expect("outer tempdir");
    let project = outer.path().join("project");
    let nested = project.join("src").join("nested");
    std::fs::create_dir_all(project.join(".git")).expect("create git dir");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    std::fs::write(project.join("package.json"), r#"{"dependencies":{"prettier":"3"}}"#)
        .expect("write package");
    assert!(in_project(&nested, || Box::pin(async { prettier_enabled().await })).await);

    std::fs::remove_file(project.join("package.json")).expect("remove package");
    std::fs::write(outer.path().join("package.json"), r#"{"dependencies":{"prettier":"3"}}"#)
        .expect("write outer package");
    assert!(!in_project(&nested, || Box::pin(async { prettier_enabled().await })).await);
}

#[tokio::test]
async fn prettier_skips_unreadable_nearest_manifest_and_uses_valid_parent() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");
    let nested = project.path().join("nested");
    std::fs::create_dir(project.path().join(".git")).expect("create git dir");
    std::fs::create_dir(&nested).expect("create nested dir");
    std::fs::create_dir(nested.join("package.json")).expect("create unreadable package path");
    std::fs::write(project.path().join("package.json"), r#"{"dependencies":{"prettier":"3"}}"#)
        .expect("write parent package");

    assert!(in_project(&nested, || Box::pin(async { prettier_enabled().await })).await);
}

#[tokio::test]
async fn prettier_is_disabled_without_bun_or_project_context() {
    let _lock = env_lock();
    let empty_bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(empty_bin.path());

    assert!(!prettier_enabled().await);
    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(project.path().join("package.json"), r#"{"dependencies":{"prettier":"3"}}"#)
        .expect("write package");
    assert!(!in_project(project.path(), || Box::pin(async { prettier_enabled().await })).await);
}

#[tokio::test]
async fn oxfmt_requires_experiment_bun_and_declared_dependency() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(project.path().join("package.json"), r#"{"dependencies":{"oxfmt":"1"}}"#)
        .expect("write package");

    assert!(!in_project(project.path(), || Box::pin(async { oxfmt_enabled().await })).await);

    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let enabled = in_project(project.path(), || Box::pin(async { oxfmt_enabled().await })).await;
    assert_eq!(enabled, *flag::VIBEWINDOW_EXPERIMENTAL_OXFMT);

    std::fs::write(project.path().join("package.json"), r#"{"dependencies":{"prettier":"3"}}"#)
        .expect("write package without oxfmt");
    assert!(!in_project(project.path(), || Box::pin(async { oxfmt_enabled().await })).await);
}

#[tokio::test]
async fn biome_requires_bun_and_config_file() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");

    assert!(!in_project(project.path(), || Box::pin(async { biome_enabled().await })).await);
    std::fs::write(project.path().join("biome.jsonc"), "{}").expect("write config");
    assert!(in_project(project.path(), || Box::pin(async { biome_enabled().await })).await);
}

#[tokio::test]
async fn biome_accepts_json_config_and_ignores_parent_above_worktree() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "bun", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let outer = tempfile::tempdir().expect("outer tempdir");
    let project = outer.path().join("project");
    let nested = project.join("src");
    std::fs::create_dir_all(project.join(".git")).expect("create git dir");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    std::fs::write(project.join("biome.json"), "{}").expect("write config");
    assert!(in_project(&nested, || Box::pin(async { biome_enabled().await })).await);

    std::fs::remove_file(project.join("biome.json")).expect("remove config");
    std::fs::write(outer.path().join("biome.jsonc"), "{}").expect("write outer config");
    assert!(!in_project(&nested, || Box::pin(async { biome_enabled().await })).await);
}

#[tokio::test]
async fn clang_and_ocamlformat_require_binary_and_config() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "clang-format", "#!/bin/sh\nexit 0\n");
    write_executable(bin.path(), "ocamlformat", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");

    assert!(!in_project(project.path(), || Box::pin(async { clang_format_enabled().await })).await);
    assert!(!in_project(project.path(), || Box::pin(async { ocamlformat_enabled().await })).await);

    std::fs::write(project.path().join(".clang-format"), "BasedOnStyle: LLVM\n")
        .expect("write clang config");
    std::fs::write(project.path().join(".ocamlformat"), "profile = default\n")
        .expect("write ocaml config");

    assert!(in_project(project.path(), || Box::pin(async { clang_format_enabled().await })).await);
    assert!(in_project(project.path(), || Box::pin(async { ocamlformat_enabled().await })).await);
}

#[tokio::test]
async fn clang_and_ocamlformat_are_disabled_without_binary() {
    let _lock = env_lock();
    let empty_bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(empty_bin.path());
    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(project.path().join(".clang-format"), "BasedOnStyle: LLVM\n")
        .expect("write clang config");
    std::fs::write(project.path().join(".ocamlformat"), "profile = default\n")
        .expect("write ocaml config");

    assert!(!in_project(project.path(), || Box::pin(async { clang_format_enabled().await })).await);
    assert!(!in_project(project.path(), || Box::pin(async { ocamlformat_enabled().await })).await);
}

#[tokio::test]
async fn ruff_detects_config_or_dependency_mentions() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "ruff", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());

    let with_config = tempfile::tempdir().expect("project tempdir");
    std::fs::write(with_config.path().join("pyproject.toml"), "[tool.ruff]\n")
        .expect("write pyproject");
    assert!(in_project(with_config.path(), || Box::pin(async { ruff_enabled().await })).await);

    let with_dep = tempfile::tempdir().expect("project tempdir");
    std::fs::write(with_dep.path().join("requirements.txt"), "ruff==0.6.0\n")
        .expect("write requirements");
    assert!(in_project(with_dep.path(), || Box::pin(async { ruff_enabled().await })).await);

    let without_match = tempfile::tempdir().expect("project tempdir");
    std::fs::write(without_match.path().join("pyproject.toml"), "[project]\n")
        .expect("write pyproject");
    assert!(!in_project(without_match.path(), || Box::pin(async { ruff_enabled().await })).await);
}

#[tokio::test]
async fn ruff_requires_binary_and_supported_config_content() {
    let _lock = env_lock();
    let empty_bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(empty_bin.path());
    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(project.path().join("ruff.toml"), "").expect("write ruff config");
    assert!(!in_project(project.path(), || Box::pin(async { ruff_enabled().await })).await);

    write_executable(empty_bin.path(), "ruff", "#!/bin/sh\nexit 0\n");
    std::fs::remove_file(project.path().join("ruff.toml")).expect("remove ruff config");
    std::fs::write(project.path().join("pyproject.toml"), "[tool.black]\n")
        .expect("write pyproject");
    assert!(!in_project(project.path(), || Box::pin(async { ruff_enabled().await })).await);
}

#[tokio::test]
async fn ruff_accepts_dedicated_configs_and_each_dependency_manifest() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "ruff", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());

    for (file, content) in [
        ("ruff.toml", ""),
        (".ruff.toml", ""),
        ("requirements.txt", "ruff==0.6.0\n"),
        ("pyproject.toml", "[project.dependencies]\nruff = '*'\n"),
        ("Pipfile", "[packages]\nruff = '*'\n"),
    ] {
        let project = tempfile::tempdir().expect("project tempdir");
        std::fs::write(project.path().join(file), content).expect("write ruff candidate");

        assert!(
            in_project(project.path(), || Box::pin(async { ruff_enabled().await })).await,
            "{file}"
        );
    }
}

#[tokio::test]
async fn uvformat_checks_uv_help_and_is_disabled_when_ruff_is_enabled() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(bin.path(), "uv", "#!/bin/sh\nexit 0\n");
    let _path = PathGuard::set(bin.path());

    let project = tempfile::tempdir().expect("project tempdir");
    assert!(in_project(project.path(), || Box::pin(async { uvformat_enabled().await })).await);

    write_executable(bin.path(), "ruff", "#!/bin/sh\nexit 0\n");
    std::fs::write(project.path().join("ruff.toml"), "").expect("write ruff config");
    assert!(!in_project(project.path(), || Box::pin(async { uvformat_enabled().await })).await);
}

#[tokio::test]
async fn uvformat_is_disabled_when_uv_missing_or_help_fails() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().expect("project tempdir");

    assert!(!in_project(project.path(), || Box::pin(async { uvformat_enabled().await })).await);

    write_executable(bin.path(), "uv", "#!/bin/sh\nexit 2\n");
    assert!(!in_project(project.path(), || Box::pin(async { uvformat_enabled().await })).await);
}

#[tokio::test]
async fn pint_detects_laravel_pint_in_composer_sections() {
    let project = tempfile::tempdir().expect("project tempdir");
    assert!(!in_project(project.path(), || Box::pin(async { pint_enabled().await })).await);

    std::fs::write(
        project.path().join("composer.json"),
        r#"{"require-dev":{"laravel/pint":"^1.0"}}"#,
    )
    .expect("write composer");
    assert!(in_project(project.path(), || Box::pin(async { pint_enabled().await })).await);

    std::fs::write(project.path().join("composer.json"), r#"{"require":{"laravel/pint":"^1.0"}}"#)
        .expect("write composer");
    assert!(in_project(project.path(), || Box::pin(async { pint_enabled().await })).await);
}

#[tokio::test]
async fn pint_ignores_invalid_json_and_missing_sections() {
    let project = tempfile::tempdir().expect("project tempdir");

    std::fs::write(project.path().join("composer.json"), "{").expect("write composer");
    assert!(!in_project(project.path(), || Box::pin(async { pint_enabled().await })).await);

    std::fs::write(project.path().join("composer.json"), r#"{"suggest":{"laravel/pint":"^1.0"}}"#)
        .expect("write composer");
    assert!(!in_project(project.path(), || Box::pin(async { pint_enabled().await })).await);
}

#[tokio::test]
async fn pint_skips_invalid_nearest_manifest_and_uses_valid_parent() {
    let project = tempfile::tempdir().expect("project tempdir");
    let nested = project.path().join("nested");
    std::fs::create_dir_all(project.path().join(".git")).expect("create git dir");
    std::fs::create_dir(&nested).expect("create nested dir");
    std::fs::write(nested.join("composer.json"), "{").expect("write invalid composer");
    std::fs::write(
        project.path().join("composer.json"),
        r#"{"require-dev":{"laravel/pint":"^1.0"}}"#,
    )
    .expect("write parent composer");

    assert!(in_project(&nested, || Box::pin(async { pint_enabled().await })).await);
}

#[tokio::test]
async fn rlang_air_validates_help_output_first_line() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(
        bin.path(),
        "air",
        "#!/bin/sh\nprintf 'R language formatter\\nmore help\\n'\n",
    );
    let _path = PathGuard::set(bin.path());

    assert!(rlang_air_enabled().await);

    write_executable(bin.path(), "air", "#!/bin/sh\nprintf 'unrelated tool\\n'\n");
    assert!(!rlang_air_enabled().await);
}

#[tokio::test]
async fn rlang_air_is_disabled_when_missing_or_command_fails() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    let _path = PathGuard::set(bin.path());

    assert!(!rlang_air_enabled().await);

    write_executable(bin.path(), "air", "#!/bin/sh\nexit 1\n");
    assert!(!rlang_air_enabled().await);
}

#[tokio::test]
async fn rlang_air_rejects_signature_outside_first_help_line() {
    let _lock = env_lock();
    let bin = tempfile::tempdir().expect("bin tempdir");
    write_executable(
        bin.path(),
        "air",
        "#!/bin/sh\nprintf 'another tool\\nR language formatter\\n'\n",
    );
    let _path = PathGuard::set(bin.path());

    assert!(!rlang_air_enabled().await);
}

#[tokio::test]
async fn file_up_any_enabled_check_finds_nearest_target() {
    let state = Arc::new(State::default());
    let project = tempfile::tempdir().expect("project tempdir");
    std::fs::write(project.path().join("marker.toml"), "").expect("write marker");
    let item = FormatterInfo {
        name: "marker".to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled: EnabledCheck::FileUpAny(&["marker.toml"]),
    };

    assert!(
        in_project(project.path(), move || {
            let state = Arc::clone(&state);
            Box::pin(async move { is_enabled(&state, &item).await })
        })
        .await
    );
}

#[tokio::test]
async fn file_up_any_enabled_check_uses_cache_for_false_result() {
    let state = Arc::new(State::default());
    let project = tempfile::tempdir().expect("project tempdir");
    let item = FormatterInfo {
        name: "marker".to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled: EnabledCheck::FileUpAny(&["marker.toml"]),
    };

    assert!(
        !in_project(project.path(), {
            let state = Arc::clone(&state);
            move || Box::pin(async move { is_enabled(&state, &item).await })
        })
        .await
    );
    std::fs::write(project.path().join("marker.toml"), "").expect("write marker");

    let cached = FormatterInfo {
        name: "marker".to_string(),
        command: vec![],
        environment: HashMap::new(),
        extensions: vec![],
        enabled: EnabledCheck::FileUpAny(&["marker.toml"]),
    };
    assert!(
        !in_project(project.path(), {
            let state = Arc::clone(&state);
            move || Box::pin(async move { is_enabled(&state, &cached).await })
        })
        .await
    );
}

#[tokio::test]
async fn file_up_any_enabled_check_is_disabled_without_project_context() {
    let state = Arc::new(State::default());
    let item = formatter("marker", EnabledCheck::FileUpAny(&["marker.toml"]));

    assert!(!is_enabled(&state, &item).await);
    assert_eq!(state.enabled.lock().unwrap().get("marker"), Some(&false));
}
