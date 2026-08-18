//! 管理桌面端停靠 WebView 辅助进程、控制协议与停靠几何。

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crate::app::Screen;
use crate::app::state::{WorkResourceMode, WorkspaceMode};

const WEBVIEW_HELPER_NAME: &str = if cfg!(windows) { "vw-webview.exe" } else { "vw-webview" };
const NAVIGATION_EVENT_PREFIX: &str = "navigation ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WebviewBounds {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkBrowserLayout {
    pub(crate) window_x: f32,
    pub(crate) window_y: f32,
    pub(crate) window_width: f32,
    pub(crate) window_height: f32,
    pub(crate) work_sidebar_width: f32,
    pub(crate) work_divider_width: f32,
    pub(crate) top_bar_height: f32,
    pub(crate) resource_header_height: f32,
    pub(crate) horizontal_inset: f32,
    pub(crate) bottom_inset: f32,
    pub(crate) split_ratio: f32,
    pub(crate) resource_fullscreen: bool,
    pub(crate) resource_half_fullscreen: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkBrowserSurface {
    pub(crate) screen: Screen,
    pub(crate) workspace_mode: WorkspaceMode,
    pub(crate) resource_mode: WorkResourceMode,
    pub(crate) resources_visible: bool,
    pub(crate) system_settings_open: bool,
    pub(crate) chat_fullscreen: bool,
    pub(crate) chat_half_fullscreen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkBrowserLaunch {
    pub(crate) target: String,
    pub(crate) bounds: WebviewBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkBrowserEvent {
    NavigationChanged(String),
}

pub(crate) trait BrowserProcess: Send {
    fn write_protocol(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn take_events(&mut self) -> Vec<WorkBrowserEvent>;
    fn is_running(&mut self) -> io::Result<bool>;
    fn terminate(&mut self) -> io::Result<()>;
    fn terminate_now(&mut self) -> io::Result<()>;
}

struct ChildBrowserProcess {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<WorkBrowserEvent>,
}

impl BrowserProcess for ChildBrowserProcess {
    fn write_protocol(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.stdin.write_all(bytes)?;
        self.stdin.flush()
    }

    fn take_events(&mut self) -> Vec<WorkBrowserEvent> {
        self.events.try_iter().collect()
    }

    fn is_running(&mut self) -> io::Result<bool> {
        self.child.try_wait().map(|status| status.is_none())
    }

    fn terminate(&mut self) -> io::Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        let _ = self.child.wait();
        Ok(())
    }

    fn terminate_now(&mut self) -> io::Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct WorkBrowserController {
    process: Option<Box<dyn BrowserProcess>>,
    last_bounds: Option<WebviewBounds>,
    visible: bool,
    always_on_top: bool,
}

impl fmt::Debug for WorkBrowserController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkBrowserController")
            .field("has_process", &self.process.is_some())
            .field("last_bounds", &self.last_bounds)
            .field("visible", &self.visible)
            .field("always_on_top", &self.always_on_top)
            .finish()
    }
}

impl WorkBrowserController {
    pub(crate) fn ensure_started(
        &mut self,
        target: &str,
        bounds: WebviewBounds,
    ) -> Result<bool, String> {
        self.ensure_started_with(target, bounds, |launch| spawn_embedded_webview(&launch))
    }

    pub(crate) fn ensure_started_with<F>(
        &mut self,
        target: &str,
        bounds: WebviewBounds,
        spawn: F,
    ) -> Result<bool, String>
    where
        F: FnOnce(&WorkBrowserLaunch) -> Result<Box<dyn BrowserProcess>, String>,
    {
        if self.has_running_child() {
            return Ok(false);
        }
        self.process = None;
        self.visible = false;

        let target = normalize_navigation_url(target)?;
        let launch = WorkBrowserLaunch { target, bounds };
        self.process = Some(spawn(&launch)?);
        self.last_bounds = Some(bounds);
        self.always_on_top = true;
        Ok(true)
    }

    pub(crate) fn has_running_child(&mut self) -> bool {
        let Some(process) = self.process.as_mut() else {
            return false;
        };
        match process.is_running() {
            Ok(true) => true,
            Ok(false) | Err(_) => {
                self.process = None;
                self.visible = false;
                self.always_on_top = false;
                false
            }
        }
    }

    pub(crate) fn navigate(&mut self, target: &str) -> Result<(), String> {
        let target = normalize_navigation_url(target)?;
        self.send_command(&format!("navigate {target}"))
    }

    pub(crate) fn back(&mut self) -> Result<(), String> {
        self.send_command("back")
    }

    pub(crate) fn forward(&mut self) -> Result<(), String> {
        self.send_command("forward")
    }

    pub(crate) fn reload(&mut self) -> Result<(), String> {
        self.send_command("reload")
    }

    pub(crate) fn take_latest_navigation_url(&mut self) -> Option<String> {
        let process = self.process.as_mut()?;
        process
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                WorkBrowserEvent::NavigationChanged(url) => normalize_navigation_url(&url).ok(),
            })
            .next_back()
    }

    pub(crate) fn sync_bounds(&mut self, bounds: WebviewBounds) -> Result<(), String> {
        if self.last_bounds == Some(bounds) {
            return Ok(());
        }
        self.send_command(&format!("move {} {}", bounds.x, bounds.y))?;
        self.send_command(&format!("resize {} {}", bounds.width, bounds.height))?;
        self.last_bounds = Some(bounds);
        Ok(())
    }

    pub(crate) fn set_visible(&mut self, visible: bool) -> Result<(), String> {
        if self.visible == visible {
            return Ok(());
        }
        self.send_command(if visible { "show" } else { "hide" })?;
        self.visible = visible;
        Ok(())
    }

    pub(crate) fn set_always_on_top(&mut self, always_on_top: bool) -> Result<(), String> {
        if self.always_on_top == always_on_top {
            return Ok(());
        }
        self.send_command(if always_on_top { "raise" } else { "lower" })?;
        self.always_on_top = always_on_top;
        Ok(())
    }

    pub(crate) fn close(&mut self) -> Result<(), String> {
        let Some(mut process) = self.process.take() else {
            self.visible = false;
            self.last_bounds = None;
            self.always_on_top = false;
            return Ok(());
        };
        let protocol_result = process
            .write_protocol(b"close\n")
            .map_err(|error| format!("关闭 WebView 失败: {error}"));
        let terminate_result =
            process.terminate().map_err(|error| format!("终止 WebView 失败: {error}"));
        self.visible = false;
        self.last_bounds = None;
        self.always_on_top = false;
        protocol_result.and(terminate_result)
    }

    /// 关机专用快速终止：不写控制管道，也不等待子进程退出。
    pub(crate) fn shutdown(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.terminate_now();
        }
        self.visible = false;
        self.last_bounds = None;
        self.always_on_top = false;
    }

    fn send_command(&mut self, command: &str) -> Result<(), String> {
        let process = self.process.as_mut().ok_or_else(|| "内置浏览器进程尚未启动".to_string())?;
        let mut line = String::with_capacity(command.len() + 1);
        line.push_str(command);
        line.push('\n');
        process
            .write_protocol(line.as_bytes())
            .map_err(|error| format!("写入内置浏览器控制命令失败: {error}"))
    }
}

