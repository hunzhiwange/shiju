use super::*;
use std::ffi::OsString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

const PRIMARY_ENV: &str = "VW_LSP_CONFIG_TEST_PRIMARY";
const SECONDARY_ENV: &str = "VW_LSP_CONFIG_TEST_SECONDARY";
const FALLBACK_ENV: &str = "VW_LSP_CONFIG_TEST_FALLBACK";
const MISSING_ENV: &str = "VW_LSP_CONFIG_TEST_MISSING";
const MISSING_PROGRAM: &str = "vw-lsp-config-test-command-that-does-not-exist";
const OTHER_MISSING_PROGRAM: &str = "vw-lsp-config-test-other-command-that-does-not-exist";

const ENV_OVERRIDE_COMMANDS: &[ServerCommandCandidate] = &[ServerCommandCandidate {
    program: MISSING_PROGRAM,
    args: &["--stdio"],
    env_vars: &[PRIMARY_ENV, SECONDARY_ENV],
}];

const CANDIDATE_FALLBACK_COMMANDS: &[ServerCommandCandidate] = &[
    ServerCommandCandidate {
        program: MISSING_PROGRAM,
        args: &["--first"],
        env_vars: &[MISSING_ENV],
    },
    ServerCommandCandidate {
        program: OTHER_MISSING_PROGRAM,
        args: &["--second"],
        env_vars: &[FALLBACK_ENV],
    },
];

const ENV_OVERRIDE_CONFIG: ServerConfig = ServerConfig {
    server_key: "test-server",
    language_id: "test",
    extensions: &["test"],
    commands: ENV_OVERRIDE_COMMANDS,
};

const CANDIDATE_FALLBACK_CONFIG: ServerConfig = ServerConfig {
    server_key: "fallback-server",
    language_id: "test",
    extensions: &["test"],
    commands: CANDIDATE_FALLBACK_COMMANDS,
};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }

    #[cfg(unix)]
    fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

