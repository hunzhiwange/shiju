//! Shell 命令路径提取表，负责按命令参数约定抽取需要校验的路径参数。

/// PathArgKind 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathArgKind {
    Single,
    Optional,
    Multiple,
}

/// PathExtractor 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathExtractor {
    /// command 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub command: &'static str,
    /// path_positions 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub path_positions: Option<&'static [usize]>,
    /// flag_paths 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub flag_paths: &'static [(&'static str, PathArgKind)],
    /// respects_double_dash 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub respects_double_dash: bool,
}

const ZERO: &[usize] = &[0];
const ONE: &[usize] = &[1];
const ZERO_ONE: &[usize] = &[0, 1];
const NONE: &[usize] = &[];

/// PATH_EXTRACTORS 提供当前模块共享的静态数据。
pub static PATH_EXTRACTORS: &[PathExtractor] = &[
    PathExtractor {
        command: "cd",
        path_positions: Some(ZERO),
        flag_paths: &[],
        respects_double_dash: false,
    },
    PathExtractor {
        command: "ls",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "cat",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "head",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "tail",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "wc",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "sort",
        path_positions: Some(ZERO),
        flag_paths: &[
            ("-o", PathArgKind::Single),
            ("--output", PathArgKind::Single),
            ("-T", PathArgKind::Single),
            ("--temporary-directory", PathArgKind::Single),
        ],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "uniq",
        path_positions: Some(ZERO),
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "find",
        path_positions: Some(ZERO),
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "mkdir",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "rm",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "rmdir",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "cp",
        path_positions: Some(ZERO_ONE),
        flag_paths: &[("-t", PathArgKind::Single), ("--target-directory", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "mv",
        path_positions: Some(ZERO_ONE),
        flag_paths: &[("-t", PathArgKind::Single), ("--target-directory", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "install",
        path_positions: Some(ZERO_ONE),
        flag_paths: &[("-t", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "touch",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "ln",
        path_positions: Some(ZERO_ONE),
        flag_paths: &[("-t", PathArgKind::Single), ("--target-directory", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "grep",
        path_positions: Some(ONE),
        flag_paths: &[
            ("-f", PathArgKind::Single),
            ("--file", PathArgKind::Single),
            ("--include", PathArgKind::Single),
            ("--exclude", PathArgKind::Single),
        ],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "rg",
        path_positions: Some(ONE),
        flag_paths: &[
            ("-f", PathArgKind::Single),
            ("--file", PathArgKind::Single),
            ("-g", PathArgKind::Single),
            ("--glob", PathArgKind::Single),
        ],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "ag",
        path_positions: Some(ONE),
        flag_paths: &[("-G", PathArgKind::Single), ("-g", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "ack",
        path_positions: Some(ONE),
        flag_paths: &[("--output", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "sed",
        path_positions: Some(ONE),
        flag_paths: &[("-f", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "awk",
        path_positions: Some(ONE),
        flag_paths: &[("-f", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "cut",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "tar",
        path_positions: Some(NONE),
        flag_paths: &[
            ("-f", PathArgKind::Single),
            ("-C", PathArgKind::Single),
            ("--directory", PathArgKind::Single),
        ],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "gzip",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "gunzip",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "zip",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "unzip",
        path_positions: None,
        flag_paths: &[("-d", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "chmod",
        path_positions: None,
        flag_paths: &[("--reference", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "chown",
        path_positions: None,
        flag_paths: &[("--reference", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "scp",
        path_positions: Some(ZERO_ONE),
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "rsync",
        path_positions: None,
        flag_paths: &[("--files-from", PathArgKind::Single), ("--temp-dir", PathArgKind::Single)],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "tee",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "less",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "more",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "basename",
        path_positions: Some(ZERO),
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "dirname",
        path_positions: Some(ZERO),
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "realpath",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
    PathExtractor {
        command: "readlink",
        path_positions: None,
        flag_paths: &[],
        respects_double_dash: true,
    },
];

/// 执行 extract_paths 操作，并返回调用方需要的结果。
pub fn extract_paths(command: &str, args: &[String]) -> Vec<String> {
    let Some(extractor) = PATH_EXTRACTORS.iter().find(|extractor| extractor.command == command)
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut positional_index = 0usize;
    let mut after_double_dash = false;
    let mut index = 0usize;

    while index < args.len() {
        let arg = args[index].as_str();
        if extractor.respects_double_dash && !after_double_dash && arg == "--" {
            after_double_dash = true;
            index += 1;
            continue;
        }

        if !after_double_dash {
            if let Some((value, consumed)) =
                match_flag(arg, args.get(index + 1), extractor.flag_paths)
            {
                if !value.is_empty() {
                    out.push(value);
                }
                index += consumed;
                continue;
            }

            if arg.starts_with('-') && arg != "-" {
                index += 1;
                continue;
            }
        }

        if extractor.path_positions.is_none_or(|positions| positions.contains(&positional_index)) {
            out.push(arg.to_string());
        }
        positional_index += 1;
        index += 1;
    }

    out
}

fn match_flag(
    arg: &str,
    next: Option<&String>,
    flag_paths: &[(&'static str, PathArgKind)],
) -> Option<(String, usize)> {
    for (flag, kind) in flag_paths {
        if arg == *flag {
            return match kind {
                PathArgKind::Single | PathArgKind::Optional | PathArgKind::Multiple => {
                    Some((next.cloned().unwrap_or_default(), 2))
                }
            };
        }

        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Some((value.to_string(), 1));
        }

        if !flag.starts_with("--") && arg.starts_with(flag) && arg.len() > flag.len() {
            return Some((arg[flag.len()..].to_string(), 1));
        }
    }

    None
}
