// 覆盖内置格式化器目录、扩展名及命令配置。

use super::builtins::builtin_formatters;
use super::state::EnabledCheck;

#[test]
fn builtin_formatters_include_core_languages() {
    let formatters = builtin_formatters();

    assert!(formatters.contains_key("prettier"));
    assert!(formatters.contains_key("gofmt"));
    assert!(formatters["prettier"].extensions.contains(&".ts".to_string()));
}

#[test]
fn builtin_formatters_include_expected_full_catalog() {
    let formatters = builtin_formatters();
    let expected = [
        "air",
        "biome",
        "clang-format",
        "dart",
        "gleam",
        "gofmt",
        "htmlbeautifier",
        "ktlint",
        "latexindent",
        "mix",
        "nixfmt",
        "ocamlformat",
        "ormolu",
        "oxfmt",
        "pint",
        "prettier",
        "rubocop",
        "ruff",
        "rustfmt",
        "shfmt",
        "standardrb",
        "terraform",
        "uv",
        "zig",
    ];

    assert_eq!(formatters.len(), expected.len());
    for name in expected {
        let item = formatters.get(name).unwrap_or_else(|| panic!("missing formatter {name}"));
        assert_eq!(item.name, name);
        assert!(!item.command.is_empty(), "{name} command should not be empty");
        assert!(!item.extensions.is_empty(), "{name} extensions should not be empty");
    }
}

#[test]
fn javascript_formatters_use_bun_environment_and_expected_commands() {
    let formatters = builtin_formatters();

    let prettier = &formatters["prettier"];
    assert_eq!(prettier.command, ["bun", "x", "prettier", "--write", "$FILE"]);
    assert_eq!(prettier.environment.get("BUN_BE_BUN").map(String::as_str), Some("1"));
    assert!(matches!(prettier.enabled, EnabledCheck::Prettier));

    let oxfmt = &formatters["oxfmt"];
    assert_eq!(oxfmt.command, ["bun", "x", "oxfmt", "$FILE"]);
    assert_eq!(oxfmt.environment.get("BUN_BE_BUN").map(String::as_str), Some("1"));
    assert!(matches!(oxfmt.enabled, EnabledCheck::Oxfmt));

    let biome = &formatters["biome"];
    assert_eq!(biome.command, ["bun", "x", "@biomejs/biome", "check", "--write", "$FILE"]);
    assert_eq!(biome.environment.get("BUN_BE_BUN").map(String::as_str), Some("1"));
    assert!(matches!(biome.enabled, EnabledCheck::Biome));
}

#[test]
fn language_specific_formatters_have_expected_extensions_and_checks() {
    let formatters = builtin_formatters();

    assert_eq!(formatters["gofmt"].command, ["gofmt", "-w", "$FILE"]);
    assert_eq!(formatters["gofmt"].extensions, [".go"]);
    assert!(matches!(formatters["gofmt"].enabled, EnabledCheck::Which("gofmt")));

    assert!(formatters["mix"].extensions.iter().any(|ext| ext == ".heex"));
    assert!(matches!(formatters["mix"].enabled, EnabledCheck::Which("mix")));

    assert!(formatters["clang-format"].extensions.iter().any(|ext| ext == ".hpp"));
    assert!(formatters["clang-format"].extensions.iter().any(|ext| ext == ".C"));
    assert!(matches!(formatters["clang-format"].enabled, EnabledCheck::ClangFormat));

    assert_eq!(formatters["ruff"].command, ["ruff", "format", "$FILE"]);
    assert!(matches!(formatters["ruff"].enabled, EnabledCheck::Ruff));

    assert_eq!(formatters["uv"].command, ["uv", "format", "--", "$FILE"]);
    assert!(matches!(formatters["uv"].enabled, EnabledCheck::UvFormat));

    assert_eq!(formatters["pint"].command, ["./vendor/bin/pint", "$FILE"]);
    assert!(matches!(formatters["pint"].enabled, EnabledCheck::Pint));
}

#[test]
fn remaining_formatters_keep_expected_commands() {
    let formatters = builtin_formatters();
    let cases: &[(&str, &[&str], &[&str])] = &[
        ("zig", &["zig", "fmt", "$FILE"], &[".zig", ".zon"]),
        ("ktlint", &["ktlint", "-F", "$FILE"], &[".kt", ".kts"]),
        ("air", &["air", "format", "$FILE"], &[".R"]),
        ("rubocop", &["rubocop", "--autocorrect", "$FILE"], &[".rb", ".rake"]),
        ("standardrb", &["standardrb", "--fix", "$FILE"], &[".rb", ".gemspec"]),
        ("htmlbeautifier", &["htmlbeautifier", "$FILE"], &[".erb", ".html.erb"]),
        ("dart", &["dart", "format", "$FILE"], &[".dart"]),
        ("ocamlformat", &["ocamlformat", "-i", "$FILE"], &[".ml", ".mli"]),
        ("terraform", &["terraform", "fmt", "$FILE"], &[".tf", ".tfvars"]),
        ("latexindent", &["latexindent", "-w", "-s", "$FILE"], &[".tex"]),
        ("gleam", &["gleam", "format", "$FILE"], &[".gleam"]),
        ("shfmt", &["shfmt", "-w", "$FILE"], &[".sh", ".bash"]),
        ("nixfmt", &["nixfmt", "$FILE"], &[".nix"]),
        ("rustfmt", &["rustfmt", "$FILE"], &[".rs"]),
        ("ormolu", &["ormolu", "-i", "$FILE"], &[".hs"]),
    ];

    for (name, command, extensions) in cases {
        let item = &formatters[*name];
        assert_eq!(item.command, command.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        for extension in *extensions {
            assert!(item.extensions.contains(&extension.to_string()), "{name} missing {extension}");
        }
    }
}
