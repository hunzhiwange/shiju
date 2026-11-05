# 与 new-shiju-agent 的实现对比

参考目录：`/Users/xiangmin/worker/code/new-shiju-agent`。只读源码；不把补丁存在等同于随包二进制已接入或真实模型已验收。

| 关注点 | 参考项目 | 当前仓库 | 本计划选择 |
| --- | --- | --- | --- |
| 模型来源 | `extra.shiju_cli_model` 有值走平台分支，否则原生配置 | ACP 请求以虚拟 model 路由，客户端真实选择可能被替换 | 独立模型来源枚举，默认客户端会话模型；CLI 本地来源显式选择 |
| 支持范围 | 前后端都限制 claude/codex/qwen/opencode | 22 个启动注册 | 全部 22 个列入能力矩阵，逐项完成证据；不将目录数量当模型支持数量 |
| 模型目录 | resolve 固定读取 `shiju-platform` | 原生完整 Provider 目录 | 引用当前租户与项目的完整 `provider/model`，不固定平台，不复制凭据 |
| 模型中继 | 桌面 Node HTTP loopback，随机令牌，真实 key 留主进程 | 未看到通用 CLI 模型中继 | 在实际运行 CLI 的 vw-agent/Gateway 主机创建 loopback 中继 |
| 协议 | 代理提供三个入口，主要转发；多协议依赖 NewAPI | 原生模型驱动已有转换、认证和流事件 | 同协议优先透传；跨协议在低层提供有测试的 Chat/Messages/Responses/Gemini 转换，不走 session 工具循环 |
| 配置覆盖 | 4 路 match 生成 args/env | command/args/env 已可注入 | 复用启动通道，按 CLI 原生入口生成覆盖；ACP 适配器与原生 CLI 参数分别核对 |
| 隔离 | 专属用户+backend目录，Qwen随机别名 | 缓存含 tenant、command、args、env、cwd | 状态按租户/来源/driver及必要会话范围隔离；缓存使用非秘密指纹；中继令牌按会话和代次约束 |
| 持久化 | 会话只存模型 ID；有专用切模服务 | SessionRecord 序列化 agent_config/agent_command | 新增运行时覆盖与持久化启动描述分离，禁止代理令牌进入记录、命令文本与日志 |
| 切模 | 空闲停进程、保存、下一轮恢复；不跨来源热切换 | ACP advertised model/config 控制已存在 | 区分客户端模型与 CLI advertised model；忙碌拒绝；原子重建和回滚；来源切换用新会话 |
| 默认值 | README 写默认 CLI 原有配置 | 当前无统一来源 | 按用户要求新建 ACP 会话默认客户端模型 |
| 上下文限制 | 额外补丁给 Codex用 Provider 下最小窗口 | 已有所选模型限制与 auto 选择 | 使用当前实际所选模型限制，每轮固定快照；不把全目录最小值当模型本身窗口 |

## 参考源码定位

- `packages/desktop/src/common/types/platform/shijuCli.ts`：4 个 backend 白名单，平台目录筛选，Qwen 模型别名还原。
- `packages/desktop/src/renderer/components/agent/AcpModelSelector/ShijuCliModelSelector.tsx`：平台/本地来源选择，查询后端能力。
- `packages/desktop/src/renderer/components/agent/AcpModelSelector/ShijuCliConversationModel.tsx`：保存会话模型并重新拉取运行态。
- `packages/desktop/shiju/cli-models/aioncore.patch`：`factory/shiju_cli.rs` 的 resolve/launch；ACP 工厂接线；后端能力接口；空闲切模和运行时停止。
- `packages/desktop/shiju/cli-models/aioncore-team.patch`：团队成员字段及校验；不能据此额外扩大本任务到团队调度重构。
- `packages/desktop/src/process/services/shiju/proxy/server.ts`：loopback 绑定、随机令牌、鉴权、路由、转发、流背压、超时和用量。
- `packages/desktop/src/process/services/shiju/proxy/index.ts`：账号代次检查、令牌失效、请求中断。
- `packages/desktop/shiju/backend/manifest.json`：补丁集及应用顺序；后续 `codex-model-window.patch`、`claude-first-turn-recovery.patch` 也影响最终行为。
- `packages/desktop/shiju/cli-models/README.md`：隔离约束、支持范围、交付与未验收边界。

## 当前仓库已确认的修改入口

- `crates/vw-agent/src/session/llm/prompt.rs`：`resolve_model_for_options` 在 ACP 时返回 `acp_route_model`，需要分离真实模型与执行路由。
- `crates/vw-agent/src/session/llm/stream.rs`：ACP 路由和 Provider 认证解析；保留所选模型、禁止递归选择 ACP 模型作为上游。
- `crates/vw-agent/src/session/llm/acp/config.rs`：`lookup_acp_command`、`parse_acp_options`，复用配置解析。
- `crates/vw-agent/src/session/llm/acp/request.rs`：`do_stream_request_acp`、`with_copilot_auth_environment`；后一项不是通用模型凭据映射。
- `crates/vw-agent/src/session/llm/acp/session.rs`：客户端缓存和会话记录；不能把运行时秘密直接纳入可输出 cache key 或持久化 agent_config。
- `crates/vw-acp/src/client/process.rs`、`spawn_command_options.rs`：子进程参数/env 注入；权限配置不因模型来源改变。
- `crates/vw-acp/src/session_persistence/serialize.rs`：当前会写 agent_config 与 agent_command；新注入的短期令牌必须剥离，恢复时重新签发。
- `crates/vw-api-types/src/acp_session.rs`、`chat.rs`：会话来源、能力、请求的共享类型。
- `crates/vw-agent/src/gateway/api/handlers/session/acp_options.rs`：现有 ACP 控制，不启动 Agent 的读取约束继续保留。
- `crates/vw-desktop/src/app/components/input_panel/`、`message/chat/acp_options.rs`、`session/runtime.rs`：模型来源和确认后更新 UI。
- `crates/vw-agent/src/session/llm/aisdk/`：参考低层模型驱动、认证、流转换；不能把中继请求送回有工具执行和会话副作用的 prompt 路由。

## 不照搬的部分

不照搬固定平台 Provider、不照搬四项白名单作为全量支持、不照搬“代理负责所有协议转换”的错误假设、不把原生 CLI 参数直接追加给 ACP 包、不把状态目录隔离当作权限或网络沙箱、不在业务代码里设全局环境变量、不改变厂商账户权限。
