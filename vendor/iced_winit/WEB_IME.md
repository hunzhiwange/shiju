# Web 输入法补丁

本目录基于 crates.io `iced_winit 0.14.0`（Iced 提交 `3997291f318a8bc06fa522f5579836fb3feb94df` 的 `winit` 目录），保留上游 MIT 许可证。根目录通过 `[patch.crates-io]` 使用本地实现；原生平台继续走上游输入法路径。

## 接入边界

不按页面注册事件。已有控件通过 `Shell::request_input_method` 请求输入法，`Window::request_input_method` 统一更新 DOM 代理；DOM 组合与提交事件转换为 `iced_core::Event::InputMethod`，进入相应窗口的正常事件队列。支持标准 `text_input`、`text_editor` 及遵守该协议的自定义控件。

每个 Canvas 维护一组普通 textarea 与 password input，只有当前用途对应的代理获得焦点。密码代理使用原生 `type=password`；不记录输入载荷，不写日志、存储或网络。代理值在提交、取消、失焦及销毁时清空。销毁窗口先移除 JS 监听，再释放 Rust 回调，避免泄漏与悬空回调。

候选位置来自控件实际光标，按 Iced viewport 缩放及 Canvas CSS 缩放转换；滚动和窗口变化时重新定位。Canvas 与代理之间的焦点转移不产生 Iced 窗口失焦；页面其他控件获得焦点时停止抢焦点。业务页面无需发送聊天专用消息。

上游 Web 键盘转换使用修饰后的字符；代理为快捷键还原字母大小写，普通文本从 DOM input 提交，组合过程的按键不转发。浏览器原生粘贴从 input 提交，避免与 Iced 的粘贴快捷键重复。

自定义控件如果未实现 Iced 输入协议，不会自动获得文本编辑能力。移动端虚拟键盘、各系统原生候选窗、浏览器剪贴板权限差异仍需各平台实机验收。本次不新增这些平台能力。

## 自动验证

需安装 Playwright 与 Chrome；通过 `NODE_PATH` 指向已有 Playwright 的 node_modules，或在环境中提供该包。

```sh
node scripts/tests/test_web_ime.mjs
```

测试真实 DOM 输入、Chromium IME 协议提交、组合事件顺序去重、取消、按键转发、坐标、安全用途、禁用和销毁。合成事件顺序重放不能代替系统输入法实机测试。

真实 Iced/WGPU 渲染示例使用仓库 Noto Sans CJK SC 字体，避免中文等宽字体整形问题：

```sh
cargo build --manifest-path vendor/iced_winit/Cargo.toml --target wasm32-unknown-unknown --example web_ime
wasm-bindgen --target web --out-dir /tmp/shiju-web-ime-dist "$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/examples/web_ime.wasm"
WEB_IME_DIST=/tmp/shiju-web-ime-dist node scripts/tests/test_web_ime_rendering.mjs
```

Cargo 阶段必须遵守计划的共享槽位规则。仓库的 `build.build-dir` 配置为公共 target，验证时另设 `CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"`，使中间产物也位于已领取槽位内。绑定工具版本必须匹配 wasm-bindgen 0.2.126。

渲染验收覆盖普通单行中文、选区替换、多行换行和密码框切换；截图默认保存在 `/tmp/shiju-web-ime-rendering.png`。示例仅用密码字符数报告结果，不暴露密码内容。浏览器无异常才能通过。

升级 Iced 时重新比对上游 `src/lib.rs` 与 `src/window.rs` 的补丁入口；本地新增入口为 `src/web_ime.rs` 和 `src/web_ime.js`。两处 futures channel 的弃用接口已等价改为 `try_recv`，Cargo lint key 使用下划线形式以避免新版 Cargo 弃用警告。
