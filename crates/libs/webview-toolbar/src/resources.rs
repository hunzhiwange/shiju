//! 工具栏 HTML、脚本与共享图标引用。

const ARROW_LEFT_ICON: &str = include_str!("../../../../assets/icons/bootstrap/arrow-left.svg");
const ARROW_RIGHT_ICON: &str = include_str!("../../../../assets/icons/bootstrap/arrow-right.svg");
const RELOAD_ICON: &str = include_str!("../../../../assets/icons/bootstrap/arrow-clockwise.svg");
const GLOBE_ICON: &str = include_str!("../../../../assets/icons/bootstrap/globe.svg");
const CLOSE_ICON: &str = include_str!("../../../../assets/icons/bootstrap/x-lg.svg");

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

pub fn toolbar_html() -> String {
    TOOLBAR_TEMPLATE
        .replace("__ARROW_LEFT__", ARROW_LEFT_ICON)
        .replace("__ARROW_RIGHT__", ARROW_RIGHT_ICON)
        .replace("__RELOAD__", RELOAD_ICON)
        .replace("__GLOBE__", GLOBE_ICON)
        .replace("__CLOSE__", CLOSE_ICON)
}

pub fn toolbar_address_script(address: &str) -> Result<String, serde_json::Error> {
    let address = serde_json::to_string(address)?;
    Ok(format!("window.VibeWindowToolbar?.setAddress({address});"))
}
