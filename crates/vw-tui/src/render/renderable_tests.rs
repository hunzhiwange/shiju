//! 覆盖纵向 Renderable 布局对固定高度与弹性子项的空间分配。
//! 还验证同一帧的测量结果可跨布局、光标查询复用，避免重复昂贵计算。

use super::*;
use pretty_assertions::assert_eq;
use std::cell::Cell;

struct HeightRenderable(u16);

impl HeightRenderable {
    // 固定高度替身让用例只观察容器分配算法。
    fn with_height(height: u16) -> Self {
        Self(height)
    }
}

impl Renderable for HeightRenderable {
    fn render(&self, _area: Rect, _buf: &mut Buffer) {}

    fn desired_height(&self, _width: u16) -> u16 {
        self.0
    }
}

#[test]
// 较矮弹性子项未用完的空间应重新分给仍可增长的兄弟项。
fn flex_redistributes_space_unused_by_short_children() {
    let mut flex = FlexRenderable::new();
    flex.push(
        /*flex*/ 1,
        RenderableItem::Owned(Box::new(HeightRenderable::with_height(/*height*/ 20))),
    );
    flex.push(
        /*flex*/ 1,
        RenderableItem::Owned(Box::new(HeightRenderable::with_height(/*height*/ 2))),
    );

    let allocated =
        flex.allocate(Rect::new(/*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 10));

    assert_eq!(allocated.into_iter().map(|area| area.height).collect::<Vec<_>>(), vec![8, 2],);
}

#[test]
// 固定子项应先获得所需空间，剩余高度才由弹性子项按权重分配。
fn flex_reserves_non_flex_space_before_flexible_children() {
    let mut flex = FlexRenderable::new();
    flex.push(
        /*flex*/ 1,
        RenderableItem::Owned(Box::new(HeightRenderable::with_height(/*height*/ 20))),
    );
    flex.push(
        /*flex*/ 0,
        RenderableItem::Owned(Box::new(HeightRenderable::with_height(/*height*/ 2))),
    );
    flex.push(
        /*flex*/ 1,
        RenderableItem::Owned(Box::new(HeightRenderable::with_height(/*height*/ 20))),
    );

    let allocated =
        flex.allocate(Rect::new(/*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 10));

    assert_eq!(allocated.into_iter().map(|area| area.height).collect::<Vec<_>>(), vec![4, 2, 4],);
}

#[test]
// 一帧内的多次布局与光标查询应共享子项高度，不能重复触发测量。
fn flex_caches_child_height_across_frame_passes() {
    struct CountingRenderable<'a>(&'a Cell<usize>);

    impl Renderable for CountingRenderable<'_> {
        fn render(&self, _area: Rect, _buf: &mut Buffer) {}

        fn desired_height(&self, _width: u16) -> u16 {
            self.0.set(self.0.get() + 1);
            1
        }

        fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
            Some((area.x, area.y))
        }
    }

    let calls = Cell::new(0);
    let renderable = CountingRenderable(&calls);
    let mut flex = FlexRenderable::new();
    flex.push(/*flex*/ 1, RenderableItem::Borrowed(&renderable));
    let area = Rect::new(/*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 10);
    let mut buf = Buffer::empty(area);

    assert_eq!(flex.desired_height(area.width), 1);
    flex.render(area, &mut buf);
    assert_eq!(flex.cursor_pos(area), Some((0, 0)));
    assert!(matches!(flex.cursor_style(area), crossterm::cursor::SetCursorStyle::DefaultUserShape));
    assert_eq!(calls.get(), 1);

    assert_eq!(flex.desired_height(/*width*/ 100), 1);
    assert_eq!(calls.get(), 2);
}
