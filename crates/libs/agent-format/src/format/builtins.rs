//! 内置格式化器清单。
//!
//! 该模块集中声明文件扩展名、执行命令、环境变量和启用条件之间的映射，
//! 供格式化运行时按文件类型选择可用格式化器。

use super::state::{EnabledCheck, FormatterInfo};
use std::collections::HashMap;

/// 构建内置格式化器索引。
///
/// # 返回值
///
/// 返回以格式化器名称为 key 的配置表。每个条目描述命令模板、适用扩展名、
/// 额外环境变量以及启用探测方式。
pub(super) fn builtin_formatters() -> HashMap<String, FormatterInfo> {
    let mut out = HashMap::new();
    let mut insert = |f: FormatterInfo| {
        out.insert(f.name.clone(), f);
    };

    insert(FormatterInfo {
        name: "gofmt".to_string(),
        command: vec!["gofmt".to_string(), "-w".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".go".to_string()],
        enabled: EnabledCheck::Which("gofmt"),
    });
    insert(FormatterInfo {
        name: "mix".to_string(),
        command: vec!["mix".to_string(), "format".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".ex", ".exs", ".eex", ".heex", ".leex", ".neex", ".sface"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        enabled: EnabledCheck::Which("mix"),
    });
    insert(FormatterInfo {
        name: "prettier".to_string(),
        command: vec![
            "bun".to_string(),
            "x".to_string(),
            "prettier".to_string(),
            "--write".to_string(),
            "$FILE".to_string(),
        ],
        environment: HashMap::from([("BUN_BE_BUN".to_string(), "1".to_string())]),
        extensions: vec![
            ".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts", ".html", ".htm", ".css",
            ".scss", ".sass", ".less", ".vue", ".svelte", ".json", ".jsonc", ".yaml", ".yml",
            ".toml", ".xml", ".md", ".mdx", ".graphql", ".gql",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
        enabled: EnabledCheck::Prettier,
    });
    insert(FormatterInfo {
        name: "oxfmt".to_string(),
        command: vec!["bun".to_string(), "x".to_string(), "oxfmt".to_string(), "$FILE".to_string()],
        environment: HashMap::from([("BUN_BE_BUN".to_string(), "1".to_string())]),
        extensions: vec![".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        enabled: EnabledCheck::Oxfmt,
    });
    insert(FormatterInfo {
        name: "biome".to_string(),
        command: vec![
            "bun".to_string(),
            "x".to_string(),
            "@biomejs/biome".to_string(),
            "check".to_string(),
            "--write".to_string(),
            "$FILE".to_string(),
        ],
        environment: HashMap::from([("BUN_BE_BUN".to_string(), "1".to_string())]),
        extensions: vec![
            ".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts", ".html", ".htm", ".css",
            ".scss", ".sass", ".less", ".vue", ".svelte", ".json", ".jsonc", ".yaml", ".yml",
            ".toml", ".xml", ".md", ".mdx", ".graphql", ".gql",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
        enabled: EnabledCheck::Biome,
    });
    insert(FormatterInfo {
        name: "zig".to_string(),
        command: vec!["zig".to_string(), "fmt".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".zig".to_string(), ".zon".to_string()],
        enabled: EnabledCheck::Which("zig"),
    });
    insert(FormatterInfo {
        name: "clang-format".to_string(),
        command: vec!["clang-format".to_string(), "-i".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![
            ".c", ".cc", ".cpp", ".cxx", ".c++", ".h", ".hh", ".hpp", ".hxx", ".h++", ".ino", ".C",
            ".H",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
        enabled: EnabledCheck::ClangFormat,
    });
    insert(FormatterInfo {
        name: "ktlint".to_string(),
        command: vec!["ktlint".to_string(), "-F".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".kt".to_string(), ".kts".to_string()],
        enabled: EnabledCheck::Which("ktlint"),
    });
    insert(FormatterInfo {
        name: "ruff".to_string(),
        command: vec!["ruff".to_string(), "format".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".py".to_string(), ".pyi".to_string()],
        enabled: EnabledCheck::Ruff,
    });
    insert(FormatterInfo {
        name: "air".to_string(),
        command: vec!["air".to_string(), "format".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".R".to_string()],
        enabled: EnabledCheck::RLangAir,
    });
    insert(FormatterInfo {
        name: "uv".to_string(),
        command: vec![
            "uv".to_string(),
            "format".to_string(),
            "--".to_string(),
            "$FILE".to_string(),
        ],
        environment: HashMap::new(),
        extensions: vec![".py".to_string(), ".pyi".to_string()],
        enabled: EnabledCheck::UvFormat,
    });
    insert(FormatterInfo {
        name: "rubocop".to_string(),
        command: vec!["rubocop".to_string(), "--autocorrect".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".rb", ".rake", ".gemspec", ".ru"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        enabled: EnabledCheck::Which("rubocop"),
    });
    insert(FormatterInfo {
        name: "standardrb".to_string(),
        command: vec!["standardrb".to_string(), "--fix".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".rb", ".rake", ".gemspec", ".ru"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        enabled: EnabledCheck::Which("standardrb"),
    });
    insert(FormatterInfo {
        name: "htmlbeautifier".to_string(),
        command: vec!["htmlbeautifier".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".erb".to_string(), ".html.erb".to_string()],
        enabled: EnabledCheck::Which("htmlbeautifier"),
    });
    insert(FormatterInfo {
        name: "dart".to_string(),
        command: vec!["dart".to_string(), "format".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".dart".to_string()],
        enabled: EnabledCheck::Which("dart"),
    });
    insert(FormatterInfo {
        name: "ocamlformat".to_string(),
        command: vec!["ocamlformat".to_string(), "-i".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".ml".to_string(), ".mli".to_string()],
        enabled: EnabledCheck::Ocamlformat,
    });
    insert(FormatterInfo {
        name: "terraform".to_string(),
        command: vec!["terraform".to_string(), "fmt".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".tf".to_string(), ".tfvars".to_string()],
        enabled: EnabledCheck::Which("terraform"),
    });
    insert(FormatterInfo {
        name: "latexindent".to_string(),
        command: vec![
            "latexindent".to_string(),
            "-w".to_string(),
            "-s".to_string(),
            "$FILE".to_string(),
        ],
        environment: HashMap::new(),
        extensions: vec![".tex".to_string()],
        enabled: EnabledCheck::Which("latexindent"),
    });
    insert(FormatterInfo {
        name: "gleam".to_string(),
        command: vec!["gleam".to_string(), "format".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".gleam".to_string()],
        enabled: EnabledCheck::Which("gleam"),
    });
    insert(FormatterInfo {
        name: "shfmt".to_string(),
        command: vec!["shfmt".to_string(), "-w".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".sh".to_string(), ".bash".to_string()],
        enabled: EnabledCheck::Which("shfmt"),
    });
    insert(FormatterInfo {
        name: "nixfmt".to_string(),
        command: vec!["nixfmt".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".nix".to_string()],
        enabled: EnabledCheck::Which("nixfmt"),
    });
    insert(FormatterInfo {
        name: "rustfmt".to_string(),
        command: vec!["rustfmt".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".rs".to_string()],
        enabled: EnabledCheck::Which("rustfmt"),
    });
    insert(FormatterInfo {
        name: "pint".to_string(),
        command: vec!["./vendor/bin/pint".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".php".to_string()],
        enabled: EnabledCheck::Pint,
    });
    insert(FormatterInfo {
        name: "ormolu".to_string(),
        command: vec!["ormolu".to_string(), "-i".to_string(), "$FILE".to_string()],
        environment: HashMap::new(),
        extensions: vec![".hs".to_string()],
        enabled: EnabledCheck::Which("ormolu"),
    });

    out
}
