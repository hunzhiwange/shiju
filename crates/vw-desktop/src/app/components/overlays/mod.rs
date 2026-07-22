//! Overlay 定位组件。
//!
//! 本模块封装 Iced overlay 的定位、尺寸裁剪和外部点击关闭行为。

pub mod above;
/// below 子模块承载当前组件的一部分独立职责。
pub mod below;
/// inline_right 子模块承载当前组件的一部分独立职责。
pub mod inline_right;
/// left 子模块承载当前组件的一部分独立职责。
pub mod left;
/// point_below 子模块承载当前组件的一部分独立职责。
pub mod point_below;
/// side 子模块承载当前组件的一部分独立职责。
pub mod side;

/// 重新导出 above::{AboveOverlay, PointAboveOverlay}，让上层模块通过稳定路径访问。
pub use above::{AboveOverlay, PointAboveOverlay};
/// 重新导出 below::BelowOverlay，让上层模块通过稳定路径访问。
pub use below::BelowOverlay;
/// 重新导出 inline_right::InlineRightOverlay，让上层模块通过稳定路径访问。
pub use inline_right::InlineRightOverlay;
/// 重新导出 left::{LeftOverlay, PointLeftOverlay}，让上层模块通过稳定路径访问。
pub use left::{LeftOverlay, PointLeftOverlay};
/// 重新导出 point_below::PointBelowOverlay，让上层模块通过稳定路径访问。
pub use point_below::PointBelowOverlay;
/// 重新导出 side::SideOverlay，让上层模块通过稳定路径访问。
pub use side::SideOverlay;
