# 浏览器反爬兼容计划

目标：降低默认浏览器后端暴露的标准自动化指纹，使普通网页访问不再因 `navigator.webdriver` 等基础信号被普遍拦截。

范围：修正 `agent-browser` 默认后端和 Rust Native WebDriver 后端的 Chrome 启动参数；保留域名白名单、URL 校验、会话隔离和现有后端选择；增加聚焦测试与真实浏览器信号验证。

范围外：不实现验证码破解、代理轮换、TLS/硬件指纹伪造、站点访问控制绕过，也不改变 `web_fetch`、`http_request` 或 Computer Use 后端。

假设：当前默认后端为 `agent_browser`，本机 `agent-browser 0.31.1` 支持 `--args`；Rust Native 通过 ChromeDriver 能力启动 Chrome。两条路径目前都未关闭 Blink 的 `AutomationControlled` 特征。

## 设计决策

- 采用 Browser Use 等现有 agent 浏览器使用的 `--disable-blink-features=AutomationControlled` 启动参数，同时覆盖默认 `agent-browser` 与 Rust Native 后端。
- 不伪造固定 User-Agent，避免 User-Agent、Client Hints 与真实浏览器版本不一致形成新的指纹。
- 不引入 stealth 插件或新依赖；仅使用现有 CLI 和 ChromeDriver 已支持的启动参数。
- 保留用户显式提供的 `AGENT_BROWSER_ARGS`，在缺少目标参数时追加，不覆盖已有参数。

## 架构说明

- `BrowserConfig` 经 `tools/toolset/mod.rs` 构造 `BrowserTool`；默认后端命令集中在 `agent_browser.rs`，原生会话能力集中在 `native_backend/session.rs`。
- 修复只作用于浏览器进程创建，不放宽 `SecurityPolicy`、域名白名单、私网限制或 URL scheme 校验。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。

