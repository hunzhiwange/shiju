//! 独立 WebView 窗口的本地导航工具栏。

pub(crate) const TOOLBAR_REGION_HEIGHT: u32 = 46;

const ARROW_LEFT_ICON: &str =
    include_str!("../../../assets/icons/bootstrap/arrow-left.svg");
const ARROW_RIGHT_ICON: &str =
    include_str!("../../../assets/icons/bootstrap/arrow-right.svg");
const RELOAD_ICON: &str =
    include_str!("../../../assets/icons/bootstrap/arrow-clockwise.svg");
const GLOBE_ICON: &str = include_str!("../../../assets/icons/bootstrap/globe.svg");
const CLOSE_ICON: &str = include_str!("../../../assets/icons/bootstrap/x-lg.svg");

const TOOLBAR_TEMPLATE: &str = r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  :root {
    color-scheme: dark;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }
  * {
    box-sizing: border-box;
  }
  html, body {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: #10172a;
  }
  body {
    padding: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    padding: 4px 6px;
    gap: 5px;
    border: 1px solid #26365f;
    border-radius: 0;
    background: #182341;
  }
  button {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    margin: 0;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    color: rgba(240, 244, 255, 0.88);
    background: transparent;
    cursor: pointer;
  }
  button:hover {
    background: rgba(72, 88, 124, 0.62);
  }
  button:active {
    background: rgba(82, 101, 142, 0.72);
  }
  button svg {
    width: 16px;
    height: 16px;
    pointer-events: none;
  }
  #address {
    min-width: 0;
    height: 28px;
    flex: 1 1 auto;
    padding: 5px 10px;
    border: 1px solid #304575;
    border-radius: 12px;
    outline: none;
    color: #edf2ff;
    caret-color: #edf2ff;
    background: #202d53;
    font: 12px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }
  #address::placeholder {
    color: rgba(237, 242, 255, 0.42);
  }
  #address:focus {
    border-color: rgba(89, 131, 230, 0.82);
  }
</style>
</head>
<body>
  <div class="toolbar">
    <button type="button" data-action="back" title="后退">__ARROW_LEFT__</button>
    <button type="button" data-action="forward" title="前进">__ARROW_RIGHT__</button>
    <input id="address" data-action="navigate" type="text" placeholder="输入网址"
           autocomplete="off" autocapitalize="off" spellcheck="false">
    <button type="button" data-action="reload" title="刷新">__RELOAD__</button>
    <button type="button" data-action="open_external" title="在外部浏览器中打开">__GLOBE__</button>
    <button type="button" data-action="close" title="关闭">__CLOSE__</button>
  </div>
<script>
  const address = document.getElementById('address');

  function post(type, url) {
    if (!window.ipc || typeof window.ipc.postMessage !== 'function') {
      return;
    }
    const message = url === undefined ? { type } : { type, url };
    window.ipc.postMessage(JSON.stringify(message));
  }

  document.querySelectorAll('button[data-action]').forEach((button) => {
    button.addEventListener('click', () => {
      const action = button.dataset.action;
      if (action === 'open_external') {
        post(action, address.value);
      } else {
        post(action);
      }
    });
  });

  address.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      address.select();
      return;
    }
    if (event.key === 'Enter') {
      post('navigate', address.value);
      address.blur();
    }
  });

  window.VibeWindowToolbar = {
    setAddress(value) {
      address.value = String(value || '');
    }
  };
</script>
</body>
</html>
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolbarAction {
    Back,
    Forward,
    Navigate(String),
    Reload,
    OpenExternal(String),
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewRegion {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl ViewRegion {
    pub(crate) fn to_wry_rect(self) -> wry::Rect {
        wry::Rect {
            position: wry::dpi::LogicalPosition::new(self.x, self.y).into(),
            size: wry::dpi::LogicalSize::new(self.width, self.height).into(),
        }
    }
}

pub(crate) fn toolbar_html() -> String {
    TOOLBAR_TEMPLATE
        .replace("__ARROW_LEFT__", ARROW_LEFT_ICON)
        .replace("__ARROW_RIGHT__", ARROW_RIGHT_ICON)
        .replace("__RELOAD__", RELOAD_ICON)
        .replace("__GLOBE__", GLOBE_ICON)
        .replace("__CLOSE__", CLOSE_ICON)
}

pub(crate) fn toolbar_address_script(address: &str) -> Result<String, serde_json::Error> {
    let address = serde_json::to_string(address)?;
    Ok(format!(
        "window.VibeWindowToolbar?.setAddress({address});"
    ))
}

pub(crate) fn parse_toolbar_ipc_message(message: &str) -> Option<ToolbarAction> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    match value.get("type").and_then(serde_json::Value::as_str)? {
        "back" => Some(ToolbarAction::Back),
        "forward" => Some(ToolbarAction::Forward),
        "navigate" => toolbar_url(&value).map(ToolbarAction::Navigate),
        "reload" => Some(ToolbarAction::Reload),
        "open_external" => toolbar_url(&value).map(ToolbarAction::OpenExternal),
        "close" => Some(ToolbarAction::Clear),
        _ => None,
    }
}

fn toolbar_url(value: &serde_json::Value) -> Option<String> {
    normalize_toolbar_url(value.get("url")?.as_str()?)
}

fn normalize_toolbar_url(input: &str) -> Option<String> {
    let trimmed = input.trim().trim_matches(['`', '"', '\'']).trim();
    if trimmed.is_empty()
        || trimmed.chars().any(|character| character.is_whitespace() || character.is_control())
    {
        return None;
    }

    let target =
        if trimmed.contains("://") { trimmed.to_string() } else { format!("https://{trimmed}") };
    let (scheme, remainder) = target.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority =
        remainder.split_once(['/', '?', '#']).map_or(remainder, |(authority, _)| authority);
    authority.rsplit('@').next().is_some_and(|host| !host.is_empty()).then_some(target)
}

pub(crate) fn window_regions(
    width: u32,
    height: u32,
    independent: bool,
) -> (Option<ViewRegion>, ViewRegion) {
    if !independent {
        return (None, ViewRegion { x: 0, y: 0, width, height });
    }

    let toolbar_height = TOOLBAR_REGION_HEIGHT.min(height);
    (
        Some(ViewRegion { x: 0, y: 0, width, height: toolbar_height }),
        ViewRegion {
            x: 0,
            y: toolbar_height,
            width,
            height: height.saturating_sub(toolbar_height),
        },
    )
}
