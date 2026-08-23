//! Web 搜索活动历史单元。
//!
//! 调用标识用于把开始与完成事件关联到同一活动单元。
//! 活动状态可显示动画，完成后只保留稳定的查询和结果摘要。
//! 缺失动作详情时仍展示查询，避免信息不完整导致整条活动消失。
//! 主视口可使用紧凑摘要，转录则保留调用者已经提供的详细文本。
//! 本模块只渲染事件，不自行发起搜索或访问结果链接。
//! 结果缺失或调用失败时保留明确状态，不用空成功摘要掩盖协议结果。
//! 查询文本按当前宽度折行，状态前缀与正文样式保持分离。
//! Web-search activity history cells.

use super::*;

fn web_search_header(completed: bool) -> &'static str {
    if completed {
        "Searched the web"
    } else {
        "Searching the web"
    }
}

fn web_search_action_detail(action: &WebSearchAction) -> String {
    match action {
        WebSearchAction::Search { query, queries } => {
            query.clone().filter(|q| !q.is_empty()).unwrap_or_else(|| {
                let items = queries.as_ref();
                let first = items
                    .and_then(|queries| queries.first())
                    .cloned()
                    .unwrap_or_default();
                if items.is_some_and(|queries| queries.len() > 1) && !first.is_empty() {
                    format!("{first} ...")
                } else {
                    first
                }
            })
        }
        WebSearchAction::OpenPage { url } => url.clone().unwrap_or_default(),
        WebSearchAction::FindInPage { url, pattern } => match (pattern, url) {
            (Some(pattern), Some(url)) => format!("'{pattern}' in {url}"),
            (Some(pattern), None) => format!("'{pattern}'"),
            (None, Some(url)) => url.clone(),
            (None, None) => String::new(),
        },
        WebSearchAction::Other => String::new(),
    }
}

fn web_search_detail(action: Option<&WebSearchAction>, query: &str) -> String {
    let detail = action.map(web_search_action_detail).unwrap_or_default();
    if detail.is_empty() {
        query.to_string()
    } else {
        detail
    }
}

#[derive(Debug)]
pub(crate) struct WebSearchCell {
    call_id: String,
    query: String,
    action: Option<WebSearchAction>,
    start_time: Instant,
    completed: bool,
    animations_enabled: bool,
}

impl WebSearchCell {
    pub(crate) fn new(
        call_id: String,
        query: String,
        action: Option<WebSearchAction>,
        animations_enabled: bool,
    ) -> Self {
        Self {
            call_id,
            query,
            action,
            start_time: Instant::now(),
            completed: false,
            animations_enabled,
        }
    }

    pub(crate) fn call_id(&self) -> &str {
        &self.call_id
    }

    pub(crate) fn update(&mut self, action: WebSearchAction, query: String) {
        self.action = Some(action);
        self.query = query;
    }

    pub(crate) fn complete(&mut self) {
        self.completed = true;
    }
}

impl HistoryCell for WebSearchCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let bullet = if self.completed {
            "•".dim()
        } else {
            activity_indicator(
                Some(self.start_time),
                MotionMode::from_animations_enabled(self.animations_enabled),
                ReducedMotionIndicator::StaticBullet,
            )
            .unwrap_or_else(|| "•".dim())
        };
        let header = web_search_header(self.completed);
        let detail = web_search_detail(self.action.as_ref(), &self.query);
        let text: Text<'static> = if detail.is_empty() {
            Line::from(vec![header.bold()]).into()
        } else {
            let separator = if self.completed { " for " } else { " " };
            Line::from(vec![header.bold(), separator.into(), detail.into()]).into()
        };
        PrefixedWrappedHistoryCell::new(text, vec![bullet, " ".into()], "  ").display_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let header = web_search_header(self.completed);
        let detail = web_search_detail(self.action.as_ref(), &self.query);
        if detail.is_empty() {
            vec![Line::from(header)]
        } else {
            let separator = if self.completed { " for " } else { " " };
            vec![Line::from(format!("{header}{separator}{detail}"))]
        }
    }
}

pub(crate) fn new_active_web_search_call(
    call_id: String,
    query: String,
    animations_enabled: bool,
) -> WebSearchCell {
    WebSearchCell::new(call_id, query, /*action*/ None, animations_enabled)
}

pub(crate) fn new_web_search_call(
    call_id: String,
    query: String,
    action: WebSearchAction,
) -> WebSearchCell {
    let mut cell = WebSearchCell::new(
        call_id,
        query,
        Some(action),
        /*animations_enabled*/ false,
    );
    cell.complete();
    cell
}
