// 覆盖格式化运行时缓存、命令执行及失败容错。

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_runtime_is_singleton() {
    let first = super::runtime::format_runtime() as *const _;
    let second = super::runtime::format_runtime() as *const _;

    assert_eq!(first, second);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn init_ignores_edited_events_without_a_string_file() {
    super::runtime::init();
    super::runtime::init();

    crate::app::agent::bus::publish_value(
        crate::app::agent::file::event::EDITED,
        serde_json::Value::Null,
        None,
    );
    crate::app::agent::bus::publish_value(
        crate::app::agent::file::event::EDITED,
        serde_json::json!({ "file": 7 }),
        None,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(not(target_arch = "wasm32"))]
struct PathGuard {
    old: Option<std::ffi::OsString>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PathGuard {
    fn set(path: &std::path::Path) -> Self {
        let old = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", path.as_os_str()) };
        Self { old }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var("PATH", value) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_executable(dir: &std::path::Path, name: &str, body: &str) {
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

#[cfg(not(target_arch = "wasm32"))]
async fn in_project<T: Send + 'static>(
    dir: &std::path::Path,
    f: impl FnOnce() -> crate::app::agent::project::BoxFuture<T> + Send + 'static,
) -> T {
    crate::app::agent::project::instance::provide(dir, None, f).await.expect("project context")
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_without_extension_returns_ok_without_instance_state() {
    assert_eq!(super::runtime::format_file("README").await, Ok(()));
    assert_eq!(super::runtime::format_file(".env").await, Ok(()));
    assert_eq!(super::runtime::format_file("").await, Ok(()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_with_unknown_extension_returns_ok_in_project_instance() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let result = crate::app::agent::project::instance::provide(temp.path(), None, || {
        Box::pin(async move { super::runtime::format_file("notes.unknown").await })
    })
    .await
    .expect("instance should be provided");

    assert_eq!(result, Ok(()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_runs_enabled_formatter_with_replaced_file_argument() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("main.go");
    std::fs::write(&file, "package main\n").expect("write go file");
    write_executable(
        bin.path(),
        "gofmt",
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$1\" \"$2\" > gofmt.log\n",
    );
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("gofmt".to_string(), true);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    let log = std::fs::read_to_string(project.path().join("gofmt.log")).expect("read log");
    let lines = log.lines().collect::<Vec<_>>();
    let cwd = std::fs::canonicalize(project.path()).expect("canonical project path");
    assert_eq!(lines[0], cwd.to_string_lossy());
    assert_eq!(lines[1], "-w");
    assert_eq!(lines[2], file.to_string_lossy());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_applies_formatter_environment_and_continues_after_failure() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("app.js");
    std::fs::write(&file, "const x=1\n").expect("write js file");
    std::fs::write(project.path().join("package.json"), r#"{"devDependencies":{"prettier":"3"}}"#)
        .expect("write package");
    write_executable(
        bin.path(),
        "bun",
        "#!/bin/sh\nprintf '%s\\n' \"$BUN_BE_BUN\" \"$1\" \"$2\" \"$3\" \"$4\" > bun.log\nexit 7\n",
    );
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("prettier".to_string(), true);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    let log = std::fs::read_to_string(project.path().join("bun.log")).expect("read log");
    let lines = log.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], "x");
    assert_eq!(lines[2], "prettier");
    assert_eq!(lines[3], "--write");
    assert_eq!(lines[4], file.to_string_lossy());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_skips_formatter_when_cached_as_disabled() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("main.go");
    std::fs::write(&file, "package main\n").expect("write go file");
    write_executable(bin.path(), "gofmt", "#!/bin/sh\n: > gofmt-ran\n");
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("gofmt".to_string(), false);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    assert!(!project.path().join("gofmt-ran").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_matches_extensions_case_sensitively() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("main.GO");
    std::fs::write(&file, "package main\n").expect("write go file");
    write_executable(bin.path(), "gofmt", "#!/bin/sh\n: > gofmt-ran\n");
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("gofmt".to_string(), true);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    assert!(!project.path().join("gofmt-ran").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_returns_ok_when_formatter_process_cannot_start() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("empty bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("main.go");
    std::fs::write(&file, "package main\n").expect("write go file");
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("gofmt".to_string(), true);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_runs_every_matching_formatter_when_commands_fail() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("app.rb");
    std::fs::write(&file, "puts 'hello'\n").expect("write ruby file");
    write_executable(
        bin.path(),
        "rubocop",
        "#!/bin/sh\nprintf '%s\\n' \"$1\" \"$2\" > rubocop.log\nexit 7\n",
    );
    write_executable(
        bin.path(),
        "standardrb",
        "#!/bin/sh\nprintf '%s\\n' \"$1\" \"$2\" > standardrb.log\nexit 9\n",
    );
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            {
                let mut enabled = state.enabled.lock().expect("enabled lock");
                enabled.insert("rubocop".to_string(), true);
                enabled.insert("standardrb".to_string(), true);
            }
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    let rubocop =
        std::fs::read_to_string(project.path().join("rubocop.log")).expect("read rubocop log");
    let standardrb = std::fs::read_to_string(project.path().join("standardrb.log"))
        .expect("read standardrb log");
    let expected_file = file.to_string_lossy();
    let rubocop_args = rubocop.lines().collect::<Vec<_>>();
    let standardrb_args = standardrb.lines().collect::<Vec<_>>();
    assert_eq!(rubocop_args, ["--autocorrect", expected_file.as_ref()]);
    assert_eq!(standardrb_args, ["--fix", expected_file.as_ref()]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn format_file_passes_shell_metacharacters_as_a_single_file_argument() {
    let _lock = env_lock();
    let bin = tempfile::TempDir::new().expect("bin temp dir");
    let project = tempfile::TempDir::new().expect("project temp dir");
    let file = project.path().join("main; : > injected.go");
    std::fs::write(&file, "package main\n").expect("write go file");
    write_executable(
        bin.path(),
        "gofmt",
        "#!/bin/sh\nprintf '%s\\n' \"$#\" \"$1\" \"$2\" > argv.log\n",
    );
    let _path = PathGuard::set(bin.path());
    let file_arg = file.to_string_lossy().to_string();

    let result = in_project(project.path(), move || {
        let file_arg = file_arg.clone();
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("gofmt".to_string(), true);
            super::runtime::format_file(&file_arg).await
        })
    })
    .await;

    assert_eq!(result, Ok(()));
    let argv = std::fs::read_to_string(project.path().join("argv.log")).expect("read argv log");
    let expected_file = file.to_string_lossy();
    let args = argv.lines().collect::<Vec<_>>();
    assert_eq!(args, ["2", "-w", expected_file.as_ref()]);
    assert!(!project.path().join("injected.go").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn status_returns_builtin_formatter_statuses_in_project_instance() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let statuses = crate::app::agent::project::instance::provide(temp.path(), None, || {
        Box::pin(async move { super::runtime::status().await })
    })
    .await
    .expect("instance should be provided");

    assert!(statuses.iter().any(|item| item.name == "prettier"));
    assert!(statuses.iter().any(|item| item.name == "rustfmt"));
    assert!(statuses.iter().all(|item| !item.extensions.is_empty()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn status_uses_cached_enabled_values() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let statuses = in_project(temp.path(), || {
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            state.enabled.lock().expect("enabled lock").insert("rustfmt".to_string(), true);
            super::runtime::status().await
        })
    })
    .await;

    let rustfmt = statuses.iter().find(|item| item.name == "rustfmt").expect("rustfmt status");
    assert!(rustfmt.enabled);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn status_uses_cached_disabled_values() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let statuses = in_project(temp.path(), || {
        Box::pin(async move {
            let state = super::state::instance_state()().await;
            let names = state.formatters.keys().cloned().collect::<Vec<_>>();
            {
                let mut enabled = state.enabled.lock().expect("enabled lock");
                for name in names {
                    enabled.insert(name, false);
                }
            }
            super::runtime::status().await
        })
    })
    .await;

    let rustfmt = statuses.iter().find(|item| item.name == "rustfmt").expect("rustfmt status");
    assert!(!rustfmt.enabled);
    assert!(statuses.iter().all(|item| !item.enabled));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn runtime_test_module_is_loaded_on_wasm() {
    assert!(true);
}
