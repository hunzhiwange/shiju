# ACP 模型来源与 CLI 兼容性

截至 2026-11-05，整体仍未达到 22/22 全量交付。下表的历史证据仅适用于固定发行版和 Darwin arm64；本轮回归以[脱敏索引](acp-model-source/evidence/regression-35.json)为准。Linux、Windows、macOS x86_64 和其他发行版均未验证。

新建会话默认 `session_model`，使用客户端选定的完整 Provider/model；`cli_local` 使用 CLI 执行主机上的原生配置。远程网关执行时，该主机是网关所在主机。更改来源创建新会话身份，同来源空闲切模先验证新配置再替换。忙碌时拒绝变更；模型禁用、密钥撤销或协议不支持时明确失败，不回退厂商账号。

| CLI | 固定发行版 | 历史双来源证据 / 限制 |
| --- | --- | --- |
| auggie | 0.36.0 | Provider registration rejected by fixed upstream |
| claude | 0.26.0 | [历史真实请求](acp-model-source/evidence/claude.md) |
| codex | 2.5.0 | [历史真实请求](acp-model-source/evidence/codex.md) |
| codebuddy | 2.179.3 | [历史真实请求](acp-model-source/evidence/codebuddy.md) |
| goose | v1.57.0 | [历史真实请求](acp-model-source/evidence/goose.md) |
| vibe | 3.0.4 | [历史真实请求](acp-model-source/evidence/vibe.md) |
| hermes | 0.19.0 | [历史真实请求](acp-model-source/evidence/hermes.md) |
| snow | 0.9.14 | [历史真实请求](acp-model-source/evidence/snow.md) |
| copilot | 1.0.105 | [历史真实请求](acp-model-source/evidence/copilot.md) |
| cursor | 2026.10.22-27dcf5f | No verified arbitrary CLI Provider injection contract |
| droid | 1.3.0 | 原 CLI 双向 ACP 已响应，但新建会话要求 Factory 认证；完整来源隔离未验收 |
| gemini | 0.66.0 | [历史真实请求](acp-model-source/evidence/gemini.md) |
| iflow | 0.5.19 | [历史真实请求](acp-model-source/evidence/iflow.md) |
| kilocode | 7.8.18 | [历史真实请求](acp-model-source/evidence/kilocode.md) |
| kimi | 2.5.0 | [历史真实请求](acp-model-source/evidence/kimi.md) |
| kiro | 3.0.0 | [历史真实请求](acp-model-source/evidence/kiro.md) |
| opencode | 1.19.5 | [历史真实请求](acp-model-source/evidence/opencode.md) |
| openclaw | 2026.10.10 | [历史真实请求](acp-model-source/evidence/openclaw.md) |
| pi | 0.0.22 | [历史真实请求](acp-model-source/evidence/pi.md) |
| qoder | 1.1.82 | Authorized personal-account login and Custom Provider entitlement required |
| qwen | 0.26.7 | [历史真实请求](acp-model-source/evidence/qwen.md) |
| trae | 0.121.3 | Authorized platform login and fixed-version runtime-root isolation required |

补丁运行时必须通过现有安装器安装已审查的精确发行物，不能用全局任意同名 CLI 替代；原生运行时也只对表列版本有证据。详见[安装、运行时身份与会话边界](acp-model-source/integration.md)。

Qoder 已找到官方 SDK 的 Custom Provider 接口，但实际注册需要获授权的个人账号及相应权限。TRAE 0.121.3 已证明可枚举和选择自定义模型，但无登录环境在模型请求前失败，且专属运行时目录隔离尚未证明。两者不能继续描述为“没有自定义 Provider 接口”，也不能把握手、模型枚举或认证负向测试计作通过。Auggie 的 Provider 注册被实际发行版拒绝；Cursor 的 endpoint 候选仍调用平台认证/控制面；Droid 的 ACP 双向握手可用，但会话创建要求 Factory 认证。详见[本轮上游关口复验](acp-model-source/evidence/upstream-gates-35.md)。

验收只模拟外部模型 HTTP 服务；真实 CLI、ACP、工具读取、多轮、取消、进程恢复及来源隔离必须实际运行。每个 CLI 两种来源各需至少一个具名 ignored 测试，观察到真实模型请求并通过全部生命周期断言。普通聚焦测试中的 ignored 项不计作供应商通过。

安全回归覆盖租户/会话/Provider/模型/generation 绑定、同目录隔离、撤销、禁用及重定向拒绝。auto 在轮次开始解析为固定真实模型。协议转换存在显式能力边界，不支持的媒体、托管工具、思考签名及跨身份引用拒绝转发。完整证据见各 CLI 页和本轮索引；单元测试不能补足未运行的跨平台或供应商入口。

上游最小契约必须允许原 CLI 的 ACP 路径接收任意 endpoint、认证与 model，并约束主/辅助/恢复请求、配置优先级和独立运行时目录；需要平台登录的入口须先取得明确授权，不伪造账号状态。Auggie 需证明固定发行版加载 Provider 扩展，Cursor 需正式任意 Provider 入口，Droid 需获授权的 Factory 认证及完整来源隔离证明；Qoder/TRAE 还需各自认证和权限的真实证明。

恢复全量验收需先解除五项上游缺口并补齐两条来源的真实请求，再由原会话收尾任务24/26状态。未满足前保留任务35为已阻塞。

本轮指定五 crate 聚焦回归以 `RUST_MIN_STACK=33554432` 和 `--test-threads=1` 执行，157 项通过；全仓 Clippy 退出0。真实来源路径34/44通过；10条不可用保留失败原因。初次默认栈溢出及并行环境干扰均保留在索引，不算通过。
