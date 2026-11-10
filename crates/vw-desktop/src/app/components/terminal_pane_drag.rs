use iced::advanced::widget::{Operation, operation};
use iced::widget::Id;
use iced::widget::pane_grid::{Edge, Pane, Region, Target};
use iced::{Point, Rectangle, Vector};

use super::{TAB_STRIP_ID, TAB_TITLE_ID};

pub(super) struct TabTitleHit {
    cursor: Point,
    strip: Option<(Rectangle, Vector)>,
    pub(super) hit: bool,
}

impl TabTitleHit {
    pub(super) fn new(cursor: Point) -> Self {
        Self { cursor, strip: None, hit: false }
    }
}

impl Operation for TabTitleHit {
    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.strip = (id == Some(&Id::from(TAB_STRIP_ID))).then_some((bounds, translation));
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&Id::from(TAB_TITLE_ID))
            && let Some((strip, translation)) = self.strip
            && strip.contains(self.cursor)
            && bounds.contains(self.cursor + translation)
        {
            self.hit = true;
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if !self.hit {
            operate(self);
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct TabDrag {
    pub(super) pane: Pane,
    pub(super) origin: Point,
    pub(super) moving: bool,
}

pub(super) fn drop_target(
    cursor: Point,
    grid: Rectangle,
    source: Pane,
    panes: impl Iterator<Item = (Pane, Rectangle)>,
) -> Option<(Target, Rectangle)> {
    if !grid.contains(cursor) {
        return None;
    }

    // An outer edge docks against the entire grid. Inside a pane, the center
    // swaps panes and an edge docks beside the destination.
    let inset = grid.width.min(grid.height) * 0.04;
    if let Some(edge) = edge_at(grid, cursor, inset, inset) {
        return Some((Target::Edge(edge), edge_preview(grid, edge)));
    }

    let (pane, bounds) =
        panes.filter(|(pane, _)| *pane != source).find(|(_, b)| b.contains(cursor))?;
    if let Some(edge) = edge_at(bounds, cursor, bounds.width / 3.0, bounds.height / 3.0) {
        Some((Target::Pane(pane, Region::Edge(edge)), edge_preview(bounds, edge)))
    } else {
        Some((Target::Pane(pane, Region::Center), bounds))
    }
}

fn edge_at(bounds: Rectangle, cursor: Point, x_inset: f32, y_inset: f32) -> Option<Edge> {
    if cursor.x < bounds.x + x_inset {
        Some(Edge::Left)
    } else if cursor.x > bounds.x + bounds.width - x_inset {
        Some(Edge::Right)
    } else if cursor.y < bounds.y + y_inset {
        Some(Edge::Top)
    } else if cursor.y > bounds.y + bounds.height - y_inset {
        Some(Edge::Bottom)
    } else {
        None
    }
}

fn edge_preview(bounds: Rectangle, edge: Edge) -> Rectangle {
    match edge {
        Edge::Left => Rectangle { width: bounds.width / 2.0, ..bounds },
        Edge::Right => {
            Rectangle { x: bounds.x + bounds.width / 2.0, width: bounds.width / 2.0, ..bounds }
        }
        Edge::Top => Rectangle { height: bounds.height / 2.0, ..bounds },
        Edge::Bottom => {
            Rectangle { y: bounds.y + bounds.height / 2.0, height: bounds.height / 2.0, ..bounds }
        }
    }
}
