use serde::{Deserialize, Serialize};

/// 文件变更状态枚举
///
/// 表示文件在版本控制系统中的状态变化。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Added,
    Deleted,
    Modified,
}

/// 文件变更信息
///
/// 描述单个文件的变更详情，包括路径、新增行数、删除行数和变更状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub path: String,
    pub added: i64,
    pub removed: i64,
    pub status: Status,
}

/// 文件系统节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    File,
    Directory,
}

/// 文件系统节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub path: String,
    pub absolute: String,
    pub r#type: NodeType,
    pub ignored: bool,
}

/// 文件内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Binary,
}

/// 文件内容结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub r#type: ContentType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// 文件操作错误类型
#[derive(Debug)]
pub enum Error {
    AccessDenied(String),
    Io(std::io::Error),
    BinaryFile(String),
    InvalidText(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied(path) => {
                write!(f, "Access denied: path escapes project directory: {}", path)
            }
            Self::Io(err) => write!(f, "{}", err),
            Self::BinaryFile(path) => write!(f, "Binary file cannot be read as text: {path}"),
            Self::InvalidText(path) => write!(f, "Invalid or unsupported text encoding: {path}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// 文件搜索输入参数
#[derive(Debug, Clone)]
pub struct SearchInput {
    pub query: String,
    pub limit: usize,
    pub dirs: bool,
    pub r#type: Option<String>,
}
