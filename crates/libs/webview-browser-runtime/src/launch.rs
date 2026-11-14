use tao::window::WindowBuilder;
use webview_launch::LaunchOptions;
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowBuilderExtMacOS;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowBuilderExtWindows;

pub(crate) fn window_builder(options: &LaunchOptions, embedded_mode: bool) -> WindowBuilder {
    let mut wb = WindowBuilder::new().with_title(options.window_title.clone());
    if embedded_mode {
        wb = wb.with_decorations(false).with_always_on_top(true);
        #[cfg(target_os = "macos")]
        {
            wb = wb.with_has_shadow(false);
        }
        #[cfg(target_os = "windows")]
        {
            wb = wb.with_undecorated_shadow(false);
        }
    }

    let size_w = options.width.unwrap_or(1280) as f64;
    let size_h = options.height.unwrap_or(800) as f64;
    wb = wb.with_inner_size(tao::dpi::LogicalSize::new(size_w, size_h));
    if let (Some(px), Some(py)) = (options.pos_x, options.pos_y) {
        wb = wb.with_position(tao::dpi::LogicalPosition::new(px as f64, py as f64));
    }

    wb
}
