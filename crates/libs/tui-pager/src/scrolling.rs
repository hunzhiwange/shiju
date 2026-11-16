//! Scroll calculations and visible-row rendering shared by the transcript pager.
//! 转录分页器共享的滚动计算与可见行渲染。

pub(super) fn max_scroll_offset(content_height: usize, viewport_height: usize) -> usize {
    // 饱和减法兼容内容短于视口的情况，并把零高视口按一行处理。
    content_height.saturating_sub(viewport_height.max(1))
}

pub(super) fn clamp_offset(offset: usize, content_height: usize, viewport_height: usize) -> usize {
    offset.min(max_scroll_offset(content_height, viewport_height))
}

#[cfg(test)]
#[path = "scrolling_tests.rs"]
mod tests;
