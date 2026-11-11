# 调研依据与边界

调研日期：2026-11-11。ShiJu 基线 HEAD：96f95ca6a，工作区存在大量其他任务的未提交变更；本次仅新增计划目录。Codex 参考目录不是 Git checkout，不能提供提交号；仅作为本地源码参考。

## 已读入口

| 来源 | 观察及实现意义 |
| --- | --- |
| `Cargo.toml`、`crates/vw-desktop/Cargo.toml` | 桌面是 Iced 0.14，已有原生/wasm 条件依赖，未发现桌面录音或听写实现；不得把通道配置中的 Telnyx 当桌面语音。 |
| `crates/vw-desktop/src/app/config_desktop.rs` | 桌面偏好由网关持久化，按 endpoint/tenant 隔离并合并并发写入。 |
| `crates/vw-desktop/src/app/config_system_settings.rs` | 现有设置分区及网关启动缓存；设备 ID、热键权限和录音文件是本机信息，不能套用远端主机设备。 |
| `crates/vw-desktop/src/app/impls/app_subscription.rs` | 已有窗口/键盘事件注册，但窗口事件不等于系统全局热键。 |
| `crates/vw-desktop/src/app/message/chat/input.rs` | 草稿编辑入口；听写结果不应自动发送或覆盖切换后的会话草稿。 |
| `crates/vw-desktop/src/app/components/input_panel/send_controls.rs` | 发送/停止按钮已有在途修改，新增麦克风需保留其行为。 |
| `crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal.rs` | 复用标准表单与弹窗；中文不得使用等宽字体。 |
| `crates/vw-agent/src/gateway/api/handlers/desktop.rs` | 已有桌面偏好路由，语音业务另建处理器，复用鉴权与 TenantScope。 |
| `crates/vw-gateway-client/src/client/desktop_settings_api.rs`、两个服务 crate 的 Cargo.toml | 已有 HTTP 和 WebSocket 依赖，优先沿用网络设施。 |
| `crates/vw-agent/src/tools/screenshot/mod.rs` | 现有截图使用安全策略；网关主机截图不能冒充桌面客户端前台上下文。 |
| `scripts/bundle_macos.sh` | 已给嵌入网站设置麦克风说明，需扩充为原生语音用途，实际签名使用 macos_webview_entitlements.plist，已含 audio-input；保留即可，不重复增权。 |
| `/Users/xiangmin/worker/code/codex-main/codex-rs/voice-host/README.md` | 明确是私有 helper 基础；设备默认静音、队列有界、generation 拒绝旧采样；尚有集成/媒体实证边界，不能当作可直接交付的完整语音。 |
| `/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/app/realtime_settings.rs` | 设备与音色分开持久化，音色来自服务目录；借鉴行为，不复制未知 API。 |
| `/Users/xiangmin/worker/code/codex-main/codex-rs/codex-api/src/endpoint/realtime_call.rs` | 含公开接口与 backend-api 私有请求形状分支；ShiJu 不接私有分支。 |
| `/Users/xiangmin/worker/code/codex-main/codex-rs/app-server-protocol/src/protocol/v2/realtime.rs` | 会话、转录片段、工作交接与关闭结果是不同事件；避免字幕增量重复触发代理。 |

RustCodeGraph 已存在并用于入口导航；未索引文件与配置用 rg/读取补充。无需初始化或重建索引。

## 外部接口依据

以下官方文档已在规划时查询；具体模型名、格式、事件和音色在任务 1 固化快照，并在实际接入时重新验证。不能从截图推出 API 音色可用性。

- [音频转写](https://developers.openai.com/api/docs/guides/transcription)：文件转写和实时转写是不同工作流。
- [双向 WebSocket](https://developers.openai.com/api/docs/guides/voice-websockets)：认证及双向音频事件；静音本身不停止已生成的回答。
- [实时对话](https://developers.openai.com/api/docs/guides/realtime-conversations)：中断需同步取消服务生成与本机排队播放。

## 方案权衡

1. 桌面直接请求服务：接入少一跳，但引入桌面密钥管理和重复网络策略；不选。
2. 网关持有凭据、桌面持有物理设备：复用租户、审批和服务配置；多一跳但可验证、易隔离，选择此方案。
3. 移植 Codex 私有 helper/WebRTC/GStreamer：媒体能力丰富，但私有服务形状、打包与原生依赖成本高；本阶段不选。采集/播放先选最小原生音频依赖，不引入整套媒体运行时。

## 成功标准及需求映射

| 需求 | 任务 | 验收 |
| --- | --- | --- |
| 截图 1 输入框麦克风 | 3–7 | 点击录音、完成回填、取消、忙碌态、原发送停止行为保持正确 |
| 截图 2 设备、语言、词典 | 2、4、5、13 | 设备切换持久化、自动检测/指定语言、词典作为识别提示 |
| 截图 2/3 语音聊天、音色、试听 | 8–13 | 双向音频、字幕去重、静音/结束/打断、真实可用音色 |
| 语音执行 ShiJu 工作 | 12 | 一次交接、复用审批、状态与最终答复返回语音会话 |
| 截图 2/4 全局热键及跨应用听写 | 3、15、16 | 按住/松开、双击免提、独立热键、权限引导、固定目标插入 |
| 截图 2 最近二十段录音 | 14 | 用户主动开启后仅本机保存，上限二十、可播放/重试/删除 |
| 截图 2 屏幕上下文 | 17 | 开关默认关闭，每次分享有可见状态；权限不足不采集 |
| 正式桌面可用 | 18、19 | 本机安装包权限、实际窗口渲染、真人麦克风/扬声器验证 |

## 全局约束

- 尚未获得任何真实音频服务凭据、麦克风实测、跨平台结果；不得编造成功。缺实机/凭据时保留“已完成，待回归”，不能用 mock 宣称完成真实验收。
- 只参考截图交互，保留 ShiJu 名称与设计系统。系统授权由用户在系统对话框主动完成；代理不得代点、修改 TCC 数据库或绕过权限。
- 非语音聊天的听写只回填；跨应用插入仅对用户启动时选定且结束时仍匹配的可编辑目标生效。
- 语音历史默认不保留；开启时明确最近二十段仅本机保存。不得记录原始音频、字幕、截图、服务凭据到 tracing。
- 服务模型与音色、识别语言、词典归网关用户配置；本机设备、权限、快捷键与录音归本机，按已有 project_dirs 路径解析存储，不能新建第二套连接/凭据配置。
- 所有任务串行：共享 App/Message/设置注册和 Cargo 清单；槽位协议仅解决构建缓存互斥，不能证明源码可并行。