impl Drop for WorkBrowserController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn work_browser_bounds(layout: WorkBrowserLayout) -> Option<WebviewBounds> {
    let values = [
        layout.window_x,
        layout.window_y,
        layout.window_width,
        layout.window_height,
        layout.work_sidebar_width,
        layout.work_divider_width,
        layout.top_bar_height,
        layout.resource_header_height,
        layout.horizontal_inset,
        layout.bottom_inset,
        layout.split_ratio,
    ];
    if values.iter().any(|value| !value.is_finite())
        || (!layout.resource_fullscreen && layout.window_width <= layout.work_sidebar_width)
        || layout.window_height <= layout.top_bar_height + layout.resource_header_height
    {
        return None;
    }

    let sidebar_offset = if layout.resource_fullscreen { 0.0 } else { layout.work_sidebar_width };
    let work_width = layout.window_width - sidebar_offset;
    let resource_offset = if layout.resource_fullscreen || layout.resource_half_fullscreen {
        0.0
    } else {
        let content_width = work_width - layout.work_divider_width;
        if content_width < 1.0 {
            return None;
        }
        content_width * layout.split_ratio.clamp(0.25, 0.75) + layout.work_divider_width
    };
    let resource_width = work_width - resource_offset;
    let width = resource_width - layout.horizontal_inset * 2.0;
    let height = layout.window_height
        - layout.top_bar_height
        - layout.resource_header_height
        - layout.bottom_inset;
    if width < 1.0 || height < 1.0 {
        return None;
    }

    Some(WebviewBounds {
        x: (layout.window_x + sidebar_offset + resource_offset + layout.horizontal_inset).round()
            as i32,
        y: (layout.window_y + layout.top_bar_height + layout.resource_header_height).round() as i32,
        width: width.round() as u32,
        height: height.round() as u32,
    })
}

