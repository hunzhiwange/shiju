use crate::views::project::layout::work_sidebar::{self, WorkSidebarModel};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::{Font, Pixels, Size};
use work_sidebar::presentation::*;
const VIEWPORT: Size = Size::new(320.0, 800.0);
struct SidebarViewFixture {
    tab: WorkSidebarTab,
    width: f32,
}
impl Default for SidebarViewFixture {
    fn default() -> Self {
        Self { tab: WorkSidebarTab::Tasks, width: 280.0 }
    }
}
impl SidebarViewFixture {
    fn view(&self) -> iced::Element<'static, work_sidebar::events::Message> {
        work_sidebar::view(
            &work_sidebar::ViewInput {
                desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                recent_projects: &[],
                project_sessions_loading: &Default::default(),
                work_sidebar_collapsed: false,
                work_sidebar_more_open: false,
                work_sidebar_filter_open: false,
                work_sidebar_project_nav_hovered: false,
                work_sidebar_filter_scope: WorkSidebarFilterScope::All,
                work_sidebar_filter_status: WorkSidebarFilterStatus::All,
                work_sidebar_filter_time: WorkSidebarFilterTime::All,
                work_sidebar_tab: self.tab,
                work_sidebar_width: self.width,
                new_session_creating: false,
            },
            &WorkSidebarModel { tasks: vec![], spaces: vec![], task_total: 0, space_total: 0 },
            None,
            &|_| unreachable!("empty sidebar has no task rows"),
        )
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_replaces_search_row_with_compact_header() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = SidebarViewFixture::default();
    let mut sidebar = app.view();
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(sidebar.as_widget());
    let node = sidebar.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let column = Layout::new(&node).child(1).child(0);

    assert_eq!(app.tab, WorkSidebarTab::Tasks);
    assert_eq!(
        column.children().count(),
        3,
        "navigation and tabs should stay outside the scrolling list"
    );
    assert_eq!(column.child(0).children().count(), 2, "expanded navigation keeps its header");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_sidebar_uses_resizable_width_from_app_state() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = SidebarViewFixture::default();
    app.width = 420.0;
    let mut sidebar = app.view();
    let limits = layout::Limits::new(Size::ZERO, Size::new(520.0, 800.0));
    let mut tree = Tree::new(sidebar.as_widget());
    let node = sidebar.as_widget_mut().layout(&mut tree, &renderer, &limits);

    assert_eq!(node.bounds().width, 420.0);
}
