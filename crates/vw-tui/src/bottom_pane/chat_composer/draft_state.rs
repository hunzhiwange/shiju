//! 本模块集中实现 `draft_state` 在底部面板中的职责。
//! 关键声明 `DraftState`、`new`、`ComposerMentionBinding` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。


//! 编辑的输入编辑器草稿状态与作曲控制流保持分离。
//! Editable composer draft state kept separate from composer control flow.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::bottom_pane::MentionBinding;
use crate::bottom_pane::paste_burst::PasteBurst;
use crate::bottom_pane::textarea::TextArea;
use crate::bottom_pane::textarea::TextAreaState;

pub(super) struct DraftState {
    pub(super) textarea: TextArea,
    pub(super) textarea_state: RefCell<TextAreaState>,
    pub(super) is_bash_mode: bool,
    pub(super) pending_pastes: Vec<(String, String)>,
    pub(super) input_enabled: bool,
    pub(super) input_disabled_placeholder: Option<String>,
    pub(super) paste_burst: PasteBurst,
    pub(super) disable_paste_burst: bool,
    pub(super) mention_bindings: HashMap<u64, ComposerMentionBinding>,
    pub(super) recent_submission_mention_bindings: Vec<MentionBinding>,
}

impl DraftState {
    pub(super) fn new() -> Self {
        Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            is_bash_mode: false,
            pending_pastes: Vec::new(),
            input_enabled: true,
            input_disabled_placeholder: None,
            paste_burst: PasteBurst::default(),
            disable_paste_burst: false,
            mention_bindings: HashMap::new(),
            recent_submission_mention_bindings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ComposerMentionBinding {
    pub(super) sigil: char,
    pub(super) mention: String,
    pub(super) path: String,
}
