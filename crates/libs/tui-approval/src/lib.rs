//! 本模块集中实现 `approval_overlay` 在底部面板中的职责。
//! 关键声明 `ApprovalOverlay`、`new`、`build_list`、`decision_for` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。

//! 通过 VibeWindow 拥有的行为边界复制了审批表面。
//! Copied approval surface with a VibeWindow-owned action boundary.
//!
//! The overlay owns presentation and focus only.  It receives read-only security data and emits
//! [`V3Action::ResolveApproval`]; it never builds a permission reply, calls a client, or silently
//! turns a dismissal into approval.  Every request keeps explicit allow, custom, deny, and cancel
//! choices supplied by the bridge.

use tui_widgets::bottom_pane_view::BottomPaneView;
use tui_widgets::list_selection_view::{
    ListSelectionView, SelectionItem, SelectionViewParams,
};
use tui_widgets::selection_popup_common::render_menu_surface;
use tui_render::renderable::Renderable;
use tui_contracts::action::{V3Action, V3ApprovalDecision, V3ApprovalRequest};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

pub struct ApprovalOverlay {
    request: V3ApprovalRequest,
    list: ListSelectionView,
    done: bool,
}

impl ApprovalOverlay {
    #[must_use]
    pub fn new(request: V3ApprovalRequest) -> Self {
        let list = Self::build_list(&request);
        Self { request, list, done: false }
    }

    fn build_list(request: &V3ApprovalRequest) -> ListSelectionView {
        let items = request
            .choices
            .iter()
            .map(|choice| SelectionItem {
                id: choice.id.clone(),
                name: choice.label.clone(),
                ..SelectionItem::default()
            })
            .collect();
        ListSelectionView::new(SelectionViewParams {
            surface: "approval".to_string(),
            title: Some(request.title.clone()),
            items,
            header: Box::new(ApprovalHeader { kind: request.kind, detail: request.detail.clone() }),
            ..SelectionViewParams::default()
        })
    }

    fn decision_for(&self, choice_id: &str) -> Option<V3ApprovalDecision> {
        self.request
            .choices
            .iter()
            .find(|choice| choice.id == choice_id)
            .map(|choice| choice.decision.clone())
    }

    fn resolve(&mut self, choice_id: String) -> V3Action {
        let Some(decision) = self.decision_for(&choice_id) else {
            return V3Action::None;
        };
        self.done = true;
        V3Action::ResolveApproval { request_id: self.request.id.clone(), decision }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.done
    }

    #[must_use]
    pub fn request(&self) -> &V3ApprovalRequest {
        &self.request
    }
}

impl BottomPaneView for ApprovalOverlay {
    fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if self.done {
            return V3Action::None;
        }
        if event.code == KeyCode::Esc {
            self.done = true;
            return V3Action::ResolveApproval {
                request_id: self.request.id.clone(),
                decision: V3ApprovalDecision::Cancel,
            };
        }
        match self.list.handle_key_event(event) {
            V3Action::Select { item_id, .. } => self.resolve(item_id),
            V3Action::Cancel => {
                self.done = true;
                V3Action::ResolveApproval {
                    request_id: self.request.id.clone(),
                    decision: V3ApprovalDecision::Cancel,
                }
            }
            _ => V3Action::None,
        }
    }
}

impl Renderable for ApprovalOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let content = render_menu_surface(area, buf);
        self.list.render(content, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.list.desired_height(width).saturating_add(2)
    }
}

struct ApprovalHeader {
    kind: tui_contracts::action::V3ApprovalKind,
    detail: String,
}

impl Renderable for ApprovalHeader {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let kind = match self.kind {
            tui_contracts::action::V3ApprovalKind::Command => "Command",
            tui_contracts::action::V3ApprovalKind::FileChange => "File change",
            tui_contracts::action::V3ApprovalKind::Network => "Network access",
            tui_contracts::action::V3ApprovalKind::Permissions => "Additional permissions",
            tui_contracts::action::V3ApprovalKind::Elicitation => "External question",
        };
        Paragraph::new(vec![Line::from(kind.cyan()), Line::from(self.detail.clone())])
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        Paragraph::new(vec![Line::from("security"), Line::from(self.detail.clone())])
            .wrap(Wrap { trim: false })
            .line_count(width.max(1)) as u16
    }
}

#[cfg(test)]
mod tests;