pub(crate) fn work_browser_surface_visible(surface: WorkBrowserSurface) -> bool {
    surface.screen == Screen::Project
        && surface.workspace_mode == WorkspaceMode::Work
        && surface.resource_mode == WorkResourceMode::Browser
        && surface.resources_visible
        && !surface.system_settings_open
        && !surface.chat_fullscreen
        && !surface.chat_half_fullscreen
}

pub(crate) fn normalize_navigation_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim().trim_matches(['`', '"', '\'']).trim();
    if trimmed.is_empty() {
        return Err("浏览器地址不能为空".to_string());
    }
    if trimmed.chars().any(|character| character.is_whitespace() || character.is_control()) {
        return Err("浏览器地址不能包含空白或控制字符".to_string());
    }

    let target =
        if trimmed.contains("://") { trimmed.to_string() } else { format!("https://{trimmed}") };
    let Some((scheme, remainder)) = target.split_once("://") else {
        return Err("浏览器地址缺少有效协议".to_string());
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err("内置浏览器仅允许 http 或 https 地址".to_string());
    }
    let authority =
        remainder.split_once(['/', '?', '#']).map_or(remainder, |(authority, _)| authority);
    if authority.rsplit('@').next().is_none_or(str::is_empty) {
        return Err("浏览器地址缺少主机名".to_string());
    }
    Ok(target)
}

pub(crate) fn webview_helper_candidates() -> Vec<PathBuf> {
    webview_helper_candidates_from(std::env::current_exe().ok().as_deref())
}

fn webview_helper_candidates_from(current_exe: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = current_exe.and_then(Path::parent) {
        candidates.push(path.join(WEBVIEW_HELPER_NAME));
    }
    if let Some(target_dir) = current_exe.and_then(|path| {
        path.ancestors().find(|ancestor| ancestor.file_name().is_some_and(|name| name == "target"))
    }) && let Some(root) = target_dir.parent()
    {
        candidates.push(root.join("target").join("release").join(WEBVIEW_HELPER_NAME));
        candidates.push(root.join("target").join("debug").join(WEBVIEW_HELPER_NAME));
    }
    candidates.push(PathBuf::from(WEBVIEW_HELPER_NAME));
    candidates.dedup();
    candidates
}

fn embedded_launch_args(launch: &WorkBrowserLaunch) -> Vec<String> {
    vec![
        launch.target.clone(),
        "--mode=embedded".to_string(),
        "--title=VibeWindow Browser".to_string(),
        format!("--width={}", launch.bounds.width),
        format!("--height={}", launch.bounds.height),
        format!("--x={}", launch.bounds.x),
        format!("--y={}", launch.bounds.y),
    ]
}

fn spawn_embedded_webview(launch: &WorkBrowserLaunch) -> Result<Box<dyn BrowserProcess>, String> {
    let args = embedded_launch_args(launch);
    let mut failures = Vec::new();
    for candidate in webview_helper_candidates() {
        let mut command = Command::new(&candidate);
        command.args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        match command.spawn() {
            Ok(mut child) => {
                let Some(stdin) = child.stdin.take() else {
                    let _ = child.kill();
                    let _ = child.wait();
                    failures.push(format!("{}: 无法获取标准输入", candidate.display()));
                    continue;
                };
                let Some(stdout) = child.stdout.take() else {
                    let _ = child.kill();
                    let _ = child.wait();
                    failures.push(format!("{}: 无法获取标准输出", candidate.display()));
                    continue;
                };
                let (event_tx, event_rx) = mpsc::channel();
                let reader =
                    thread::Builder::new().name("vw-webview-events".to_string()).spawn(move || {
                        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                            let Some(event) = parse_webview_event(&line) else {
                                continue;
                            };
                            if event_tx.send(event).is_err() {
                                break;
                            }
                        }
                    });
                if let Err(error) = reader {
                    let _ = child.kill();
                    let _ = child.wait();
                    failures
                        .push(format!("{}: 无法启动事件读取线程: {error}", candidate.display()));
                    continue;
                }
                return Ok(Box::new(ChildBrowserProcess { child, stdin, events: event_rx }));
            }
            Err(error) => failures.push(format!("{}: {error}", candidate.display())),
        }
    }

    Err(format!("无法启动 vw-webview 辅助程序：{}", failures.join("；")))
}

fn parse_webview_event(line: &str) -> Option<WorkBrowserEvent> {
    let payload = line.strip_prefix(NAVIGATION_EVENT_PREFIX)?;
    let url = serde_json::from_str::<String>(payload).ok()?;
    Some(WorkBrowserEvent::NavigationChanged(url))
}

#[cfg(test)]
#[path = "webview_runtime_tests.rs"]
mod work_browser_controller_tests;
