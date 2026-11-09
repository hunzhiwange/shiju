# WebView 权限与兼容性修复计划

目标：修复普通 WebView 和 MCP 应用中已识别的兼容性拦截，让授权操作可完成，失败有明确反馈。

范围：普通 WebView 的原生新窗口、登录和表单导出、系统协议、摄像头和麦克风。MCP 应用的声明联网、表单提交、外链及下载。覆盖 macOS、Windows、Linux 可发货平台，回归已修复的附件、Blob 和 Data 链路。

范围外：操作系统隐私策略、任意网页的本地文件或命令权限、MCP 工具授权变更、对外发布及与本次排查无关的浏览器能力。

假设：以现有桌面产品行为和提交 `2248ce283` 为基线。macOS 已实测新窗口表单 POST 变成 GET，其他平台和媒体设备行为由任务复核。把「放开」落实为各操作真实可用，权限来自用户动作、可信宿主或系统授权。

## 设计决策

- 原生新窗口保留请求及会话。重放 URL 改写请求，逐平台维护私有补丁成本较高，优先采用满足行为验证的 Wry 公共接口。
- MCP 通过宿主授予精确来源和操作权限，保留 iframe 隔离。

## 架构说明

- 普通浏览器由 `vw-webview` 和 Tao/Wry 管理，MCP 由独立助手及桌面桥接管理。
- 反馈使用对应界面的现有通知方式，系统设备授权由操作系统负责。敏感 URL、凭据和文件内容不进入日志。

## 开发策略

- 行为变更先写可观察的失败验证，再做最小修复。保留文件字节、网络请求、RPC 结果或设备采集证据。
- 优先用本地受控页面、真实 WebView 和有限的合成数据。无原生运行环境时记录平台缺口，保留待回归任务。
- 按编号线性执行，每批一项。普通浏览器共享启动及窗口代码，MCP 共享沙箱和桥接代码，原生验证还会争用图形桌面和媒体设备。

## 参考

- [Wry 0.56.0 发布说明](https://github.com/tauri-apps/wry/releases/tag/wry-v0.56.0)
- [MCP Apps 2026-01-26 规范](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx)
- [MCP Apps 下载扩展草案](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx)
- [Apple 媒体采集授权](https://developer.apple.com/documentation/avfoundation/requesting-authorization-to-capture-and-save-media)