#[test]
fn server_config_maps_every_supported_extension_to_expected_metadata() {
    let cases: &[(&str, &str, &str, &str, &[&str])] = &[
        ("rs", "rust-analyzer", "rust", "rust-analyzer", &[]),
        (
            "ts",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "tsx",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "js",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "jsx",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "mts",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "cts",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "mjs",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        (
            "cjs",
            "typescript-language-server",
            "typescript",
            "typescript-language-server",
            &["--stdio"],
        ),
        ("py", "pyright-langserver", "python", "pyright-langserver", &["--stdio"]),
        ("go", "gopls", "go", "gopls", &[]),
        ("c", "clangd", "cpp", "clangd", &[]),
        ("cc", "clangd", "cpp", "clangd", &[]),
        ("cpp", "clangd", "cpp", "clangd", &[]),
        ("cxx", "clangd", "cpp", "clangd", &[]),
        ("h", "clangd", "cpp", "clangd", &[]),
        ("hh", "clangd", "cpp", "clangd", &[]),
        ("hpp", "clangd", "cpp", "clangd", &[]),
        ("hxx", "clangd", "cpp", "clangd", &[]),
        (
            "json",
            "vscode-json-language-server",
            "json",
            "vscode-json-language-server",
            &["--stdio"],
        ),
        (
            "jsonc",
            "vscode-json-language-server",
            "json",
            "vscode-json-language-server",
            &["--stdio"],
        ),
        ("yaml", "yaml-language-server", "yaml", "yaml-language-server", &["--stdio"]),
        ("yml", "yaml-language-server", "yaml", "yaml-language-server", &["--stdio"]),
        ("toml", "taplo", "toml", "taplo", &["lsp", "stdio"]),
        ("lua", "lua-language-server", "lua", "lua-language-server", &[]),
    ];

    for (extension, server_key, language_id, program, args) in cases {
        let path = format!("source.{extension}");
        let config = server_config_for_path(Path::new(&path)).unwrap();

        assert_eq!(config.server_key, *server_key, "extension: {extension}");
        assert_eq!(config.language_id, *language_id, "extension: {extension}");
        assert_eq!(config.commands[0].program, *program, "extension: {extension}");
        assert_eq!(config.commands[0].args, *args, "extension: {extension}");
    }
}

#[test]
fn python_config_preserves_pyright_and_pylsp_candidates_in_order() {
    let config = server_config_for_path(Path::new("script.py")).unwrap();

    assert_eq!(config.commands.len(), 2);
    assert_eq!(config.commands[0].program, "pyright-langserver");
    assert_eq!(config.commands[0].args, ["--stdio"]);
    assert_eq!(config.commands[0].env_vars, ["PYRIGHT_LANGSERVER", "PYRIGHT_LANGSERVER_PATH"]);
    assert_eq!(config.commands[1].program, "pylsp");
    assert!(config.commands[1].args.is_empty());
    assert_eq!(config.commands[1].env_vars, ["PYLSP", "PYLSP_PATH"]);
}

#[test]
fn server_config_matches_extensions_case_insensitively() {
    let config = server_config_for_path(Path::new("src/lib.RS")).unwrap();

    assert_eq!(config.server_key, "rust-analyzer");
    assert_eq!(config.language_id, "rust");
}

#[test]
fn server_config_rejects_unknown_extensions() {
    assert!(server_config_for_path(Path::new("README.unknown")).is_none());
}

#[test]
fn server_config_rejects_paths_without_extensions() {
    assert!(server_config_for_path(Path::new("README")).is_none());
}

#[cfg(unix)]
#[test]
fn server_config_rejects_non_utf8_extensions() {
    use std::os::unix::ffi::OsStringExt;

    let path = std::path::PathBuf::from(OsString::from_vec(b"source.\xff".to_vec()));

    assert!(server_config_for_path(&path).is_none());
}

#[test]
fn resolve_command_uses_first_environment_override_and_trims_it() {
    let _lock = env_lock();
    let _primary = EnvVarGuard::set(PRIMARY_ENV, "  /custom/bin/language-server  ");
    let _secondary = EnvVarGuard::set(SECONDARY_ENV, "/ignored/language-server");

    let command = resolve_command(&ENV_OVERRIDE_CONFIG).unwrap();

    assert_eq!(command.program, "/custom/bin/language-server");
    assert_eq!(command.args, ["--stdio"]);
}

#[test]
fn resolve_command_skips_blank_environment_overrides() {
    let _lock = env_lock();
    let _primary = EnvVarGuard::set(PRIMARY_ENV, " \t ");
    let _secondary = EnvVarGuard::set(SECONDARY_ENV, "/fallback/language-server");

    let command = resolve_command(&ENV_OVERRIDE_CONFIG).unwrap();

    assert_eq!(command.program, "/fallback/language-server");
    assert_eq!(command.args, ["--stdio"]);
}

#[cfg(unix)]
#[test]
fn resolve_command_skips_non_utf8_environment_overrides() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = env_lock();
    let non_utf8 = OsString::from_vec(vec![0xff]);
    let _primary = EnvVarGuard::set_os(PRIMARY_ENV, &non_utf8);
    let _secondary = EnvVarGuard::set(SECONDARY_ENV, "/fallback/language-server");

    let command = resolve_command(&ENV_OVERRIDE_CONFIG).unwrap();

    assert_eq!(command.program, "/fallback/language-server");
    assert_eq!(command.args, ["--stdio"]);
}

#[test]
fn resolve_command_falls_back_to_a_later_candidate() {
    let _lock = env_lock();
    let _missing = EnvVarGuard::unset(MISSING_ENV);
    let _fallback = EnvVarGuard::set(FALLBACK_ENV, "/fallback/bin/language-server");

    let command = resolve_command(&CANDIDATE_FALLBACK_CONFIG).unwrap();

    assert_eq!(command.program, "/fallback/bin/language-server");
    assert_eq!(command.args, ["--second"]);
}

#[test]
fn resolve_command_uses_an_existing_program_when_overrides_are_unset() {
    let current_exe = std::env::current_exe().expect("current test executable should be available");
    let program: &'static str =
        Box::leak(current_exe.to_string_lossy().into_owned().into_boxed_str());
    let commands: &'static [ServerCommandCandidate] = Box::leak(
        vec![ServerCommandCandidate { program, args: &["--stdio"], env_vars: &[] }]
            .into_boxed_slice(),
    );
    let config = ServerConfig {
        server_key: "current-test-binary",
        language_id: "test",
        extensions: &["test"],
        commands,
    };

    let command = resolve_command(&config).unwrap();

    assert_eq!(Path::new(&command.program), current_exe);
    assert_eq!(command.args, ["--stdio"]);
}

#[test]
fn resolve_command_returns_none_when_no_candidate_is_available() {
    let _lock = env_lock();
    let _primary = EnvVarGuard::unset(PRIMARY_ENV);
    let _secondary = EnvVarGuard::unset(SECONDARY_ENV);

    assert!(resolve_command(&ENV_OVERRIDE_CONFIG).is_none());
}
