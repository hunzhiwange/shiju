use crate::app::App;
pub(crate) fn visible_paths(app: &App, root: &str) -> Vec<String> {
    super::adapter::inputs(app).visible_paths(root)
}
