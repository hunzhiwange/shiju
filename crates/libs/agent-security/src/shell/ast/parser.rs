//! Shell 命令解析器，负责在 tree-sitter AST 与回退解析之间提供统一的命令结构。

use parking_lot::Mutex;
#[cfg(not(feature = "shell-ast"))]
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};

#[cfg(feature = "shell-ast")]
use tree_sitter::{Parser, Tree};

static LAST_PARSED_COMMAND: LazyLock<Mutex<Option<(String, BashAst)>>> =
    LazyLock::new(|| Mutex::new(None));

/// ParseQuality 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseQuality {
    Full,
    Partial,
    Fallback,
}

/// BashAst 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone)]
pub struct BashAst {
    source: Arc<str>,
    quality: ParseQuality,
    #[cfg(feature = "shell-ast")]
    tree: Option<Arc<Tree>>,
}

/// BashNode 类型别名用于隐藏底层实现差异并统一调用点。
#[cfg(feature = "shell-ast")]
pub type BashNode<'tree> = tree_sitter::Node<'tree>;

/// BashNode 结构体保存当前模块对外暴露的数据。
#[cfg(not(feature = "shell-ast"))]
#[derive(Debug, Clone, Copy)]
pub struct BashNode<'tree>(PhantomData<&'tree ()>);

impl PartialEq for BashAst {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.quality == other.quality
    }
}

impl Eq for BashAst {}

impl BashAst {
    /// 执行 parse 操作，并返回调用方需要的结果。
    pub fn parse(command: &str) -> (Self, ParseQuality) {
        let trimmed = command.trim();
        if trimmed.is_empty() || !has_supported_shell_shape(trimmed) {
            let ast = Self::fallback(command);
            return (ast, ParseQuality::Fallback);
        }

        if let Some((cached_command, cached_ast)) = LAST_PARSED_COMMAND.lock().as_ref() {
            if cached_command == trimmed {
                return (cached_ast.clone(), cached_ast.quality());
            }
        }

        let ast = Self::parse_uncached(trimmed);
        LAST_PARSED_COMMAND.lock().replace((trimmed.to_string(), ast.clone()));
        (ast.clone(), ast.quality())
    }

    /// 执行 quality 操作，并返回调用方需要的结果。
    pub fn quality(&self) -> ParseQuality {
        self.quality
    }

    /// 执行 source 操作，并返回调用方需要的结果。
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 执行 root_node 操作，并返回调用方需要的结果。
    pub fn root_node(&self) -> Option<BashNode<'_>> {
        #[cfg(feature = "shell-ast")]
        {
            self.tree.as_ref().map(|tree| tree.root_node())
        }

        #[cfg(not(feature = "shell-ast"))]
        {
            None
        }
    }

    fn fallback(command: &str) -> Self {
        Self {
            source: Arc::from(command.to_string()),
            quality: ParseQuality::Fallback,
            #[cfg(feature = "shell-ast")]
            tree: None,
        }
    }

    #[cfg(feature = "shell-ast")]
    fn parse_uncached(command: &str) -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_bash::LANGUAGE;
        if parser.set_language(&language.into()).is_err() {
            return Self::fallback(command);
        }

        match parser.parse(command, None) {
            Some(tree) => Self {
                source: Arc::from(command.to_string()),
                quality: if tree.root_node().has_error() {
                    ParseQuality::Partial
                } else {
                    ParseQuality::Full
                },
                tree: Some(Arc::new(tree)),
            },
            None => Self::fallback(command),
        }
    }

    #[cfg(not(feature = "shell-ast"))]
    fn parse_uncached(command: &str) -> Self {
        Self::fallback(command)
    }
}

fn has_supported_shell_shape(command: &str) -> bool {
    let mut single_quote = false;
    let mut double_quote = false;
    let mut backtick = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if single_quote {
            if ch == '\'' {
                single_quote = false;
            }
            continue;
        }

        if backtick {
            if ch == '`' {
                backtick = false;
            }
            continue;
        }

        if ch == '\\' {
            chars.next();
            continue;
        }

        if double_quote {
            match ch {
                '"' => double_quote = false,
                '$' | '<' | '>' => {
                    if chars.peek() == Some(&'(') {
                        paren_depth += 1;
                        chars.next();
                    }
                }
                ')' => {
                    if paren_depth == 0 {
                        return false;
                    }
                    paren_depth -= 1;
                }
                _ => {}
            }
            continue;
        }

        match ch {
            '\'' => single_quote = true,
            '"' => double_quote = true,
            '`' => backtick = true,
            '(' => paren_depth += 1,
            '$' | '<' | '>' => {
                if chars.peek() == Some(&'(') {
                    paren_depth += 1;
                    chars.next();
                }
            }
            ')' => {
                if paren_depth == 0 {
                    return false;
                }
                paren_depth -= 1;
            }
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth == 0 {
                    return false;
                }
                brace_depth -= 1;
            }
            _ => {}
        }
    }

    !single_quote && !double_quote && !backtick && paren_depth == 0 && brace_depth == 0
}
