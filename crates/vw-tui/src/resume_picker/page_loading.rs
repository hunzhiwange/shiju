//! 保存会话选择器分页请求的令牌、游标来源与累计扫描进度。

use super::PageCursor;

/// Selects whether thread listing trusts indexed metadata or uses the store's normal behavior.
/// 选择线程列表仅读取索引元数据，还是采用存储层的常规行为。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageLoadMode {
    /// Return only the store's indexed metadata without repairing it from rollout files.
    /// 只返回存储层索引元数据，不从 rollout 文件修复。
    StateDbOnly,
    /// Use the store's normal listing behavior, which may include repair for local stores.
    /// 使用存储层常规列表行为，本地存储可能同时执行修复。
    StoreDefault,
}

/// Tracks the current request and the cursor policy inherited by its next page.
/// 跟踪当前请求，以及下一页继承的游标读取策略。
pub(super) struct PaginationState {
    pub(super) next_cursor: Option<PageCursor>,
    next_page_mode: PageLoadMode,
    pub(super) num_scanned_files: usize,
    pub(super) reached_scan_cap: bool,
    loading: LoadingState,
}

#[derive(Clone, Copy, Debug)]
enum LoadingState {
    Idle,
    Pending(PendingLoad),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingLoad {
    // 请求令牌防止旧响应收尾新请求，搜索令牌进一步约束所属查询批次。
    request_token: usize,
    pub(super) search_token: Option<usize>,
    pub(super) mode: PageLoadMode,
}

impl PaginationState {
    pub(super) fn new() -> Self {
        Self {
            next_cursor: None,
            next_page_mode: PageLoadMode::StoreDefault,
            num_scanned_files: 0,
            reached_scan_cap: false,
            loading: LoadingState::Idle,
        }
    }

    pub(super) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(super) fn start_load(
        &mut self,
        request_token: usize,
        search_token: Option<usize>,
        mode: PageLoadMode,
    ) {
        self.loading = LoadingState::Pending(PendingLoad {
            request_token,
            search_token,
            mode,
        });
    }

    /// Completes only the matching in-flight request, leaving newer requests untouched.
    /// 仅完成令牌匹配的在途请求，不影响更新的请求。
    pub(super) fn finish_load(&mut self, request_token: usize) -> Option<PendingLoad> {
        let LoadingState::Pending(pending) = self.loading else {
            return None;
        };
        if pending.request_token != request_token {
            return None;
        }
        self.loading = LoadingState::Idle;
        self.next_page_mode = pending.mode;
        Some(pending)
    }

    /// Records a page cursor and accumulates scan progress across the current listing.
    /// 记录分页游标，并在当前列表周期内累加扫描进度。
    pub(super) fn complete_page(
        &mut self,
        next_cursor: Option<PageCursor>,
        num_scanned_files: usize,
        reached_scan_cap: bool,
    ) {
        self.next_cursor = next_cursor;
        self.num_scanned_files = self.num_scanned_files.saturating_add(num_scanned_files);
        self.reached_scan_cap |= reached_scan_cap;
    }

    /// Returns the next cursor together with the listing mode that produced it.
    /// 返回下一页游标及生成该游标时采用的列表模式。
    pub(super) fn next_page(&self) -> Option<(PageCursor, PageLoadMode)> {
        if self.is_loading() {
            return None;
        }
        self.next_cursor
            .clone()
            .map(|cursor| (cursor, self.next_page_mode))
    }

    pub(super) fn is_loading(&self) -> bool {
        matches!(self.loading, LoadingState::Pending(_))
    }
}
