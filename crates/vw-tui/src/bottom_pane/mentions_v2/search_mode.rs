//! 本模块集中实现 `search_mode` 在底部面板中的职责。
//! 关键声明 `SearchMode`、`previous`、`next`、`accepts` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。


use super::candidate::MentionType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SearchMode {
    Results,
    FilesystemOnly,
    Tools,
}

impl SearchMode {
    pub(super) fn previous(self) -> Self {
        match self {
            Self::Results => Self::Tools,
            Self::FilesystemOnly => Self::Results,
            Self::Tools => Self::FilesystemOnly,
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Results => Self::FilesystemOnly,
            Self::FilesystemOnly => Self::Tools,
            Self::Tools => Self::Results,
        }
    }

    pub(super) fn accepts(self, mention_type: MentionType) -> bool {
        match self {
            Self::Results => true,
            Self::FilesystemOnly => {
                matches!(mention_type, MentionType::File | MentionType::Directory)
            }
            Self::Tools => matches!(mention_type, MentionType::Plugin | MentionType::Skill),
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Results => "All Results",
            Self::FilesystemOnly => "Filesystem Only",
            Self::Tools => "Plugins",
        }
    }
}
