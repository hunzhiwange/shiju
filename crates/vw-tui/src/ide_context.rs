//! TUI `/ide` 支持所用的 IDE 上下文数据模型和对外辅助函数。
//! IDE context data model and public helpers for TUI `/ide` support.

mod ipc;
mod prompt;
#[cfg(windows)]
mod windows_pipe;

pub(crate) use ipc::fetch_ide_context;
pub(crate) use prompt::apply_ide_context_to_user_input;
pub(crate) use prompt::extract_prompt_request_with_offset;
pub(crate) use prompt::has_prompt_context;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// IDE 提供的活动文件与已打开标签页快照。
pub(crate) struct IdeContext {
    active_file: Option<ActiveFile>,
    #[serde(default)]
    open_tabs: Vec<FileDescriptor>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ActiveFile {
    // 活动文件在通用描述符之外还携带主选区、选中文本及全部选区。
    #[serde(flatten)]
    descriptor: FileDescriptor,
    selection: Range,
    #[serde(default)]
    active_selection_content: String,
    #[serde(default)]
    selections: Vec<Range>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FileDescriptor {
    // 只保留提示构造所需的显示标签和协议路径，忽略额外线形字段。
    label: String,
    path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Range {
    // IDE 协议位置按零基行列坐标表示一个半开选区。
    start: Position,
    end: Position,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Position {
    // 行列均使用协议原始的无符号整数，避免在反序列化边界引入坐标换算。
    line: u32,
    character: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    // 验证可读取现有 camelCase 载荷，并容忍 TUI 模型未声明的协议字段。
    fn deserializes_existing_ide_context_shape() {
        let value = json!({
            "activeFile": {
                "label": "lib.rs",
                "path": "src/lib.rs",
                "fsPath": "/repo/src/lib.rs",
                "selection": {
                    "start": { "line": 1, "character": 2 },
                    "end": { "line": 3, "character": 4 }
                },
                "activeSelectionContent": "selected",
                "selections": []
            },
            "openTabs": [
                {
                    "label": "main.rs",
                    "path": "src/main.rs",
                    "fsPath": "/repo/src/main.rs",
                    "startLine": 2,
                    "endLine": 10
                }
            ],
            "processEnv": {
                "path": "/usr/bin"
            }
        });

        // 断言同时覆盖扁平文件描述符、选区以及打开标签页列表的映射。
        let context: IdeContext = serde_json::from_value(value).expect("deserialize ide context");
        assert_eq!(
            context,
            IdeContext {
                active_file: Some(ActiveFile {
                    descriptor: FileDescriptor {
                        label: "lib.rs".to_string(),
                        path: "src/lib.rs".to_string(),
                    },
                    selection: Range {
                        start: Position {
                            line: 1,
                            character: 2,
                        },
                        end: Position {
                            line: 3,
                            character: 4,
                        },
                    },
                    active_selection_content: "selected".to_string(),
                    selections: Vec::new(),
                }),
                open_tabs: vec![FileDescriptor {
                    label: "main.rs".to_string(),
                    path: "src/main.rs".to_string(),
                }],
            }
        );
    }
}
