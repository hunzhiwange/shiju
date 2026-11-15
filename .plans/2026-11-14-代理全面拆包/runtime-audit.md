# Agent 运行时审计补充（2026-11-15）

本文件补充只读 coverage.md，不替代原清单；仅静态审计，代码拆分尚未完成，未运行 Cargo、测试、实机。

## 入口与历史证据

前置目录已在 d7f1bee47 删除；22 项实施提交可从 `git log -- .plans/2026-11-14-代理底层拆包` 追溯。不得因入口删除重建实施卡。续篇原始范围为23—364共342卡；此处为历史审计快照，不是实时进度。2026-11-15用户要求改为并行，当前批次见prompt.md，任务边界及公共接线协议见plan.md；旧的全串行约束已被替代。

前置产物及提交：
- b4377de51 chore(agent): 清理拆包后未使用的直接依赖
- 1d275ce4a refactor(plugins): 提取插件清单模型独立库
- a41fe9213 refactor(database): 抽取全局数据库核心独立库
- e0fa955fe refactor(config): 提取运行目录契约独立库
- b6f3ba616 refactor(office): 提取办公会话状态独立库
- 62ece1148 refactor(security): 提取安全审计独立库
- 3defe2e9f refactor(agent-bus): 提取事件总线独立库
- 211033ba7 refactor(agent-log): 提取运行日志独立库
- feb34a9b6 refactor(agent-paths): 提取全局路径独立库
- 94f02f207 refactor(agent-async): 提取异步协调工具独立库
- d662086a7 refactor(observer-otel): 提取链路观测后端独立库
- ff475f10c refactor(observer-prometheus): 提取指标观测后端独立库
- 403d81b12 refactor(observer-backends): 提取基础观测后端独立库
- d8f5bd1b3 refactor(observer-types): 提取观测契约独立库
- fed62e22f refactor(agent-context): 提取内存上下文工具独立库
- e50fb099c refactor(agent-text): 提取纯文本工具独立库
- 203968c59 refactor(canary-guard): 提取金丝雀防护独立库
- 245745af5 refactor(command-arity): 提取命令元数独立库
- 89115755d refactor(agent-identity): 提取身份纯模型独立库
- e6266bbc1 refactor(goal-engine): 抽取目标执行引擎独立库
- 1af32ed4c refactor(runtime-flags): 抽取运行时环境标志独立库

## 全包入口、配置与消费者

实际入口：Cargo.toml 的 lib/src/lib.rs；无 build.rs。src/package_boundaries 属静态边界测试，不是运行时服务。
直接路径消费者：crates/libs/cli-bootstrap/Cargo.toml、crates/libs/cli-agent-session/Cargo.toml、crates/libs/cli-task-commands/Cargo.toml、crates/libs/cli-security-commands/Cargo.toml、crates/libs/cli-runtime-status/Cargo.toml、crates/libs/cli-command-schema/Cargo.toml
上层 CLI 通过这些职责库间接消费；不据旧引用臆造 vw-desktop 的直接 Cargo 依赖。

features（完整转发，未验证）：`default`, `browser-native`, `rag-pdf`, `firecrawl`, `web-fetch-html2md`, `web-fetch-plaintext`, `probe`, `memory-postgres`, `memory-mariadb`, `observability-otel`, `sandbox-bubblewrap`, `sandbox-landlock`, `shell-ast`, `channel-lark`, `whatsapp-web`, `name`, `path`。
平台表：`target.'cfg(not(target_arch = "wasm32"))'.dependencies`, `target.'cfg(all(not(target_arch = "wasm32"), target_os = "windows"))'.dependencies`, `target.'cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))'.dependencies`, `target.'cfg(target_os = "macos")'.dependencies`, `target.'cfg(target_arch = "wasm32")'.dependencies`。依赖表及特性以当前清单为准；Windows bundled SQLite、WASM Tokio、macOS 网络配置不得扁平化。

## 补充归属与边界决定

- `global_database.rs`：既有 agent-database；保留薄导出
- `global_database_tests.rs`：任务 364 → 主包跨数据库集成验收（涉及三领域，不下沉数据库库）
- `flag/mod.rs`：既有 runtime-flags；保留薄导出
- `util/signal.rs`：任务 59 → 既有 agent-async
- `util/signal_tests.rs`：任务 59 → 既有 agent-async
- `util/log.rs`：既有 agent-log；保留薄导出
- `security/syscall_anomaly.rs`：既有 syscall-anomaly；保留薄导出
- `security/canary_guard/mod.rs`：既有 canary-guard；保留薄导出
- `plugins/types.rs`：既有 plugin-types；保留薄导出
- `bus/mod.rs`：既有 agent-bus；保留薄导出
- `observability/traits/mod.rs`：既有 observer-types；保留薄导出

任务59扩展现有 agent-async 的 signal 模块，不建立同义异步库；保留 watch 通知、先触发后等待及注册唤醒后的语义。数据库集成测试留主包，任务364收口时检查三个存储共享数据库路径及安全权限。

## 依赖方向、环与共享状态

下表是从所有源文件提取 crate:: 引用的顶层有向图，包含测试/注释候选，不等同 Rust 解析图；别名及 super 引用须在对应实施卡逐符号追踪。A → B 表示 A 使用 B。纯基础库导出已读取，新的契约库名称仍为计划依赖，不称为已存在接口。

| 模块 | 引用模块候选 |
| --- | --- |
| acp | agent, agent_apps, app, config, global, global_database, id, plugins, project, provider, security, session, skills, storage, tools |
| agent | app, approval, channels, config, hooks, memory, multimodal, observability, permission, plugins, project, provider, providers, security, session, skills, task_plan_run, test_support, tools, worktree |
| agent_apps | app_providers, gateway, global, global_database, plugins, project, session, tools |
| agent_scheduler | app, config, cron, gateway, global_database, project, session, task_lifecycle, task_worktree_pool, test_support, tools |
| agent_scheduler_policy_legacy_review_exact_tests | agent_scheduler |
| app | agent, approval, auth, bus, channels, command, config, cron, daemon, doctor, env, file, flag, gateway, global, health, heartbeat, hooks, id, installation, integrations, memory, multimodal, observability, patch, permission, project, provider, providers, pty, question, runtime, scheduler, security, session, shell, skill, skills, snapshot, storage, tools, tunnel, util |
| app_providers | agent_apps, gateway, global_database, model_relay |
| app_routing | Config, agent_apps, app_providers, daemon, gateway, global, global_database, model_relay, security, usage_bridge |
| approval | app |
| auth | app |
| channels | agent, app, approval, config, env, global, global_database, id, memory, project, providers, security, session, storage, test_support, tools |
| command | app |
| computer_control | security |
| config | agent, app, global, global_database, project, session, test_support |
| cron | app, channels, config, gateway, global, global_database, memory, providers, security, session |
| daemon | agent_scheduler, app |
| device_sync | agent_apps, gateway, global_database |
| doctor | app |
| file | app |
| format | app, test_support |
| gateway | Config, acp, agent, agent_apps, agent_scheduler, app, app_providers, app_routing, approval, bus, channels, computer_control, config, cron, daemon, device_sync, env, git_repository, global, global_database, id, knowledge, memory, model_catalog, model_relay, observability, permission, plugins, project, provider, providers, security, session, shiju, skills, storage, task_lifecycle, task_plan_run, task_worktree_pool, test_support, tools, usage_bridge, util, workflow, worktree |
| global_database_tests | agent_scheduler, project, session |
| heartbeat | app |
| hooks | app, plugins, tools |
| identity | app |
| installation | app, test_support |
| integrations | app |
| knowledge | app, global_database, memory, workflow |
| mcp | app, tools |
| memory | app |
| migration | MigrateCommands, app, memory |
| model_relay | app_routing, config, env, project, provider, session, test_support, usage_bridge |
| multimodal | app, global |
| observability | app |
| package_boundaries | config, git_repository, memory, security |
| permission | app |
| plugins | agent, agent_apps, app, approval, bus, cli_commands, config, gateway, hooks, observability, project, providers, question, security, session, skills, tools |
| project | agent_scheduler, app, git_repository, global_database, test_support |
| provider | app, auth, config, env, gateway, global, model_relay, project, providers, session, test_support |
| providers | app, provider, session |
| pty | app |
| question | app, session |
| runtime | app, security, tools |
| scheduler | app |
| security | app, global_database, tools |
| service | app |
| session | acp, agent, app, approval, auth, channels, config, env, gateway, global, global_database, hooks, id, model_relay, multimodal, observability, permission, plugins, project, provider, providers, runtime, security, skills, snapshot, storage, test_support, tools, usage_bridge, util, workflow, worktree |
| skill | app, test_support |
| skillforge | app |
| skills | acp, app, config, plugins, test_support |
| snapshot | app, test_support |
| storage | app |
| task_lifecycle | app, global_database |
| task_plan_run | agent, app, channels, config, global, global_database, id, installation, project, session, test_support |
| task_worktree_pool | app |
| tests | Config |
| tools | Config, acp, agent, agent_apps, agent_scheduler, app, approval, computer_control, config, gateway, hooks, multimodal, permission, plugins, project, providers, runtime, security, session, skills, test_support, worktree |
| tunnel | app, test_support |
| usage_bridge | global, global_database, project, session |
| util | filesystem |
| workflow | agent, app, config, gateway, global, global_database, hooks, providers, question, session, skills, test_support, tools |
| worktree | agent, app, bus, config, gateway, git_repository, global, global_database, observability, project, providers, security, session, shell, storage, tools |

显式解环顺序：tool-types → model-types/runtime-types/channel-types → session-port/tool-context → app-settings-store → 领域实现 → session-engine/agent-loop → gateway → 主包组装。此箭头为先后顺序，实际依赖由上层指向下层。
已见跨域环：providers/session_llm ↔ session/llm；tools/cron/workflow ↔ session；config/agent_apps ↔ gateway；plugins/compat ↔ tools；app_routing ↔ model_relay。分别复用23—29契约卡及已有续篇关键解环边界，不新建重复任务。
全局数据库连接、agent-bus、agent-log、agent-paths 和审批/权限等待状态仅保留一个所有者；gateway/state 只持服务句柄。原包暂留 bootstrap、工厂注入、跨域集成测试和薄导出，不能保留生产引擎冒充组合。

## 候选库依赖与实际引用证据

按直接外部依赖数升序列出作为低依赖起点候选；同一循环必须先解环，不能把数量排序当作拓扑序。每行源路径见后续全文件表，契约/行为边界沿原编号卡；目标为 crates/libs/<包名>，agent-composition 保留主包。外依赖为源码词法引用与清单交集，测试依赖包含其中，不声称最小编译依赖。

| 既有任务/职责及目标 | 外依赖候选（数量） |
| --- | --- |
| 任务 342 → gateway-tests | 0:  |
| 任务 55 → agent-security | 0:  |
| 任务 96 → device-sync | 0:  |
| 任务 107 → network-policy | 1: anyhow |
| 任务 125 → task-scheduler-core | 1: anyhow |
| 任务 131 → task-scheduler-core | 1: anyhow |
| 任务 228 → agent-loop | 1: regex |
| 任务 231 → agent-loop | 1: tokio |
| 任务 324 → gateway-mod | 1: cron |
| 任务 38 → agent-env | 1: agent-context |
| 任务 42 → agent-composition（宿主重导出接线，模型来自agent-identity，加载来自agent-identity-loader） | 1: agent-identity |
| 任务 59 → 既有 agent-async | 1: tokio |
| 既有 agent-bus；保留薄导出 | 1: agent-bus |
| 既有 agent-database；保留薄导出 | 1: agent-database |
| 既有 agent-log；保留薄导出 | 1: agent-log |
| 既有 canary-guard；保留薄导出 | 1: canary-guard |
| 既有 observer-types；保留薄导出 | 1: observer-types |
| 既有 plugin-types；保留薄导出 | 1: plugin-types |
| 既有 runtime-flags；保留薄导出 | 1: runtime-flags |
| 既有 syscall-anomaly；保留薄导出 | 1: syscall-anomaly |
| 任务 105 → model-protocol | 2: reqwest, serde_json |
| 任务 227 → agent-loop | 2: cron, serde_json |
| 任务 25 → runtime-types | 2: anyhow, tokio |
| 任务 250 → channel-runtime | 2: time, tokio |
| 任务 51 → agent-security | 2: config-security, tracing |
| 任务 72 → agent-knowledge | 2: anyhow, postgres |
| 任务 114 → session-store | 3: project-types, serde_json, tempfile |
| 任务 212 → session-engine | 3: serde_json, tempfile, time |
| 任务 223 → agent-loop | 3: json-extract, regex, serde_json |
| 任务 224 → agent-loop | 3: regex, serde_json, tracing |
| 任务 260 → channel-telegram | 3: serde_json, time, tokio |
| 任务 32 → agent-cli-types | 3: clap, serde, serde_json |
| 任务 50 → agent-security | 3: config-security, parking_lot, time |
| 任务 75 → agent-mcp | 3: rmcp, serde_json, tokio |
| 任务 112 → session-store | 4: anyhow, postgres, rusqlite, vw-api-transport |
| 任务 121 → skill-core | 4: clap, serde, serde_json, toml |
| 任务 138 → agent-cron | 4: anyhow, chrono, cron, tempfile |
| 任务 159 → llm-runtime | 4: aisdk, reqwest, serde_json, tokio |
| 任务 26 → channel-types | 4: anyhow, async-trait, serde_json, tokio |
| 任务 364 → 主包跨数据库集成验收（涉及三领域，不下沉数据库库） | 4: anyhow, config-runtime-paths, rusqlite, tempfile |
| 任务 40 → agent-format | 4: serde, serde_json, tempfile, tokio |
| 任务 41 → agent-health | 4: build-identity, serde, serde_json, time |
| 任务 42 → agent-identity-loader | 4: agent-identity, anyhow, serde_json, tempfile |
| 任务 47 → agent-pty | 4: portable-pty, serde, serde_json, tokio |
| 任务 53 → agent-security | 4: domain-matcher, tempfile, time, tokio |
| 任务 56 → agent-security | 4: canary-guard, domain-matcher, process-sandbox, syscall-anomaly |
| 任务 63 → permission-runtime | 4: permission-engine, permission-types, serde, serde_json |
| 任务 90 → agent-skills | 4: anyhow, config-runtime-paths, config-skills, tracing |
| 任务 140 → agent-cron | 5: anyhow, chrono, cron, rusqlite, tempfile |
| 任务 143 → agent-scheduler-core | 5: serde_json, time, tokio, uuid, wasm-bindgen-futures |
| 任务 175 → tool-browser | 5: anyhow, async-trait, serde_json, tokio, vw-api-tools |
| 任务 184 → tool-memory | 5: anyhow, async-trait, serde_json, tempfile, tokio |
| 任务 195 → tool-shell | 5: anyhow, async-trait, serde_json, time, tokio |
| 任务 234 → channel-cli | 5: anyhow, async-trait, time, tokio, uuid |
| 任务 246 → channel-nostr | 5: anyhow, async-trait, nostr-sdk, tokio, tracing |
| 任务 261 → channel-telegram | 5: parking_lot, reqwest, serde_json, time, tokio |
| 任务 30 → agent-approval | 5: chrono, parking_lot, serde, serde_json, uuid |
| 任务 352 → agent-heartbeat | 5: anyhow, tempfile, time, tokio, tracing |
| 任务 353 → agent-migration | 5: anyhow, chrono, rusqlite, tempfile, tokio |
| 任务 43 → agent-installation | 5: config-runtime-paths, reqwest, serde, serde_json, tempfile |
| 任务 68 → agent-commands | 5: regex, serde, serde_json, tempfile, tokio |
| 任务 120 → skill-core | 6: glob, serde_json, serde_yaml, tempfile, time, tokio |
| 任务 132 → task-worktree-pool | 6: anyhow, tempfile, time, tokio, vw-api-task, vw-api-transport |
| 任务 208 → session-engine | 6: regex, serde_json, tempfile, time, tokio, tokio-util |
| 任务 219 → agent-loop | 6: anyhow, async-trait, serde_json, time, tokio, tokio-util |
| 任务 230 → agent-loop | 6: anyhow, async-trait, config-agent, serde_json, tempfile, tokio |
| 任务 253 → channel-runtime | 6: anyhow, clap, serde, time, tokio, tokio-util |
| 任务 259 → channel-telegram | 6: anyhow, async-trait, serde_json, tempfile, time, tokio |
| 任务 302 → gateway-auth | 6: auth-file-store, axum, config-runtime-paths, serde_json, tokio, tower |
| 任务 303 → gateway-backup-inventory | 6: axum, serde, tokio, vw-agent-apps, vw-api-transport, vw-api-types |
| 任务 331 → gateway-pty | 6: axum, serde, serde_json, tempfile, time, tokio |
| 任务 44 → agent-integrations | 6: anyhow, clap, config-runtime-paths, console, serde, serde_json |
| 任务 79 → agent-multimodal | 6: anyhow, base64, reqwest, serde_json, thiserror, tokio |
| 任务 84 → agent-question | 6: question-types, serde_json, time, tokio, uuid, vw-api-transport |
| 任务 94 → channel-support | 6: anyhow, hex, serde, sha2, tokio, tracing |
| 任务 98 → model-catalog | 6: hex, serde_json, sha2, vw-agent-apps, vw-api-model-catalog, vw-api-transport |
| 任务 113 → session-store | 7: anyhow, postgres, serde, serde_json, session-info, session-ui-types, vw-api-transport |
| 任务 119 → skill-core | 7: anyhow, config-skills, dialoguer, serde_json, tempfile, toml, tracing |
| 任务 126 → task-scheduler-core | 7: anyhow, project-types, rusqlite, serde_json, tempfile, vw-api-agent-scheduler, vw-api-transport |
| 任务 127 → task-scheduler-core | 7: anyhow, postgres, project-types, serde_json, tempfile, vw-api-agent-scheduler, vw-api-transport |
| 任务 128 → task-scheduler-core | 7: anyhow, postgres, rusqlite, serde_json, vw-api-agent-scheduler, vw-api-task, vw-api-transport |
| 任务 137 → agent-cron | 7: anyhow, chrono, rusqlite, serde_json, tempfile, tracing, uuid |
| 任务 165 → plugin-mcp | 7: acp-config, anyhow, base64, rmcp, serde, serde_json, url |
| 任务 174 → tool-browser | 7: anyhow, async-trait, reqwest, serde_json, tempfile, tokio, vw-api-tools |
| 任务 190 → tool-shell | 7: anyhow, async-trait, serde, serde_json, tempfile, tokio, uuid |
| 任务 191 → tool-shell | 7: parking_lot, serde, serde_json, shell-words, tracing, tree-sitter, tree-sitter-bash |
| 任务 206 → session-engine | 7: anyhow, message-types, serde, serde_json, tokio, uuid, vw-api-transport |
| 任务 23 → tool-types | 7: anyhow, async-trait, rmcp, serde, serde_json, tokio, vw-api-tools |
| 任务 232 → agent-loop | 7: config-agent, config-agent-profile, message-types, serde, serde_json, tokio, tracing |
| 任务 255 → channel-slack | 7: anyhow, async-trait, reqwest, serde_json, time, tokio, tracing |
| 任务 269 → gateway-types | 7: axum, parking_lot, serde, serde_json, time, tokio, vw-api-transport |
| 任务 27 → session-port | 7: anyhow, serde_json, session-ui-types, time, tokio-util, vw-api-chat-identity, vw-api-transport |
| 任务 322 → gateway-memory | 7: anyhow, async-trait, axum, parking_lot, serde_json, time, tokio |
| 任务 330 → gateway-provider | 7: axum, serde, serde_json, tempfile, tokio, tower, vw-api-transport |
| 任务 332 → gateway-question | 7: axum, serde, serde_json, time, tokio, tower, uuid |
| 任务 340 → gateway-status | 7: anyhow, async-trait, axum, parking_lot, serde_json, time, tokio |
| 任务 355 → agent-tunnel | 7: anyhow, async-trait, serde_json, tempfile, time, tokio, tracing |
| 任务 54 → agent-security | 7: anyhow, async-trait, axum, config-security, serde_json, tempfile, tokio |
| 任务 57 → agent-snapshot | 7: git2, serde, serde_json, snapshot-types, tempfile, time, tokio |
| 任务 70 → agent-knowledge | 7: base64, mime_guess, once_cell, pdf-extract, regex, serde_json, vw-api-knowledge |
| 任务 89 → agent-skills | 7: config-runtime-paths, config-skills, glob, serde, serde_json, tempfile, toml |
| 任务 117 → shiju-client | 8: chrono, reqwest, serde, serde_json, serde_yaml, time, tokio, vw-gateway-client |
| 任务 169 → task-scheduler | 8: anyhow, serde_json, tempfile, vw-api-agent-scheduler, vw-api-chat-identity, vw-api-task, vw-api-transport, vw-shared |
| 任务 172 → tool-browser | 8: anyhow, reqwest, serde, serde_json, tempfile, time, tokio, tracing |
| 任务 209 → session-engine | 8: aisdk, anyhow, config-agent, serde_json, tokio-util, tracing, uuid, vw-api-acp-model |
| 任务 221 → agent-loop | 8: anyhow, async-trait, serde_json, tempfile, tokio, tokio-util, tracing, uuid |
| 任务 226 → agent-loop | 8: anyhow, async-trait, base64, chrono-tz, serde_json, tempfile, time, tokio |
| 任务 239 → channel-imessage | 8: anyhow, async-trait, directories, rusqlite, tempfile, time, tokio, tracing |
| 任务 24 → model-types | 8: anyhow, async-trait, futures-util, reqwest, serde, serde_json, thiserror, tokio |
| 任务 244 → channel-mattermost | 8: anyhow, async-trait, parking_lot, reqwest, serde_json, time, tokio, tracing |
| 任务 258 → channel-telegram | 8: anyhow, axum, reqwest, serde_json, tempfile, time, tokio, tracing |
| 任务 266 → channel-whatsapp | 8: anyhow, async-trait, reqwest, serde_json, time, tokio, tracing, uuid |
| 任务 270 → tool-assembly | 8: aisdk, anyhow, async-trait, serde, serde_json, time, tokio, vw-api-tools |
| 任务 31 → agent-auth | 8: anyhow, auth-file-store, auth-types, reqwest, serde, serde_json, tempfile, tokio |
| 任务 315 → gateway-hermes-dashboard | 8: axum, libc, tokio, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 323 → gateway-misc | 8: axum, serde, serde_json, tempfile, tokio, tower, uuid, vw-api-transport |
| 任务 327 → gateway-permission | 8: axum, serde, serde_json, tempfile, tokio, tower, tracing, vw-api-transport |
| 任务 338 → gateway-shiju | 8: axum, reqwest, serde, serde_json, time, tokio, tower, vw-gateway-client |
| 任务 343 → gateway-tools | 8: anyhow, async-trait, axum, parking_lot, serde_json, time, tokio, vw-api-tools |
| 任务 351 → agent-doctor | 8: anyhow, chrono, reqwest, serde, serde_json, tempfile, time, tokio |
| 任务 36 → agent-config | 8: anyhow, config-runtime-paths, serde, tempfile, tokio, toml, tracing, uuid |
| 任务 45 → agent-observability | 8: anyhow, chrono, observer-backends, serde, serde_json, tempfile, tracing, uuid |
| 任务 115 → session-store | 9: anyhow, postgres, rusqlite, serde, serde_json, session-info, session-ui-types, thiserror, vw-api-transport |
| 任务 129 → task-scheduler-core | 9: anyhow, config-runtime-paths, postgres, project-types, rusqlite, serde_json, tempfile, vw-api-agent-scheduler, vw-api-transport |
| 任务 152 → llm-runtime | 9: acp-access-policy, acp-storage, acp-types, parking_lot, reqwest, serde_json, time, tokio, vw-api-acp-model |
| 任务 171 → tool-agents | 9: anyhow, async-trait, futures-util, serde_json, tempfile, time, tokio, vw-api-chat, vw-api-transport |
| 任务 177 → tool-cron | 9: anyhow, async-trait, chrono, cron, serde, serde_json, tempfile, time, tokio |
| 任务 182 → tool-git | 9: anyhow, async-trait, chrono, git2, serde, serde_json, tempfile, tokio, vw-api-tools |
| 任务 200 → workflow-engine | 9: anyhow, async-trait, axum, futures-util, reqwest, serde_json, time, tokio, vw-api-workflow |
| 任务 216 → session-engine | 9: anyhow, serde, serde_json, session-ui-types, tempfile, tokio, tracing, vw-api-session, vw-api-todo |
| 任务 233 → channel-clawdtalk | 9: anyhow, async-trait, config-channel-services, reqwest, serde, serde_json, time, tokio, tracing |
| 任务 319 → gateway-integrations | 9: anyhow, async-trait, axum, parking_lot, serde_json, tempfile, time, tokio, vw-api-transport |
| 任务 333 → gateway-resumable-stream | 9: axum, futures-util, hex, serde_json, sha2, time, tokio, uuid, vw-api-transport |
| 任务 354 → agent-service | 9: anyhow, clap, config-runtime-paths, directories, serde, serde_json, tempfile, time, which |
| 任务 48 → agent-runtime | 9: anyhow, config-runtime-paths, directories, serde_json, tempfile, time, tokio, uuid, which |
| 任务 58 → agent-storage | 9: config-runtime-paths, git2, serde, serde_json, tempfile, time, tokio, uuid, walkdir |
| 任务 81 → agent-plugins | 9: acp-config, agent-client-protocol, anyhow, config-plugins, libc, serde, serde_json, tempfile, tracing |
| 任务 85 → agent-skills | 9: anyhow, config-runtime-paths, config-skills, console, regex, serde, tempfile, toml, walkdir |
| 任务 104 → model-protocol | 10: axum, futures-util, model-protocols, reqwest, serde_json, time, tokio, tokio-util, uuid, vw-agent-apps |
| 任务 110 → session-instructions | 10: config-runtime-paths, glob, reqwest, serde, serde_json, serde_yaml, tempfile, time, tokio, tracing |
| 任务 111 → session-store | 10: anyhow, async-trait, project-types, runtime-time, serde, serde_json, session-ui-types, tempfile, vw-api-tools, vw-api-transport |
| 任务 118 → skill-core | 10: anyhow, config-skills, regex, reqwest, serde, serde_json, tempfile, time, tokio, toml |
| 任务 122 → skill-forge | 10: anyhow, async-trait, chrono, reqwest, serde, serde_json, tempfile, time, tokio, tracing |
| 任务 123 → task-lifecycle | 10: anyhow, config-runtime-paths, postgres, rusqlite, serde_json, tempfile, time, tokio, vw-api-task-lifecycle, vw-api-transport |
| 任务 124 → task-scheduler-core | 10: anyhow, async-trait, chrono, serde, serde_json, tempfile, tokio, vw-api-agent-scheduler, vw-api-task, vw-api-transport |
| 任务 188 → tool-services | 10: anyhow, async-trait, base64, chrono, cron, image, serde_json, tempfile, time, tokio |
| 任务 193 → tool-shell | 10: anyhow, libc, parking_lot, shell-words, tempfile, time, tokio, tokio-util, tracing, uuid |
| 任务 199 → workflow-engine | 10: anyhow, async-trait, futures-util, reqwest, serde_json, time, tokio, tracing, uuid, vw-api-workflow |
| 任务 207 → session-engine | 10: config-tools, model-state, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-transport, web-time |
| 任务 210 → session-engine | 10: anyhow, async-trait, image, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-tools |
| 任务 215 → session-engine | 10: anyhow, serde, serde_json, session-text, tempfile, time, tokio, tracing, uuid, vw-api-transport |
| 任务 222 → agent-loop | 10: anyhow, async-trait, futures-util, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-tools |
| 任务 225 → agent-loop | 10: anyhow, async-trait, chrono-tz, config-agent, serde_json, tempfile, time, tokio, uuid, vw-api-tools |
| 任务 229 → agent-loop | 10: anyhow, async-trait, chrono-tz, config-agent, config-agent-profile, hostname, serde_json, tempfile, tokio, uuid |
| 任务 236 → channel-discord | 10: anyhow, futures-util, rand, reqwest, serde_json, tempfile, time, tokio, tokio-tungstenite, tracing |
| 任务 241 → channel-lark | 10: anyhow, pulldown-cmark, rand, reqwest, serde_json, time, tokio, toml, tracing, uuid |
| 任务 256 → channel-telegram | 10: anyhow, async-trait, axum, base64, image, serde_json, tempfile, time, tokio, tracing |
| 任务 263 → channel-wati | 10: anyhow, async-trait, axum, chrono, reqwest, serde_json, time, tokio, tracing, uuid |
| 任务 279 → agent-gateway | 10: axum, hex, hmac, parking_lot, reqwest, serde_json, sha2, tempfile, time, tokio |
| 任务 282 → gateway-agent-app | 10: anyhow, axum, serde, serde_json, tokio, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 305 → gateway-computer-control | 10: axum, config-computer-control, hex, libc, serde_json, sha2, tokio, tower, vw-api-transport, vw-api-types |
| 任务 309 → gateway-device-sync | 10: axum, hex, serde, serde_json, sha2, tokio, tokio-util, vw-agent-apps, vw-api-transport, vw-api-types |
| 任务 328 → gateway-plugins | 10: anyhow, axum, config-plugins, serde, serde_json, tempfile, time, tokio, uuid, vw-gateway-client |
| 任务 337 → gateway-session | 10: anyhow, async-trait, axum, parking_lot, tempfile, time, tokio, tower, uuid, vw-api-transport |
| 任务 69 → agent-hooks | 10: anyhow, async-trait, chrono, futures-util, serde, serde_json, tempfile, time, tokio, tracing |
| 任务 71 → agent-knowledge | 10: anyhow, hex, mime_guess, postgres, serde_json, sha2, tempfile, uuid, vw-api-knowledge, vw-api-transport |
| 任务 86 → agent-skills | 10: anyhow, config-skills, hex, serde, serde_json, serde_yaml, sha2, tempfile, toml, tracing |
| 任务 130 → task-scheduler-core | 11: anyhow, config-runtime-paths, postgres, project-types, rusqlite, serde, serde_json, tempfile, vw-api-agent-scheduler, vw-api-task, vw-api-transport |
| 任务 144 → app-provider-services | 11: axum, serde_json, tempfile, time, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types |
| 任务 161 → llm-runtime | 11: acp-agent-registry, acp-types, aisdk, reqwest, serde, serde_json, tempfile, time, tokio, tracing, vw-api-acp-model |
| 任务 162 → plugin-hooks | 11: agent-client-protocol, anyhow, hex, regex, serde, serde_json, sha2, tempfile, time, tokio, tracing |
| 任务 178 → tool-files | 11: anyhow, async-trait, regex, serde, serde_json, similar, tempfile, time, tokio, vw-api-tools, which |
| 任务 180 → tool-files | 11: anyhow, async-trait, glob, image, regex, serde, serde_json, tempfile, time, tokio, vw-api-tools |
| 任务 181 → tool-files | 11: anyhow, async-trait, base64, image, pdf-extract, serde_json, tempfile, time, tokio, tracing, which |
| 任务 189 → tool-services | 11: anyhow, async-trait, config-agent, permission-types, serde, serde_json, tempfile, time, todo-types, tokio, vw-api-tools |
| 任务 211 → session-engine | 11: aisdk, anyhow, async-trait, config-agent, serde_json, tempfile, time, tokio, tokio-util, tracing, vw-api-tools |
| 任务 254 → channel-signal | 11: anyhow, async-trait, axum, futures-util, reqwest, serde, serde_json, time, tokio, tracing, uuid |
| 任务 262 → channel-tests | 11: anyhow, async-trait, axum, chrono, chrono-tz, serde_json, tempfile, time, tokio, uuid, vw-api-transport |
| 任务 264 → channel-wechat | 11: anyhow, async-trait, base64, config-channel-services, pulldown-cmark, reqwest, serde_json, time, tokio, tracing, uuid |
| 任务 280 → agent-gateway | 11: anyhow, async-trait, axum, hex, hmac, parking_lot, rand, serde_json, sha2, time, tokio |
| 任务 310 → gateway-dynamic-tools | 11: axum, futures-util, reqwest, serde_json, tempfile, time, tokio, tower, uuid, vw-api-tools, vw-api-transport |
| 任务 313 → gateway-git | 11: axum, base64, git2, similar, tempfile, time, tokio, tracing, uuid, vw-api-git, vw-api-transport |
| 任务 316 → gateway-hermes-memory | 11: axum, hex, serde, serde_json, sha2, tempfile, tokio, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 317 → gateway-hooks | 11: anyhow, async-trait, axum, config-plugins, parking_lot, serde_json, tempfile, time, tokio, tower, vw-gateway-client |
| 任务 318 → gateway-instance | 11: axum, config-runtime-paths, directories, git2, serde, serde_json, tempfile, time, tokio, tokio-stream, tower |
| 任务 326 → gateway-openclaw-settings | 11: axum, hex, serde, serde_json, sha2, tempfile, tokio, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 347 → gateway-workspace | 11: axum, hex, serde, serde_json, sha2, tempfile, tokio, uuid, vw-agent-apps, vw-api-transport, vw-api-types |
| 任务 348 → gateway-worktree | 11: axum, config-worktree, git2, serde_json, tempfile, time, tokio, tower, uuid, vw-api-transport, vw-api-worktree |
| 任务 37 → agent-config | 11: anyhow, config-gateway, config-runtime-paths, postgres, rusqlite, serde_json, serde_yaml, tempfile, time, tokio, vw-api-transport |
| 任务 80 → agent-plugins | 11: anyhow, async-trait, config-plugins, hex, serde, serde_json, sha2, tempfile, time, tokio, tracing |
| 任务 135 → worktree-core | 12: config-runtime-paths, directories, git2, regex, serde, serde_json, tempfile, time, tokio, tracing, uuid, vw-api-transport |
| 任务 139 → agent-cron | 12: axum, chrono, config-agent, config-channel-types, config-runtime-paths, cron, serde_json, tempfile, time, tokio, uuid, vw-api-transport |
| 任务 179 → tool-files | 12: anyhow, async-trait, base64, glob, image, pdf-extract, serde, serde_json, tempfile, time, tokio, vw-api-tools |
| 任务 186 → tool-services | 12: anyhow, async-trait, config-agent, config-agent-profile, futures-util, serde, serde_json, similar, tempfile, tokio, vw-api-tools, vw-api-transport |
| 任务 197 → tool-web | 12: anyhow, async-trait, config-tools, regex, reqwest, serde, serde_json, time, tokio, tracing, urlencoding, vw-api-tools |
| 任务 213 → session-engine | 12: anyhow, futures-util, serde, serde_json, tempfile, thiserror, time, tokio, tokio-util, uuid, vw-api-tools, vw-api-transport |
| 任务 237 → channel-discord | 12: anyhow, async-trait, futures-util, parking_lot, reqwest, serde_json, tempfile, time, tokio, tokio-tungstenite, tracing, uuid |
| 任务 240 → channel-irc | 12: anyhow, async-trait, chrono, ring, rustls, serde_json, time, tokio, tokio-rustls, toml, tracing, webpki-roots |
| 任务 243 → channel-linq | 12: anyhow, async-trait, chrono, hex, hmac, reqwest, serde_json, sha2, time, tokio, tracing, uuid |
| 任务 245 → channel-nextcloud-talk | 12: anyhow, async-trait, hex, hmac, reqwest, serde_json, sha2, time, tokio, tracing, urlencoding, uuid |
| 任务 252 → channel-runtime | 12: anyhow, async-trait, config-runtime-paths, model-reasoning, model-types, rusqlite, serde_json, tempfile, tokio, tokio-util, tracing, vw-api-transport |
| 任务 267 → channel-whatsapp-storage | 12: anyhow, async-trait, chrono, parking_lot, prost, rusqlite, serde_json, tempfile, tokio, wa-rs-binary, wa-rs-core, wa-rs-proto |
| 任务 299 → gateway-app-providers | 12: axum, gateway-app-provider, gateway-endpoint, serde_json, tempfile, tokio, tower, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 350 → agent-daemon | 12: anyhow, chrono, config-channel-services, cron, reqwest, serde_json, tempfile, time, tokio, tokio-util, tracing, vw-api-transport |
| 任务 52 → agent-security | 12: anyhow, async-trait, axum, content-guard, hex, secret-store, serde, serde_json, sha2, tempfile, tokio, tracing |
| 任务 74 → agent-knowledge | 12: anyhow, async-trait, axum, postgres, rusqlite, serde_json, tempfile, time, tokio, uuid, vw-api-knowledge, vw-api-transport |
| 任务 92 → channel-support | 12: anyhow, async-trait, axum, config-agent, pulldown-cmark, reqwest, serde_json, tempfile, time, tokio, toml, tracing |
| 任务 146 → app-provider-services | 13: anyhow, provider-model-cache, serde, serde_json, tempfile, tokio, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-model-catalog, vw-api-transport, vw-api-types |
| 任务 160 → llm-runtime | 13: aisdk, anyhow, async-trait, once_cell, reqwest, serde, serde_json, tempfile, time, tokio, vw-api-acp-model, vw-api-tools, web-time |
| 任务 187 → tool-services | 13: acp-config, agent-client-protocol, anyhow, async-trait, config-agent, config-tools, rmcp, serde, serde_json, tempfile, time, tokio, vw-api-tools |
| 任务 192 → tool-shell | 13: anyhow, config-security, directories, glob, libc, regex, serde_json, shell-words, shellexpand, tempfile, time, tokio, vw-api-transport |
| 任务 194 → tool-shell | 13: anyhow, config-security, libc, parking_lot, regex, serde_json, tempfile, thiserror, time, tokio, uuid, vw-api-transport, windows-sys |
| 任务 196 → tool-web | 13: anyhow, async-trait, config-tools, html2md, nanohtml2text, pulldown-cmark, reqwest, serde, serde_json, time, tokio, tracing, vw-api-tools |
| 任务 218 → agent-loop | 13: acp-agent-registry, anyhow, async-trait, chrono-tz, config-agent, config-agent-profile, parking_lot, serde_json, tempfile, time, tokio, tracing, vw-api-transport |
| 任务 235 → channel-dingtalk | 13: anyhow, async-trait, futures-util, pulldown-cmark, reqwest, serde, serde_json, time, tokio, tokio-tungstenite, toml, tracing, uuid |
| 任务 251 → channel-runtime | 13: anyhow, async-trait, axum, hex, serde_json, sha2, tempfile, tokio, tokio-util, toml, tracing, uuid, vw-api-transport |
| 任务 257 → channel-telegram | 13: anyhow, axum, config-runtime-paths, parking_lot, rand, regex, reqwest, serde_json, tempfile, time, tokio, tracing, vw-api-transport |
| 任务 274 → agent-gateway | 13: axum, chrono, hex, reqwest, serde_json, sha2, toml, toml_edit, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 281 → agent-gateway | 13: anyhow, async-trait, axum, hex, parking_lot, serde_json, sha2, time, tokio, tower, tracing, uuid, vw-api-tools |
| 任务 289 → gateway-agent-scheduler | 13: anyhow, async-trait, axum, chrono, futures-util, serde, serde_json, tokio, tracing, vw-api-agent-scheduler, vw-api-chat-identity, vw-api-task, vw-api-transport |
| 任务 33 → agent-config | 13: anyhow, config-aggregate, config-gateway, config-runtime-paths, rusqlite, serde_json, serde_yaml, tempfile, time, tokio, toml, tracing, vw-api-transport |
| 任务 34 → agent-config | 13: anyhow, config-aggregate, config-automation, config-channel-services, config-channel-types, config-channels, config-skills, serde_json, tempfile, tokio, tracing, uuid, vw-api-transport |
| 任务 341 → gateway-task | 13: anyhow, axum, config-runtime-paths, futures-util, serde_json, tempfile, tokio, tower, tracing, uuid, vw-api-task, vw-api-task-lifecycle, vw-api-transport |
| 任务 39 → agent-files | 13: base64, chardetng, encoding_rs, git2, glob, regex, serde, serde_json, strsim, tempfile, time, tokio, walkdir |
| 任务 78 → agent-multimodal | 13: anyhow, async-trait, base64, hex, image, parking_lot, serde, serde_json, sha2, tempfile, time, tokio, uuid |
| 任务 93 → channel-support | 13: anyhow, async-trait, chrono-tz, config-channels, hex, hostname, serde, serde_json, sha2, tempfile, tokio, tracing, vw-api-transport |
| 任务 102 → model-protocol | 14: axum, base64, chrono, futures-util, hex, model-protocols, model-reasoning, reqwest, serde_json, sha2, tokio, tokio-util, uuid, vw-agent-apps |
| 任务 108 → plugin-install | 14: agent-client-protocol, anyhow, git2, hex, libc, serde, serde_json, serde_yaml, sha2, tempfile, tracing, url, walkdir, which |
| 任务 116 → session-store | 14: anyhow, config-runtime-paths, project-types, rusqlite, serde, serde_json, session-info, session-ui-types, tempfile, tokio, tracing, vw-api-acp-model, vw-api-transport, vw-shared |
| 任务 170 → tool-agents | 14: anyhow, async-trait, axum, config-agent, futures-util, serde_json, session-text, tempfile, time, tokio, tokio-util, tracing, vw-api-chat, vw-api-transport |
| 任务 214 → session-engine | 14: anyhow, hex, regex, serde, serde_json, session-info, session-text, sha2, tempfile, time, tokio, uuid, vw-api-transport, web-time |
| 任务 220 → agent-loop | 14: aisdk, anyhow, async-trait, config-agent, config-agent-profile, cron, regex, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-tools |
| 任务 248 → channel-runtime | 14: anyhow, async-trait, config-agent, config-agent-profile, config-channel-services, config-channels, serde_json, tempfile, time, tokio, tracing, uuid, vw-api-acp-model, vw-api-transport |
| 任务 268 → channel-whatsapp-web | 14: anyhow, async-trait, chrono, parking_lot, qrcode, tokio, tracing, uuid, wa-rs, wa-rs-binary, wa-rs-core, wa-rs-proto, wa-rs-tokio-transport, wa-rs-ureq-http |
| 任务 271 → tool-assembly | 14: acp-config, agent-client-protocol, anyhow, async-trait, config-agent, config-agent-profile, glob, serde, serde_json, tempfile, time, tokio, tracing, vw-api-transport |
| 任务 275 → agent-gateway | 14: anyhow, async-trait, axum, config-channel-services, hex, parking_lot, serde, serde_json, sha2, tempfile, time, tokio, toml, vw-api-transport |
| 任务 28 → tool-context | 14: anyhow, async-trait, chrono, config-agent, config-agent-profile, permission-types, serde, serde_json, tempfile, time, tokio, tokio-util, vw-api-tools, vw-api-transport |
| 任务 29 → app-settings-store | 14: anyhow, postgres, reqwest, rusqlite, serde, serde_json, thiserror, time, tokio, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 290 → gateway-agent-scheduler | 14: anyhow, async-trait, axum, serde, serde_json, tempfile, time, tokio, tower, uuid, vw-api-agent-scheduler, vw-api-chat-identity, vw-api-task, vw-api-transport |
| 任务 335 → gateway-session | 14: anyhow, axum, config-agent, config-agent-profile, runtime-time, serde_json, tokio, tokio-util, tracing, vw-api-acp-model, vw-api-chat, vw-api-chat-identity, vw-api-transport, web-time |
| 任务 349 → model-relay-http | 14: anyhow, async-trait, axum, futures-util, hex, parking_lot, reqwest, serde_json, sha2, tempfile, time, tokio, tokio-util, vw-api-transport |
| 任务 59 → agent-util | 14: agent-async, agent-context, agent-text, anyhow, chrono, chrono-tz, display-format, glob, serde, serde_json, tempfile, time, tokio, web-time |
| 任务 61 → computer-control | 14: async-trait, base64, config-computer-control, image, libc, parking_lot, serde, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-types |
| 任务 91 → agent-usage | 14: anyhow, axum, chrono, futures-util, rusqlite, serde_json, time, tokio, tracing, uuid, vw-agent-apps, vw-api-transport, vw-api-types, vw-shared |
| 任务 95 → device-sync | 14: anyhow, chrono, hex, serde, serde_json, sha2, time, tokio, tokio-util, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 141 → agent-cron | 15: anyhow, async-trait, axum, chrono, clap, cron, parking_lot, serde, serde_json, tempfile, time, tokio, tower, uuid, vw-api-transport |
| 任务 142 → agent-providers | 15: aisdk, anyhow, async-trait, axum, config-provider, futures-util, gateway-chat-stream, gateway-endpoint, model-state, reqwest, serde_json, tokio, tokio-stream, tracing, vw-api-acp-model |
| 任务 167 → task-scheduler | 15: anyhow, async-trait, config-runtime-paths, project-types, rusqlite, serde_json, session-info, session-ui-types, tempfile, tokio, tracing, vw-api-agent-scheduler, vw-api-task, vw-api-task-lifecycle, vw-api-transport |
| 任务 173 → tool-browser | 15: anyhow, axum, base64, config-runtime-paths, directories, fantoccini, libc, reqwest, ring, rustls, serde_json, tempfile, time, tokio, vw-shared |
| 任务 183 → tool-mcp | 15: acp-config, acp-types, agent-client-protocol, anyhow, async-trait, hex, rmcp, serde, serde_json, sha2, tempfile, time, tokio, vw-api-tools, vw-api-types |
| 任务 204 → session-engine | 15: anyhow, async-trait, config-agent, config-agent-profile, futures-util, serde, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-api-chat, vw-api-chat-identity, vw-api-transport |
| 任务 242 → channel-lark | 15: anyhow, async-trait, axum, base64, futures-util, prost, reqwest, serde, serde_json, time, tokio, tokio-tungstenite, tower, tracing, uuid |
| 任务 249 → channel-runtime | 15: anyhow, async-trait, axum, config-agent, config-provider, futures-util, serde_json, tempfile, time, tokio, tokio-tungstenite, tokio-util, toml, tracing, vw-api-transport |
| 任务 265 → channel-wecom | 15: anyhow, async-trait, axum, config-runtime-paths, futures-util, pulldown-cmark, reqwest, serde_json, sha2, time, tokio, tokio-tungstenite, tracing, uuid, vw-api-transport |
| 任务 278 → agent-gateway | 15: anyhow, async-trait, axum, chrono, parking_lot, reqwest, serde_json, tempfile, time, tokio, tokio-stream, tower, tower-http, tracing, vw-api-transport |
| 任务 283 → gateway-agent-apps | 15: axum, hex, libc, reqwest, reqwest_012, serde, serde_json, sha2, tracing, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types, vw-shared |
| 任务 306 → gateway-config | 15: acp-agent-registry, anyhow, async-trait, axum, config-agent, config-agent-profile, config-gateway, parking_lot, serde, serde_json, tempfile, time, tokio, toml, vw-api-transport |
| 任务 307 → gateway-cron | 15: anyhow, async-trait, axum, chrono, chrono-tz, cron, parking_lot, serde_json, tempfile, time, tokio, tower, tracing, uuid, vw-api-transport |
| 任务 320 → gateway-knowledge | 15: anyhow, async-trait, axum, base64, config-runtime-paths, parking_lot, rusqlite, serde_json, time, tokio, tower, tracing, uuid, vw-api-knowledge, vw-api-transport |
| 任务 329 → gateway-project | 15: anyhow, axum, base64, git2, serde, serde_json, tempfile, tokio, tower, uuid, vw-agent-apps, vw-api-project, vw-api-transport, vw-api-types, vw-api-worktree |
| 任务 344 → gateway-usage | 15: anyhow, axum, chrono, futures-util, provider-model-cache, reqwest, rusqlite, serde, serde_json, time, tokio, uuid, vw-agent-apps, vw-api-transport, vw-api-types |
| 任务 64 → agent-app-services | 15: anyhow, base64, rusqlite, serde, serde_json, sha2, tempfile, time, tokio, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 73 → agent-knowledge | 15: anyhow, async-trait, base64, hex, mime_guess, postgres, rusqlite, serde_json, sha2, time, tokio, tracing, uuid, vw-api-knowledge, vw-api-transport |
| 任务 100 → model-protocol | 16: axum, base64, futures-util, model-protocols, model-state, parking_lot, provider-protocol-routes, reqwest, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-agent-apps, vw-api-transport |
| 任务 103 → model-protocol | 16: axum, chrono, futures-util, model-protocols, model-state, parking_lot, provider-protocol-routes, reqwest, serde_json, tempfile, time, tokio, tokio-util, uuid, vw-agent-apps, vw-api-transport |
| 任务 145 → app-provider-services | 16: base64, hex, rusqlite, serde, serde_json, sha2, tempfile, toml, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types |
| 任务 156 → llm-runtime | 16: acp-access-policy, acp-client, acp-host-io, acp-lifecycle, acp-storage, acp-types, agent-client-protocol, anyhow, config-agent-profile, parking_lot, serde_json, tempfile, tokio, tracing, vw-api-acp-model, vw-api-transport |
| 任务 166 → task-plan-runner | 16: anyhow, async-trait, config-agent, config-agent-profile, config-runtime-paths, futures-util, serde, serde_json, tempfile, time, tokio, tokio-util, tracing, uuid, vw-api-task, vw-api-transport |
| 任务 238 → channel-email-channel | 16: anyhow, async-imap, async-trait, chrono, futures-util, lettre, mail-parser, rustls, rustls-pki-types, serde_json, time, tokio, tokio-rustls, tracing, uuid, webpki-roots |
| 任务 276 → agent-gateway | 16: acp-agent-registry, anyhow, async-trait, axum, config-agent, parking_lot, reqwest, serde_json, tempfile, time, tokio, tower, tower-http, tracing, vw-api-tools, vw-api-transport |
| 任务 304 → gateway-channel | 16: axum, base64, chrono, config-channel-services, image, qrcode, rand, reqwest, ring, serde, serde_json, time, tokio, urlencoding, uuid, vw-api-transport |
| 任务 339 → gateway-skill-market | 16: async-trait, axum, base64, config-runtime-paths, directories, hex, ring, serde, serde_json, sha2, tempfile, tokio, tower, url, vw-gateway-client, zip |
| 任务 46 → agent-project | 16: anyhow, base64, config-runtime-paths, git2, postgres, project-types, rusqlite, serde, serde_json, tempfile, time, tokio, uuid, vw-api-agent-scheduler, vw-api-transport, walkdir |
| 任务 60 → computer-control | 16: anyhow, base64, config-computer-control, getrandom, hex, libc, parking_lot, serde, serde_json, tempfile, time, tokio, tokio-util, url, uuid, vw-api-types |
| 任务 83 → agent-provider | 16: base64, config-voice, futures-util, image, provider-errors, provider-model-cache, provider-model-discovery, reqwest, serde, serde_json, time, tokio, tokio-tungstenite, tokio-util, uuid, vw-api-types |
| 任务 88 → agent-skills | 16: acp-types, anyhow, config-skills, dialoguer, hex, libc, reqwest, serde, serde_json, sha2, tempfile, time, tokio, toml, tracing, vw-gateway-client |
| 任务 136 → agent-cron | 17: anyhow, axum, chrono, chrono-tz, cron, futures-util, reqwest, serde_json, tempfile, time, tokio, tokio-tungstenite, tracing, uuid, vw-api-acp-model, vw-api-chat, vw-api-transport |
| 任务 198 → workflow-engine | 17: anyhow, async-trait, axum, futures-util, git2, hex, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tracing, uuid, vw-api-workflow |
| 任务 203 → workflow-video-edit | 17: anyhow, async-trait, axum, hex, libc, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tracing, uuid, vw-api-transport, vw-api-workflow |
| 任务 247 → channel-qq | 17: anyhow, async-trait, futures-util, hex, regex, reqwest, ring, rustls, rustls-pki-types, serde_json, time, tokio, tokio-rustls, tokio-tungstenite, toml, tracing, uuid |
| 任务 293 → gateway-aggregation | 17: axum, hex, libc, reqwest, serde_json, sha2, tempfile, time, tokio, toml, tower, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types |
| 任务 77 → agent-memory | 17: anyhow, async-trait, axum, chrono, memory-algorithms, memory-response-cache, memory-snapshot, memory-sqlite, memory-types, postgres, reqwest, serde, serde_json, tempfile, tokio, tracing, uuid |
| 任务 154 → llm-runtime | 18: acp-agent-registry, anyhow, getrandom, hex, libc, model-protocols, model-reasoning, model-state, provider-protocol-routes, reqwest, serde_json, sha2, tempfile, time, tokio, toml, vw-api-acp-model, vw-shared |
| 任务 155 → llm-runtime | 18: acp-agent-registry, anyhow, async-trait, axum, getrandom, hex, hmac, libc, model-reasoning, model-state, ring, serde_json, sha2, tempfile, time, tokio, tokio-util, vw-api-transport |
| 任务 157 → llm-runtime | 18: acp-agent-registry, acp-client, acp-host-io, acp-types, agent-client-protocol, config-skills, parking_lot, regex, reqwest, serde_json, tempfile, time, tokio, tracing, vw-api-acp-model, vw-api-tools, vw-api-transport, vw-api-types |
| 任务 163 → plugin-mcp | 18: acp-config, agent-client-protocol, anyhow, chrono, hex, question-types, reqwest_012, rmcp, serde, serde_json, sha2, time, tokio, tokio-util, url, uuid, vw-api-tools, vw-api-transport |
| 任务 301 → gateway-apps-usage-acceptance | 18: axum, base64, gateway-endpoint, reqwest, ring, rustls, rustls-pki-types, serde_json, time, tokio, tokio-rustls, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 314 → gateway-global | 18: acp-agent-registry, anyhow, async-trait, axum, config-gateway, config-runtime-paths, parking_lot, reqwest, rusqlite, serde_json, tempfile, time, tokio, tokio-stream, tower, uuid, vw-api-transport, vw-shared |
| 任务 346 → gateway-workflow | 18: anyhow, async-trait, axum, config-runtime-paths, futures-util, parking_lot, reqwest, rusqlite, serde, serde_json, tempfile, time, tokio, tokio-stream, tower, uuid, vw-api-transport, vw-api-workflow |
| 任务 67 → agent-app-services | 18: anyhow, hex, libc, postgres, rusqlite, serde, serde_json, sha2, tempfile, thiserror, time, tracing, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types |
| 任务 99 → model-protocol | 18: axum, futures-util, hex, model-protocols, parking_lot, provider-model-discovery, provider-protocol-routes, reqwest, serde_json, sha2, tempfile, time, tokio, tokio-util, url, uuid, vw-agent-apps, vw-api-transport |
| 任务 133 → workflow-core | 19: anyhow, axum, config-runtime-paths, hex, minijinja, postgres, regex, rusqlite, serde_json, serde_yaml, sha2, tempfile, thiserror, time, tokio, tower, uuid, vw-api-transport, vw-api-workflow |
| 任务 164 → plugin-mcp | 19: acp-config, agent-client-protocol, anyhow, base64, getrandom, hex, libc, question-types, reqwest_012, rmcp, serde, serde_json, sha2, time, tokio, tokio-util, tracing, url, vw-api-transport |
| 任务 201 → workflow-video-edit | 19: anyhow, async-trait, axum, chrono, futures-util, hex, reqwest, rusqlite, serde, serde_json, sha2, shell-words, tempfile, time, tokio, tokio-util, tracing, uuid, vw-api-transport |
| 任务 202 → workflow-video-edit | 19: anyhow, async-trait, hex, rusqlite, serde, serde_json, serde_yaml, sha2, shell-words, tempfile, time, tokio, tokio-util, tracing, url, uuid, vw-api-chat, vw-api-transport, vw-api-workflow |
| 任务 217 → agent-acp | 19: acp-config, acp-server, acp-types, agent-client-protocol, anyhow, async-trait, hex, permission-types, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, vw-agent-apps, vw-api-transport, vw-api-types, walkdir |
| 任务 298 → gateway-app-providers | 19: axum, futures-util, gateway-app-provider, gateway-endpoint, hex, reqwest, serde, serde_json, sha2, tempfile, tokio, tower, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 300 → gateway-app-routing | 19: anyhow, axum, gateway-endpoint, hex, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tracing, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types, vw-gateway-client, vw-shared |
| 任务 311 → gateway-external-sessions | 19: anyhow, axum, chrono, futures-util, libc, rusqlite, serde, serde_json, sha2, time, tokio, tracing, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-task, vw-api-transport, vw-api-types |
| 任务 345 → gateway-voice | 19: anyhow, axum, base64, config-voice, futures-util, image, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-tungstenite, tokio-util, uuid, vw-api-chat, vw-api-transport, vw-api-types |
| 任务 62 → computer-control | 19: anyhow, config-computer-control, futures-util, libc, parking_lot, ring, rustls, rustls-pki-types, serde, serde_json, tempfile, time, tokio, tokio-rustls, tokio-tungstenite, tokio-util, url, uuid, vw-api-types |
| 任务 87 → agent-skills | 19: anyhow, async-trait, base64, config-skills, futures-util, hex, reqwest, ring, serde, serde_json, serde_yaml, sha2, tempfile, thiserror, time, tokio, url, vw-gateway-client, zip |
| 任务 168 → task-scheduler | 20: anyhow, chrono, cron, directories, git2, hex, parking_lot, serde_json, sha2, tempfile, thiserror, time, tokio, tokio-util, tracing, vw-api-acp-model, vw-api-chat, vw-api-chat-identity, vw-api-task, vw-api-transport |
| 任务 176 → tool-computer | 20: anyhow, async-trait, base64, config-computer-control, hex, image, libc, parking_lot, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, url, uuid, vw-api-tools, vw-api-transport, vw-api-types |
| 任务 277 → agent-gateway | 20: anyhow, async-trait, axum, base64, chrono, futures-util, hex, hmac, parking_lot, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, tracing, uuid, vw-api-transport |
| 任务 321 → gateway-mcp | 20: acp-server, anyhow, async-trait, axum, hex, parking_lot, serde, serde_json, sha2, tempfile, time, tokio, tower, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-task, vw-api-transport, vw-api-types |
| 任务 97 → mcp-client | 20: acp-config, acp-types, agent-client-protocol, anyhow, axum, config-plugins, futures-util, libc, reqwest_012, rmcp, serde_json, sse-stream, tempfile, time, tokio, tokio-util, tracing, url, vw-api-transport, vw-shared |
| 任务 106 → model-transport | 21: anyhow, async-trait, axum, chrono, futures-util, getrandom, hex, hmac, model-protocols, model-reasoning, provider-protocol-routes, reqwest, ring, serde_json, tempfile, time, tokio, tokio-util, vw-agent-apps, vw-api-transport, vw-api-types |
| 任务 134 → worktree-core | 21: anyhow, async-trait, axum, config-runtime-paths, config-worktree, git2, hex, libc, project-types, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, tracing, uuid, vw-api-transport, vw-api-worktree |
| 任务 205 → session-engine | 21: anyhow, async-trait, axum, config-agent, gateway-endpoint, gateway-session, hex, permission-types, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, uuid, vw-api-chat, vw-api-tools, vw-api-transport, vw-gateway-client, web-time |
| 任务 288 → gateway-agent-apps | 21: anyhow, axum, gateway-endpoint, hex, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tower, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types, vw-gateway-client, zip |
| 任务 291 → gateway-aggregation | 21: axum, hex, libc, reqwest, serde_json, sha2, tempfile, time, tokio, toml, toml_edit, tower, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-chat-identity, vw-api-model-catalog, vw-api-transport, vw-api-types |
| 任务 297 → gateway-app-providers | 21: axum, gateway-app-provider, gateway-endpoint, gateway-http, hex, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tower, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 308 → gateway-desktop | 21: acp-agent-registry, acp-client, acp-types, axum, config-acp-install, config-runtime-paths, config-skills, include_dir, once_cell, serde, serde_json, serde_yaml, tempfile, time, tokio, tokio-util, toml, urlencoding, vw-api-file, vw-api-transport, vw-gateway-client |
| 任务 35 → agent-config | 21: acp-agent-registry, anyhow, chrono, config-agent, config-agent-profile, config-aggregate, config-provider, config-proxy, config-runtime-paths, config-security, config-skills, reqwest, serde, serde_ignored, serde_json, tempfile, time, tokio, toml, tracing, vw-api-transport |
| 任务 101 → model-protocol | 22: axum, chrono, futures-util, getrandom, hex, model-protocols, model-reasoning, model-state, model-types, parking_lot, provider-protocol-routes, reqwest, ring, serde_json, tempfile, time, tokio, tokio-util, vw-agent-apps, vw-api-app-identity, vw-api-model-catalog, vw-api-transport |
| 任务 148 → app-routing-services | 22: anyhow, axum, chrono, futures-util, hex, httparse, hyper, reqwest, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, tracing, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types |
| 任务 153 → llm-runtime | 22: acp-agent-registry, acp-storage, anyhow, async-trait, axum, hex, libc, model-protocols, model-reasoning, model-state, reqwest, serde_json, sha2, tempfile, time, tokio, tokio-util, toml, url, uuid, vw-api-acp-model, vw-api-transport |
| 任务 158 → llm-runtime | 22: aisdk, anyhow, async-trait, axum, futures-util, hex, model-reasoning, model-state, model-types, provider-protocol-routes, reqwest, schemars, serde_json, sha2, tempfile, time, tokio, uuid, vw-api-app-identity, vw-api-model-catalog, vw-api-tools, vw-api-transport |
| 任务 185 → tool-office | 22: anyhow, async-trait, config-computer-control, futures-util, hex, libc, parking_lot, ring, rustls, rustls-pki-types, serde, serde_json, sha2, tempfile, thiserror, time, tokio, tokio-rustls, tokio-tungstenite, tokio-util, uuid, vw-api-types |
| 任务 285 → gateway-agent-apps | 22: axum, base64, chrono, config-runtime-paths, futures-util, hex, parking_lot, reqwest, serde_json, sha2, tempfile, time, tokio, tower, tracing, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types |
| 任务 65 → agent-app-services | 22: acp-server, acp-types, agent-client-protocol, anyhow, axum, futures-util, hex, reqwest_012, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 151 → llm-runtime | 23: acp-access-policy, acp-agent-registry, acp-cli-model, acp-client, acp-host-io, acp-storage, acp-types, agent-client-protocol, anyhow, config-agent-profile, hex, parking_lot, serde, serde_json, sha2, tempfile, tokio, tokio-util, tracing, uuid, vw-api-acp-model, vw-api-chat-identity, vw-api-transport |
| 任务 312 → gateway-file | 23: anyhow, async-trait, axum, base64, config-runtime-paths, encoding_rs, gateway-endpoint, gateway-files, gateway-http, libc, parking_lot, reqwest, serde, serde_json, tempfile, time, tokio, tower, url, uuid, vw-api-file, vw-api-transport, walkdir |
| 任务 66 → agent-app-services | 23: agent-client-protocol, anyhow, axum, futures-util, hex, libc, parking_lot, reqwest_012, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, tracing, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-task, vw-api-transport, vw-api-types |
| 任务 82 → agent-provider | 23: axum, gateway-endpoint, gateway-http, gateway-model-provider, model-reasoning, model-state, model-types, provider-model-cache, provider-model-discovery, serde, serde_json, strsim, tempfile, thiserror, time, tokio, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-model-catalog, vw-api-transport, vw-gateway-client |
| 任务 147 → app-routing-services | 24: anyhow, async-compression, axum, flate2, futures-util, gateway-endpoint, hex, parking_lot, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tower, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 272 → agent-gateway | 24: anyhow, axum, chrono, hex, model-protocols, provider-protocol-routes, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tracing, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-model-catalog, vw-api-transport, vw-api-types |
| 任务 49 → agent-security | 24: anyhow, chrono, config-gateway, config-runtime-paths, content-guard, domain-matcher, gateway-pairing, hex, hmac, parking_lot, postgres, process-sandbox, rand, ring, rusqlite, serde, serde_json, sha2, shellexpand, tempfile, time, tracing, uuid, vw-api-transport |
| 任务 76 → agent-memory | 24: anyhow, async-trait, chrono, clap, config-runtime-paths, console, dialoguer, directories, memory-algorithms, memory-lucid, memory-mariadb, memory-markdown, memory-postgres, memory-types, postgres, project-storage-paths, reqwest, rusqlite, serde, serde_json, tempfile, time, tokio, tracing |
| 任务 286 → gateway-agent-apps | 25: anyhow, axum, base64, gateway-app-read, gateway-endpoint, gateway-http, hex, libc, reqwest, serde, serde_json, serde_yaml, sha2, tempfile, time, tokio, tower, tracing, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-transport, vw-gateway-client, vw-shared |
| 任务 295 → gateway-app-providers | 25: axum, base64, gateway-app-provider, gateway-endpoint, gateway-http, hex, reqwest, rusqlite, serde, serde_json, serde_yaml, sha2, tempfile, time, tokio, tower, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 296 → gateway-app-providers | 25: axum, futures-util, gateway-app-provider, gateway-app-read, gateway-endpoint, gateway-http, hex, reqwest, serde, serde_json, serde_yaml, sha2, tempfile, time, tokio, toml, tower, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 325 → gateway-model | 26: axum, futures-util, gateway-endpoint, gateway-http, gateway-model-catalog, hex, libc, provider-model-cache, provider-runtime-paths, reqwest, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-model-catalog, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 334 → gateway-session | 26: acp-agent-registry, anyhow, async-trait, axum, config-agent-profile, futures-util, hex, parking_lot, reqwest, rusqlite, serde, serde_json, session-ui-types, sha2, tempfile, time, tokio, tower, tracing, url, uuid, vw-api-acp-model, vw-api-chat, vw-api-session, vw-api-todo, vw-api-transport |
| 任务 150 → app-routing-services | 27: anyhow, axum, flate2, futures-util, getrandom, hex, hyper, hyper-util, parking_lot, reqwest, ring, rustls, rustls-pki-types, rustls-platform-verifier, serde, serde_json, sha2, time, tokio, tokio-rustls, tokio-util, url, uuid, vw-agent-apps, vw-api-app-identity, vw-api-transport, vw-api-types |
| 任务 294 → gateway-app-providers | 27: axum, base64, gateway-app-provider, gateway-app-settings, gateway-endpoint, gateway-http, hex, reqwest, rusqlite, serde, serde_json, sha2, tempfile, time, tokio, tower, tracing, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-model-catalog, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 287 → gateway-agent-apps | 28: anyhow, axum, chrono, gateway-app-diagnostics, gateway-app-failover, gateway-endpoint, gateway-http, hex, libc, reqwest, rusqlite, serde, serde_json, sha2, tempfile, time, tokio, tokio-util, tower, tracing, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client |
| 任务 292 → gateway-aggregation | 28: axum, base64, chrono, config-runtime-paths, getrandom, hex, hmac, libc, reqwest, ring, serde, serde_json, sha2, tempfile, time, tokio, toml_edit, tower, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-model-catalog, vw-api-task, vw-api-transport, vw-api-types |
| 任务 336 → gateway-session | 28: aisdk, anyhow, axum, config-agent, config-agent-profile, config-runtime-paths, futures-util, git2, parking_lot, rusqlite, serde, serde_json, session-info, session-paths, tempfile, time, tokio, tokio-util, tower, tracing, uuid, vw-api-acp-model, vw-api-chat, vw-api-chat-identity, vw-api-session, vw-api-todo, vw-api-transport, vw-api-workflow |
| 任务 273 → agent-gateway | 29: anyhow, async-trait, axum, chrono, futures-util, libc, parking_lot, postgres, reqwest, rusqlite, serde, serde_json, sha2, tempfile, thiserror, time, tokio, tokio-util, toml, tower, tracing, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-app-provider-profiles, vw-api-transport, vw-api-types, zip |
| 任务 356 → agent-composition | 29: anyhow, chrono, config-gateway, config-runtime-paths, cron, gateway-pairing, git-discovery, git2, libc, memory-lucid, memory-mariadb, memory-markdown, memory-postgres, memory-response-cache, memory-snapshot, memory-sqlite, memory-types, ordered-id, postgres, process-sandbox, project-storage-paths, rusqlite, secret-store, serde_json, tempfile, time, tokio, vw-api-transport, vw-shared |
| 任务 284 → gateway-agent-apps | 31: axum, base64, futures-util, gateway-app-lifecycle, gateway-app-settings, gateway-endpoint, gateway-http, hex, parking_lot, reqwest, ring, rustls, rustls-pki-types, serde, serde_json, sha2, tempfile, time, tokio, tokio-rustls, tower, uuid, vw-agent-apps, vw-api-app-auth, vw-api-app-identity, vw-api-app-provider-base, vw-api-task, vw-api-transport, vw-api-types, vw-gateway-client, vw-shared |
| 任务 149 → app-routing-services | 32: anyhow, axum, chrono, flate2, futures-util, glob, hyper-util, parking_lot, reqwest, ring, rustls, rustls-pki-types, serde, serde_json, sha2, system-configuration, tempfile, thiserror, time, tokio, tokio-rustls, tokio-util, tower, tracing, url, uuid, vw-agent-apps, vw-api-app-identity, vw-api-app-provider-base, vw-api-model-catalog, vw-api-transport, vw-api-types |
| 任务 109 → plugin-install | 37: acp-config, acp-types, agent-client-protocol, anyhow, async-trait, axum, base64, clap, config-agent, config-plugins, flate2, gateway-endpoint, getrandom, git2, hex, libc, parking_lot, regex, reqwest_012, rmcp, serde, serde_json, serde_yaml, sha2, tar, tempfile, time, tokio, tokio-util, tower, tracing, url, uuid, vw-api-tools, vw-api-transport, vw-gateway-client, walkdir |

## 逐源文件归属（完整快照）

| 文件 | 行数 | 归属/保留理由 |
| --- | --- | --- |
| `Cargo.toml` | 328 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/runtime-skills/create-task-plans-runtime/SKILL.md` | 252 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/runtime-skills/do-task-plans-runtime/SKILL.md` | 36 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/security/attack-corpus-v1.jsonl` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/skills/README.md` | 3 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/skills/create-task-plans/SKILL.md` | 227 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/skills/do-task-plans/SKILL.md` | 34 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/skills/rustcodegraph/README.md` | 43 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `assets/skills/rustcodegraph/SKILL.md` | 93 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `readmd.md` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `resources/workflows/cloud_video_edit.yaml` | 72 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `src/acp/approval.rs` | 108 | 任务 217 → agent-acp |
| `src/acp/approval_tests.rs` | 204 | 任务 217 → agent-acp |
| `src/acp/backend.rs` | 858 | 任务 217 → agent-acp |
| `src/acp/backend_tests.rs` | 196 | 任务 217 → agent-acp |
| `src/acp/catalog_mounts.rs` | 118 | 任务 217 → agent-acp |
| `src/acp/catalog_mounts_tests.rs` | 158 | 任务 217 → agent-acp |
| `src/acp/mcp.rs` | 102 | 任务 217 → agent-acp |
| `src/acp/mcp_tests.rs` | 152 | 任务 217 → agent-acp |
| `src/acp/mod.rs` | 38 | 任务 217 → agent-acp |
| `src/acp/model_tests.rs` | 83 | 任务 217 → agent-acp |
| `src/acp/models.rs` | 95 | 任务 217 → agent-acp |
| `src/acp/mount_acceptance_tests.rs` | 98 | 任务 217 → agent-acp |
| `src/acp/mount_recovery_tests.rs` | 420 | 任务 217 → agent-acp |
| `src/acp/mount_store.rs` | 81 | 任务 217 → agent-acp |
| `src/acp/mount_update_tests.rs` | 238 | 任务 217 → agent-acp |
| `src/acp/mounts.rs` | 184 | 任务 217 → agent-acp |
| `src/acp/mounts_tests.rs` | 364 | 任务 217 → agent-acp |
| `src/agent/agent/builder.rs` | 314 | 任务 218 → agent-loop |
| `src/agent/agent/builder_tests.rs` | 275 | 任务 218 → agent-loop |
| `src/agent/agent/core.rs` | 420 | 任务 218 → agent-loop |
| `src/agent/agent/core_tests.rs` | 441 | 任务 218 → agent-loop |
| `src/agent/agent/mod.rs` | 39 | 任务 218 → agent-loop |
| `src/agent/agent/run.rs` | 125 | 任务 218 → agent-loop |
| `src/agent/agent/run_tests.rs` | 332 | 任务 218 → agent-loop |
| `src/agent/agent/tests.rs` | 761 | 任务 218 → agent-loop |
| `src/agent/agent/turn.rs` | 225 | 任务 218 → agent-loop |
| `src/agent/agent/turn_skills_tests.rs` | 259 | 任务 218 → agent-loop |
| `src/agent/agent/turn_tests.rs` | 566 | 任务 218 → agent-loop |
| `src/agent/catalog.rs` | 38 | 任务 218 → agent-loop |
| `src/agent/catalog_tests.rs` | 326 | 任务 218 → agent-loop |
| `src/agent/classifier/mod.rs` | 171 | 任务 218 → agent-loop |
| `src/agent/classifier/tests.rs` | 275 | 任务 218 → agent-loop |
| `src/agent/defaults.rs` | 160 | 任务 218 → agent-loop |
| `src/agent/defaults_tests.rs` | 93 | 任务 218 → agent-loop |
| `src/agent/dispatcher/mod.rs` | 570 | 任务 218 → agent-loop |
| `src/agent/dispatcher/mod_tests.rs` | 389 | 任务 218 → agent-loop |
| `src/agent/dispatcher/tests.rs` | 313 | 任务 218 → agent-loop |
| `src/agent/generate.txt` | 76 | 任务 218 → agent-loop |
| `src/agent/loop_/approval.rs` | 156 | 任务 219 → agent-loop |
| `src/agent/loop_/approval_tests.rs` | 199 | 任务 219 → agent-loop |
| `src/agent/loop_/context.rs` | 81 | 任务 219 → agent-loop |
| `src/agent/loop_/context_tests.rs` | 176 | 任务 219 → agent-loop |
| `src/agent/loop_/core/constants.rs` | 220 | 任务 220 → agent-loop |
| `src/agent/loop_/core/constants_tests.rs` | 16 | 任务 220 → agent-loop |
| `src/agent/loop_/core/errors.rs` | 100 | 任务 220 → agent-loop |
| `src/agent/loop_/core/errors_tests.rs` | 73 | 任务 220 → agent-loop |
| `src/agent/loop_/core/history.rs` | 281 | 任务 220 → agent-loop |
| `src/agent/loop_/core/history_tests.rs` | 169 | 任务 220 → agent-loop |
| `src/agent/loop_/core/mod.rs` | 66 | 任务 221 → agent-loop |
| `src/agent/loop_/core/skills.rs` | 261 | 任务 220 → agent-loop |
| `src/agent/loop_/core/timeouts.rs` | 145 | 任务 220 → agent-loop |
| `src/agent/loop_/core/timeouts_tests.rs` | 28 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/collaboration.rs` | 44 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/detection.rs` | 42 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/detection_tests.rs` | 79 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/discovery_tests.rs` | 809 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/llm_round.rs` | 409 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/llm_round_tests.rs` | 624 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/output.rs` | 105 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/output_tests.rs` | 132 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/tool_execution.rs` | 434 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop/tool_execution_tests.rs` | 725 | 任务 220 → agent-loop |
| `src/agent/loop_/core/tool_loop.rs` | 565 | 任务 221 → agent-loop |
| `src/agent/loop_/core/tool_loop_tests.rs` | 663 | 任务 221 → agent-loop |
| `src/agent/loop_/core/turn.rs` | 353 | 任务 221 → agent-loop |
| `src/agent/loop_/core/turn_tests.rs` | 418 | 任务 221 → agent-loop |
| `src/agent/loop_/cron.rs` | 178 | 任务 222 → agent-loop |
| `src/agent/loop_/cron_tests.rs` | 167 | 任务 222 → agent-loop |
| `src/agent/loop_/execution.rs` | 368 | 任务 222 → agent-loop |
| `src/agent/loop_/execution_tests.rs` | 474 | 任务 222 → agent-loop |
| `src/agent/loop_/history.rs` | 187 | 任务 222 → agent-loop |
| `src/agent/loop_/history_tests.rs` | 177 | 任务 222 → agent-loop |
| `src/agent/loop_/instructions.rs` | 251 | 任务 222 → agent-loop |
| `src/agent/loop_/instructions_tests.rs` | 209 | 任务 222 → agent-loop |
| `src/agent/loop_/parsing/deepseek_dsml.rs` | 146 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/json.rs` | 497 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/json_tests.rs` | 236 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/minimax_xml.rs` | 297 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/minimax_xml_tests.rs` | 105 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/mod.rs` | 69 | 任务 224 → agent-loop |
| `src/agent/loop_/parsing/tests.rs` | 58 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/tool_call_formats.rs` | 986 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/tool_call_formats_tests.rs` | 497 | 任务 223 → agent-loop |
| `src/agent/loop_/parsing/tool_calls.rs` | 688 | 任务 224 → agent-loop |
| `src/agent/loop_/parsing/tool_calls_tests.rs` | 470 | 任务 224 → agent-loop |
| `src/agent/loop_/parsing/xml_helpers.rs` | 339 | 任务 224 → agent-loop |
| `src/agent/loop_/parsing/xml_helpers_tests.rs` | 106 | 任务 224 → agent-loop |
| `src/agent/loop_/progress.rs` | 343 | 任务 225 → agent-loop |
| `src/agent/loop_/progress_tests.rs` | 110 | 任务 225 → agent-loop |
| `src/agent/loop_/query_engine.rs` | 654 | 任务 225 → agent-loop |
| `src/agent/loop_/query_engine_skills_tests.rs` | 209 | 任务 225 → agent-loop |
| `src/agent/loop_/query_engine_tests.rs` | 1057 | 任务 225 → agent-loop |
| `src/agent/loop_/runner.rs` | 70 | 任务 225 → agent-loop |
| `src/agent/loop_/runner_tests.rs` | 110 | 任务 225 → agent-loop |
| `src/agent/loop_/tests/constants.rs` | 77 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/cron_delivery.rs` | 96 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools/approvals.rs` | 256 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools/multimodal.rs` | 131 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools/native_mode.rs` | 95 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools/parallel_execution.rs` | 156 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools/recovery.rs` | 287 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/loop_tools.rs` | 261 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/memory_context.rs` | 446 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/mod.rs` | 73 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_edge_cases.rs` | 256 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/parsing_extract_json.rs` | 86 | 任务 226 → agent-loop |
| `src/agent/loop_/tests/parsing_glm.rs` | 401 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_protocols.rs` | 80 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_recovery.rs` | 125 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_structured.rs` | 92 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_tool_calls.rs` | 694 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/parsing_value.rs` | 161 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/shell_policy.rs` | 113 | 任务 227 → agent-loop |
| `src/agent/loop_/tests/tool_instructions.rs` | 97 | 任务 227 → agent-loop |
| `src/agent/loop_/utils.rs` | 126 | 任务 228 → agent-loop |
| `src/agent/loop_/utils_tests.rs` | 102 | 任务 228 → agent-loop |
| `src/agent/loop_.rs` | 206 | 任务 229 → agent-loop |
| `src/agent/memory_loader.rs` | 217 | 任务 229 → agent-loop |
| `src/agent/memory_loader_tests.rs` | 167 | 任务 229 → agent-loop |
| `src/agent/mod.rs` | 521 | 任务 232 → agent-loop |
| `src/agent/mod_tests.rs` | 478 | 任务 229 → agent-loop |
| `src/agent/prompt/coder.txt` | 14 | 任务 229 → agent-loop |
| `src/agent/prompt/compaction.txt` | 14 | 任务 229 → agent-loop |
| `src/agent/prompt/explore.txt` | 15 | 任务 229 → agent-loop |
| `src/agent/prompt/mod.rs` | 664 | 任务 229 → agent-loop |
| `src/agent/prompt/mod_tests.rs` | 515 | 任务 229 → agent-loop |
| `src/agent/prompt/multi_agent.rs` | 31 | 任务 229 → agent-loop |
| `src/agent/prompt/researcher.txt` | 14 | 任务 229 → agent-loop |
| `src/agent/prompt/reviewer.txt` | 14 | 任务 229 → agent-loop |
| `src/agent/prompt/summary.txt` | 11 | 任务 229 → agent-loop |
| `src/agent/prompt/tests.rs` | 466 | 任务 229 → agent-loop |
| `src/agent/prompt/title.txt` | 44 | 任务 229 → agent-loop |
| `src/agent/tests/helpers.rs` | 685 | 任务 230 → agent-loop |
| `src/agent/tests/loop_history.rs` | 182 | 任务 230 → agent-loop |
| `src/agent/tests/loop_memory.rs` | 110 | 任务 230 → agent-loop |
| `src/agent/tests/loop_text.rs` | 172 | 任务 230 → agent-loop |
| `src/agent/tests/loop_tools.rs` | 464 | 任务 230 → agent-loop |
| `src/agent/tests/memory_loader.rs` | 253 | 任务 230 → agent-loop |
| `src/agent/tests/mod.rs` | 29 | 任务 231 → agent-loop |
| `src/agent/tests/run_single.rs` | 37 | 任务 230 → agent-loop |
| `src/agent/tests/serialization.rs` | 110 | 任务 230 → agent-loop |
| `src/agent/tests/tool_dispatcher.rs` | 322 | 任务 231 → agent-loop |
| `src/agent_apps/auth_accounts/defaults.rs` | 97 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts/gemini.rs` | 71 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts/logout.rs` | 153 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts/read.rs` | 157 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts/removal.rs` | 58 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts.rs` | 1119 | 任务 64 → agent-app-services |
| `src/agent_apps/auth_accounts_tests.rs` | 1538 | 任务 64 → agent-app-services |
| `src/agent_apps/data_import.rs` | 184 | 任务 64 → agent-app-services |
| `src/agent_apps/mcp/acp_mount.rs` | 147 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/acp_mount_tests.rs` | 227 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/binding.rs` | 309 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/binding_tests.rs` | 451 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/bulk.rs` | 75 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/bulk_tests.rs` | 156 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_create.rs` | 86 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_create_tests.rs` | 171 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_delete.rs` | 86 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_delete_tests.rs` | 131 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_update.rs` | 68 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/catalog_update_tests.rs` | 149 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/connection_probe_tests.rs` | 150 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/connectivity.rs` | 205 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/connectivity_tests.rs` | 397 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/credentials.rs` | 371 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/credentials_tests.rs` | 331 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/data_migration.rs` | 248 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/data_migration_tests.rs` | 98 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/host_scope_tests.rs` | 226 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/import.rs` | 150 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/import_tests.rs` | 230 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/mod.rs` | 217 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/network_probe/details.rs` | 210 | 任务 65 → agent-app-services |
| `src/agent_apps/mcp/network_probe.rs` | 459 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/network_probe_tests.rs` | 462 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/probe_jobs.rs` | 159 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/probe_jobs_tests.rs` | 499 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/profile_slot.rs` | 123 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/resync.rs` | 237 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/resync_tests.rs` | 333 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/revocation.rs` | 135 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/revocation_tests.rs` | 251 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/runtime.rs` | 263 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/runtime_tests.rs` | 261 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/security_tests.rs` | 445 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/stdio_probe_tests.rs` | 173 | 任务 66 → agent-app-services |
| `src/agent_apps/mcp/tool_details_tests.rs` | 160 | 任务 66 → agent-app-services |
| `src/agent_apps/mod.rs` | 34 | 任务 67 → agent-app-services |
| `src/agent_apps/postgres.rs` | 58 | 任务 67 → agent-app-services |
| `src/agent_apps/postgres_tests.rs` | 158 | 任务 67 → agent-app-services |
| `src/agent_apps/preferences_import.rs` | 27 | 任务 67 → agent-app-services |
| `src/agent_apps/profile_prompt_store.rs` | 51 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_activation_store.rs` | 68 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_data_migration.rs` | 114 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_data_migration_tests.rs` | 44 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_deactivation_store.rs` | 98 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_sync_store.rs` | 81 | 任务 67 → agent-app-services |
| `src/agent_apps/prompt_toggle_store.rs` | 205 | 任务 67 → agent-app-services |
| `src/agent_apps/prompts_bootstrap.rs` | 90 | 任务 67 → agent-app-services |
| `src/agent_apps/prompts_store.rs` | 299 | 任务 67 → agent-app-services |
| `src/agent_apps/shared_snapshot.rs` | 210 | 任务 67 → agent-app-services |
| `src/agent_apps/shared_snapshot_import.rs` | 209 | 任务 67 → agent-app-services |
| `src/agent_apps/sqlite.rs` | 41 | 任务 67 → agent-app-services |
| `src/agent_apps/sqlite_tests.rs` | 74 | 任务 67 → agent-app-services |
| `src/agent_apps/store.rs` | 114 | 任务 67 → agent-app-services |
| `src/agent_apps/store_tests.rs` | 88 | 任务 67 → agent-app-services |
| `src/agent_apps/usage_settings.rs` | 181 | 任务 67 → agent-app-services |
| `src/agent_apps/usage_settings_tests.rs` | 130 | 任务 67 → agent-app-services |
| `src/agent_scheduler/adapters/dispatch.rs` | 33 | 任务 167 → task-scheduler |
| `src/agent_scheduler/adapters/local.rs` | 13 | 任务 167 → task-scheduler |
| `src/agent_scheduler/adapters/local_tests.rs` | 76 | 任务 167 → task-scheduler |
| `src/agent_scheduler/adapters/mod.rs` | 9 | 任务 167 → task-scheduler |
| `src/agent_scheduler/artifacts.rs` | 423 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/artifacts_tests.rs` | 365 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/domain.rs` | 546 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/domain_tests.rs` | 146 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/engine/dispatch.rs` | 343 | 任务 167 → task-scheduler |
| `src/agent_scheduler/engine/mod.rs` | 5 | 任务 167 → task-scheduler |
| `src/agent_scheduler/engine/recovery.rs` | 349 | 任务 167 → task-scheduler |
| `src/agent_scheduler/events.rs` | 139 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/events_tests.rs` | 11 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/integration_tests.rs` | 445 | 任务 167 → task-scheduler |
| `src/agent_scheduler/lifecycle_bridge.rs` | 282 | 任务 167 → task-scheduler |
| `src/agent_scheduler/lifecycle_bridge_tests.rs` | 167 | 任务 167 → task-scheduler |
| `src/agent_scheduler/local_e2e_tests.rs` | 195 | 任务 167 → task-scheduler |
| `src/agent_scheduler/mod.rs` | 95 | 任务 169 → task-scheduler |
| `src/agent_scheduler/outbox_dispatcher.rs` | 160 | 任务 167 → task-scheduler |
| `src/agent_scheduler/policy.rs` | 699 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/policy_tests.rs` | 1174 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/resources/mod.rs` | 42 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/resources/worktree.rs` | 106 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/resources/worktree_tests.rs` | 222 | 任务 124 → task-scheduler-core |
| `src/agent_scheduler/runner/gateway_executor.rs` | 640 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/gateway_executor_tests.rs` | 988 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/local.rs` | 942 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/local_tests.rs` | 961 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/mod.rs` | 10 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/workspace_claim.rs` | 176 | 任务 168 → task-scheduler |
| `src/agent_scheduler/runner/workspace_claim_tests.rs` | 427 | 任务 168 → task-scheduler |
| `src/agent_scheduler/scheduler/mod.rs` | 7 | 任务 168 → task-scheduler |
| `src/agent_scheduler/scheduler/recurring.rs` | 143 | 任务 168 → task-scheduler |
| `src/agent_scheduler/scheduler/recurring_tests.rs` | 165 | 任务 168 → task-scheduler |
| `src/agent_scheduler/state_machine.rs` | 251 | 任务 125 → task-scheduler-core |
| `src/agent_scheduler/state_machine_tests.rs` | 101 | 任务 125 → task-scheduler-core |
| `src/agent_scheduler/store/artifacts.rs` | 184 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/artifacts_tests.rs` | 293 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/dependencies.rs` | 252 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/dependencies_tests.rs` | 144 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/events.rs` | 204 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/events_tests.rs` | 263 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/logs.rs` | 339 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/logs_tests.rs` | 378 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/mod.rs` | 336 | 任务 130 → task-scheduler-core |
| `src/agent_scheduler/store/mod_tests.rs` | 40 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/outbox.rs` | 409 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/outbox_tests.rs` | 582 | 任务 126 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/artifacts.rs` | 177 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/artifacts_tests.rs` | 148 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/dependencies.rs` | 230 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/dependencies_tests.rs` | 146 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/events.rs` | 180 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/events_tests.rs` | 70 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/logs.rs` | 364 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/logs_tests.rs` | 136 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/mod.rs` | 95 | 任务 128 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/mod_tests.rs` | 76 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/outbox.rs` | 315 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/outbox_tests.rs` | 74 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/resources.rs` | 357 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/resources_tests.rs` | 141 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/runs.rs` | 517 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/runs_tests.rs` | 117 | 任务 127 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/schema.rs` | 327 | 任务 128 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/schema_tests.rs` | 117 | 任务 128 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/tasks.rs` | 565 | 任务 128 → task-scheduler-core |
| `src/agent_scheduler/store/postgres/tasks_tests.rs` | 44 | 任务 128 → task-scheduler-core |
| `src/agent_scheduler/store/resources.rs` | 564 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/resources_tests.rs` | 77 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/resources_unit_tests.rs` | 151 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/runs.rs` | 567 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/runs_tests.rs` | 626 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/schema.rs` | 334 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/schema_tests.rs` | 656 | 任务 129 → task-scheduler-core |
| `src/agent_scheduler/store/tasks.rs` | 854 | 任务 130 → task-scheduler-core |
| `src/agent_scheduler/store/tasks_tests.rs` | 1075 | 任务 130 → task-scheduler-core |
| `src/agent_scheduler/task_board.rs` | 1039 | 任务 169 → task-scheduler |
| `src/agent_scheduler/task_board_tests.rs` | 386 | 任务 169 → task-scheduler |
| `src/agent_scheduler/task_graph.rs` | 205 | 任务 131 → task-scheduler-core |
| `src/agent_scheduler/task_graph_tests.rs` | 128 | 任务 131 → task-scheduler-core |
| `src/agent_scheduler_policy_legacy_review_exact_tests.rs` | 70 | 任务 356 → agent-composition |
| `src/app.rs` | 697 | 任务 356 → agent-composition |
| `src/app_providers/account_removal.rs` | 75 | 任务 145 → app-provider-services |
| `src/app_providers/agy.rs` | 169 | 任务 145 → app-provider-services |
| `src/app_providers/agy_binding.rs` | 189 | 任务 145 → app-provider-services |
| `src/app_providers/agy_form.rs` | 177 | 任务 145 → app-provider-services |
| `src/app_providers/basic.rs` | 101 | 任务 145 → app-provider-services |
| `src/app_providers/claude.rs` | 129 | 任务 145 → app-provider-services |
| `src/app_providers/claude_binding.rs` | 241 | 任务 145 → app-provider-services |
| `src/app_providers/claude_desktop.rs` | 63 | 任务 145 → app-provider-services |
| `src/app_providers/codex_binding.rs` | 175 | 任务 145 → app-provider-services |
| `src/app_providers/codex_binding_tests.rs` | 121 | 任务 145 → app-provider-services |
| `src/app_providers/codex_catalog.rs` | 76 | 任务 145 → app-provider-services |
| `src/app_providers/data_import.rs` | 159 | 任务 145 → app-provider-services |
| `src/app_providers/desktop_mapping.rs` | 60 | 任务 145 → app-provider-services |
| `src/app_providers/dsh.rs` | 69 | 任务 145 → app-provider-services |
| `src/app_providers/failover.rs` | 74 | 任务 145 → app-provider-services |
| `src/app_providers/failover_tests.rs` | 88 | 任务 145 → app-provider-services |
| `src/app_providers/gemini.rs` | 70 | 任务 145 → app-provider-services |
| `src/app_providers/generation_target.rs` | 131 | 任务 145 → app-provider-services |
| `src/app_providers/grok.rs` | 60 | 任务 145 → app-provider-services |
| `src/app_providers/hermes.rs` | 57 | 任务 145 → app-provider-services |
| `src/app_providers/import.rs` | 96 | 任务 145 → app-provider-services |
| `src/app_providers/kimi.rs` | 69 | 任务 146 → app-provider-services |
| `src/app_providers/kimi_binding.rs` | 198 | 任务 146 → app-provider-services |
| `src/app_providers/kimi_binding_tests.rs` | 340 | 任务 146 → app-provider-services |
| `src/app_providers/mcode.rs` | 57 | 任务 146 → app-provider-services |
| `src/app_providers/model_edits.rs` | 99 | 任务 146 → app-provider-services |
| `src/app_providers/model_edits_tests.rs` | 158 | 任务 146 → app-provider-services |
| `src/app_providers/official_launch.rs` | 57 | 任务 146 → app-provider-services |
| `src/app_providers/omp.rs` | 57 | 任务 146 → app-provider-services |
| `src/app_providers/openclaw.rs` | 60 | 任务 146 → app-provider-services |
| `src/app_providers/opencode.rs` | 80 | 任务 146 → app-provider-services |
| `src/app_providers/pi.rs` | 96 | 任务 146 → app-provider-services |
| `src/app_providers/record_publication.rs` | 209 | 任务 146 → app-provider-services |
| `src/app_providers/record_publication_tests.rs` | 259 | 任务 146 → app-provider-services |
| `src/app_providers/request_options.rs` | 46 | 任务 146 → app-provider-services |
| `src/app_providers/snippets.rs` | 50 | 任务 146 → app-provider-services |
| `src/app_providers/universal.rs` | 72 | 任务 146 → app-provider-services |
| `src/app_providers/universal_sync.rs` | 78 | 任务 146 → app-provider-services |
| `src/app_providers.rs` | 808 | 任务 144 → app-provider-services |
| `src/app_providers_tests.rs` | 116 | 任务 144 → app-provider-services |
| `src/app_routing/adapter_selection.rs` | 336 | 任务 147 → app-routing-services |
| `src/app_routing/adapter_selection_tests.rs` | 254 | 任务 147 → app-routing-services |
| `src/app_routing/aggregation_routing.rs` | 235 | 任务 147 → app-routing-services |
| `src/app_routing/aggregation_routing_tests.rs` | 452 | 任务 147 → app-routing-services |
| `src/app_routing/body_limits.rs` | 134 | 任务 147 → app-routing-services |
| `src/app_routing/body_limits_tests.rs` | 333 | 任务 147 → app-routing-services |
| `src/app_routing/browser_guard.rs` | 46 | 任务 147 → app-routing-services |
| `src/app_routing/browser_guard_tests.rs` | 182 | 任务 147 → app-routing-services |
| `src/app_routing/cancellation.rs` | 45 | 任务 147 → app-routing-services |
| `src/app_routing/cancellation_tests.rs` | 222 | 任务 147 → app-routing-services |
| `src/app_routing/capacity.rs` | 41 | 任务 147 → app-routing-services |
| `src/app_routing/capacity_tests.rs` | 264 | 任务 147 → app-routing-services |
| `src/app_routing/compaction_tests.rs` | 289 | 任务 147 → app-routing-services |
| `src/app_routing/config_apply.rs` | 163 | 任务 147 → app-routing-services |
| `src/app_routing/config_apply_tests.rs` | 152 | 任务 147 → app-routing-services |
| `src/app_routing/decode_request.rs` | 118 | 任务 147 → app-routing-services |
| `src/app_routing/decode_request_tests.rs` | 192 | 任务 147 → app-routing-services |
| `src/app_routing/decode_response.rs` | 19 | 任务 147 → app-routing-services |
| `src/app_routing/decode_response_tests.rs` | 242 | 任务 147 → app-routing-services |
| `src/app_routing/desktop_routing.rs` | 217 | 任务 147 → app-routing-services |
| `src/app_routing/dispatch.rs` | 1124 | 任务 148 → app-routing-services |
| `src/app_routing/dispatch_tests.rs` | 808 | 任务 148 → app-routing-services |
| `src/app_routing/failover_execution.rs` | 566 | 任务 148 → app-routing-services |
| `src/app_routing/failover_execution_tests.rs` | 122 | 任务 148 → app-routing-services |
| `src/app_routing/failover_routing.rs` | 104 | 任务 148 → app-routing-services |
| `src/app_routing/failover_routing_tests.rs` | 316 | 任务 148 → app-routing-services |
| `src/app_routing/gateway_owner.rs` | 18 | 任务 148 → app-routing-services |
| `src/app_routing/gateway_owner_tests.rs` | 261 | 任务 148 → app-routing-services |
| `src/app_routing/header_wire.rs` | 154 | 任务 148 → app-routing-services |
| `src/app_routing/header_wire_tests.rs` | 128 | 任务 148 → app-routing-services |
| `src/app_routing/listener.rs` | 699 | 任务 149 → app-routing-services |
| `src/app_routing/listener_tests.rs` | 165 | 任务 149 → app-routing-services |
| `src/app_routing/mod.rs` | 99 | 任务 150 → app-routing-services |
| `src/app_routing/outbound_config.rs` | 154 | 任务 149 → app-routing-services |
| `src/app_routing/outbound_config_tests.rs` | 114 | 任务 149 → app-routing-services |
| `src/app_routing/outbound_custom.rs` | 89 | 任务 149 → app-routing-services |
| `src/app_routing/outbound_custom_tests.rs` | 430 | 任务 149 → app-routing-services |
| `src/app_routing/outbound_system.rs` | 192 | 任务 149 → app-routing-services |
| `src/app_routing/outbound_system_tests.rs` | 233 | 任务 149 → app-routing-services |
| `src/app_routing/override_apply.rs` | 49 | 任务 149 → app-routing-services |
| `src/app_routing/override_apply_tests.rs` | 241 | 任务 149 → app-routing-services |
| `src/app_routing/protocol_errors.rs` | 227 | 任务 149 → app-routing-services |
| `src/app_routing/protocol_errors_tests.rs` | 273 | 任务 149 → app-routing-services |
| `src/app_routing/provider_publication.rs` | 448 | 任务 149 → app-routing-services |
| `src/app_routing/reference_scope.rs` | 111 | 任务 149 → app-routing-services |
| `src/app_routing/reference_scope_tests.rs` | 94 | 任务 149 → app-routing-services |
| `src/app_routing/request_status.rs` | 209 | 任务 149 → app-routing-services |
| `src/app_routing/route_auth.rs` | 926 | 任务 150 → app-routing-services |
| `src/app_routing/route_auth_tests.rs` | 266 | 任务 150 → app-routing-services |
| `src/app_routing/shared_transport.rs` | 456 | 任务 150 → app-routing-services |
| `src/app_routing/shared_transport_tests.rs` | 242 | 任务 150 → app-routing-services |
| `src/app_routing/shutdown.rs` | 36 | 任务 150 → app-routing-services |
| `src/app_routing/shutdown_tests.rs` | 162 | 任务 150 → app-routing-services |
| `src/app_routing/timeouts.rs` | 432 | 任务 150 → app-routing-services |
| `src/app_routing/timeouts_deadline_tests.rs` | 41 | 任务 150 → app-routing-services |
| `src/app_routing/timeouts_tests.rs` | 313 | 任务 150 → app-routing-services |
| `src/app_routing/transport_security.rs` | 102 | 任务 150 → app-routing-services |
| `src/app_routing/transport_security_tests.rs` | 237 | 任务 150 → app-routing-services |
| `src/app_tests.rs` | 265 | 任务 356 → agent-composition |
| `src/approval/cli.rs` | 54 | 任务 30 → agent-approval |
| `src/approval/cli_tests.rs` | 95 | 任务 30 → agent-approval |
| `src/approval/manager.rs` | 62 | 任务 30 → agent-approval |
| `src/approval/manager_tests.rs` | 16 | 任务 30 → agent-approval |
| `src/approval/mod.rs` | 91 | 任务 30 → agent-approval |
| `src/approval/pending.rs` | 240 | 任务 30 → agent-approval |
| `src/approval/pending_tests.rs` | 286 | 任务 30 → agent-approval |
| `src/approval/policy.rs` | 293 | 任务 30 → agent-approval |
| `src/approval/policy_tests.rs` | 196 | 任务 30 → agent-approval |
| `src/approval/summary.rs` | 31 | 任务 30 → agent-approval |
| `src/approval/summary_tests.rs` | 44 | 任务 30 → agent-approval |
| `src/approval/test_support.rs` | 13 | 任务 30 → agent-approval |
| `src/approval/tests.rs` | 891 | 任务 30 → agent-approval |
| `src/approval/types.rs` | 112 | 任务 30 → agent-approval |
| `src/approval/types_tests.rs` | 44 | 任务 30 → agent-approval |
| `src/auth/mod.rs` | 200 | 任务 31 → agent-auth |
| `src/auth/mod_tests.rs` | 65 | 任务 31 → agent-auth |
| `src/auth/openai_oauth.rs` | 404 | 任务 31 → agent-auth |
| `src/auth/openai_oauth_tests.rs` | 99 | 任务 31 → agent-auth |
| `src/bus/mod.rs` | 3 | 既有 agent-bus；保留薄导出 |
| `src/channels/channel_project_scope_tests.rs` | 59 | 任务 92 → channel-support |
| `src/channels/channel_session_lifecycle_tests.rs` | 178 | 任务 92 → channel-support |
| `src/channels/clawdtalk/mod.rs` | 598 | 任务 233 → channel-clawdtalk |
| `src/channels/clawdtalk/tests.rs` | 596 | 任务 233 → channel-clawdtalk |
| `src/channels/cli/mod.rs` | 326 | 任务 234 → channel-cli |
| `src/channels/cli/tests.rs` | 321 | 任务 234 → channel-cli |
| `src/channels/config.rs` | 618 | 任务 92 → channel-support |
| `src/channels/config_tests.rs` | 614 | 任务 92 → channel-support |
| `src/channels/dingtalk/mod.rs` | 558 | 任务 235 → channel-dingtalk |
| `src/channels/dingtalk/outbound.rs` | 119 | 任务 235 → channel-dingtalk |
| `src/channels/dingtalk/outbound_tests.rs` | 336 | 任务 235 → channel-dingtalk |
| `src/channels/dingtalk/tests.rs` | 490 | 任务 235 → channel-dingtalk |
| `src/channels/discord/attachments.rs` | 234 | 任务 236 → channel-discord |
| `src/channels/discord/attachments_outgoing.rs` | 388 | 任务 236 → channel-discord |
| `src/channels/discord/attachments_outgoing_tests.rs` | 203 | 任务 236 → channel-discord |
| `src/channels/discord/attachments_tests.rs` | 82 | 任务 236 → channel-discord |
| `src/channels/discord/client.rs` | 215 | 任务 236 → channel-discord |
| `src/channels/discord/client_tests.rs` | 248 | 任务 236 → channel-discord |
| `src/channels/discord/content.rs` | 197 | 任务 236 → channel-discord |
| `src/channels/discord/content_tests.rs` | 54 | 任务 236 → channel-discord |
| `src/channels/discord/gateway.rs` | 268 | 任务 236 → channel-discord |
| `src/channels/discord/gateway_tests.rs` | 244 | 任务 236 → channel-discord |
| `src/channels/discord/ids.rs` | 89 | 任务 236 → channel-discord |
| `src/channels/discord/ids_tests.rs` | 38 | 任务 236 → channel-discord |
| `src/channels/discord/message_split.rs` | 132 | 任务 236 → channel-discord |
| `src/channels/discord/message_split_tests.rs` | 59 | 任务 236 → channel-discord |
| `src/channels/discord/mod.rs` | 882 | 任务 237 → channel-discord |
| `src/channels/discord/mod_tests.rs` | 628 | 任务 236 → channel-discord |
| `src/channels/discord/permissions.rs` | 107 | 任务 236 → channel-discord |
| `src/channels/discord/permissions_tests.rs` | 30 | 任务 236 → channel-discord |
| `src/channels/discord/reactions.rs` | 177 | 任务 236 → channel-discord |
| `src/channels/discord/reactions_tests.rs` | 58 | 任务 236 → channel-discord |
| `src/channels/discord/tests/attachments_incoming.rs` | 100 | 任务 237 → channel-discord |
| `src/channels/discord/tests/attachments_outgoing.rs` | 106 | 任务 237 → channel-discord |
| `src/channels/discord/tests/basics.rs` | 211 | 任务 237 → channel-discord |
| `src/channels/discord/tests/message_ids.rs` | 53 | 任务 237 → channel-discord |
| `src/channels/discord/tests/message_split.rs` | 279 | 任务 237 → channel-discord |
| `src/channels/discord/tests/mod.rs` | 24 | 任务 237 → channel-discord |
| `src/channels/discord/tests/reactions.rs` | 55 | 任务 237 → channel-discord |
| `src/channels/discord/tests/typing.rs` | 59 | 任务 237 → channel-discord |
| `src/channels/discord/typing.rs` | 183 | 任务 237 → channel-discord |
| `src/channels/discord/typing_tests.rs` | 177 | 任务 237 → channel-discord |
| `src/channels/email_channel/mod.rs` | 888 | 任务 238 → channel-email-channel |
| `src/channels/email_channel/mod_tests.rs` | 319 | 任务 238 → channel-email-channel |
| `src/channels/email_channel/tests.rs` | 522 | 任务 238 → channel-email-channel |
| `src/channels/format.rs` | 961 | 任务 92 → channel-support |
| `src/channels/format_tests.rs` | 581 | 任务 92 → channel-support |
| `src/channels/history.rs` | 209 | 任务 92 → channel-support |
| `src/channels/history_tests.rs` | 208 | 任务 92 → channel-support |
| `src/channels/http.rs` | 25 | 任务 92 → channel-support |
| `src/channels/http_tests.rs` | 57 | 任务 92 → channel-support |
| `src/channels/imessage/mod.rs` | 687 | 任务 239 → channel-imessage |
| `src/channels/imessage/mod_tests.rs` | 200 | 任务 239 → channel-imessage |
| `src/channels/imessage/tests.rs` | 847 | 任务 239 → channel-imessage |
| `src/channels/irc/mod.rs` | 610 | 任务 240 → channel-irc |
| `src/channels/irc/mod_tests.rs` | 239 | 任务 240 → channel-irc |
| `src/channels/irc/tests.rs` | 570 | 任务 240 → channel-irc |
| `src/channels/lark/ack.rs` | 507 | 任务 241 → channel-lark |
| `src/channels/lark/ack_tests.rs` | 92 | 任务 241 → channel-lark |
| `src/channels/lark/constants.rs` | 196 | 任务 241 → channel-lark |
| `src/channels/lark/constants_tests.rs` | 17 | 任务 241 → channel-lark |
| `src/channels/lark/formatting.rs` | 100 | 任务 241 → channel-lark |
| `src/channels/lark/formatting_tests.rs` | 110 | 任务 241 → channel-lark |
| `src/channels/lark/mod.rs` | 594 | 任务 242 → channel-lark |
| `src/channels/lark/mod_tests.rs` | 133 | 任务 241 → channel-lark |
| `src/channels/lark/parsing.rs` | 702 | 任务 241 → channel-lark |
| `src/channels/lark/parsing_tests.rs` | 436 | 任务 241 → channel-lark |
| `src/channels/lark/platform.rs` | 144 | 任务 241 → channel-lark |
| `src/channels/lark/platform_tests.rs` | 30 | 任务 241 → channel-lark |
| `src/channels/lark/tests/ack_tests.rs` | 119 | 任务 241 → channel-lark |
| `src/channels/lark/tests/config_tests.rs` | 143 | 任务 241 → channel-lark |
| `src/channels/lark/tests/parsing_tests.rs` | 435 | 任务 241 → channel-lark |
| `src/channels/lark/tests/token_tests.rs` | 110 | 任务 241 → channel-lark |
| `src/channels/lark/tests/ws_tests.rs` | 27 | 任务 241 → channel-lark |
| `src/channels/lark/tests.rs` | 115 | 任务 241 → channel-lark |
| `src/channels/lark/token.rs` | 663 | 任务 242 → channel-lark |
| `src/channels/lark/token_bot_tests.rs` | 153 | 任务 242 → channel-lark |
| `src/channels/lark/token_http_tests.rs` | 597 | 任务 242 → channel-lark |
| `src/channels/lark/token_test_support.rs` | 187 | 任务 242 → channel-lark |
| `src/channels/lark/token_tests.rs` | 153 | 任务 242 → channel-lark |
| `src/channels/lark/types.rs` | 199 | 任务 242 → channel-lark |
| `src/channels/lark/types_tests.rs` | 100 | 任务 242 → channel-lark |
| `src/channels/lark/webhook.rs` | 202 | 任务 242 → channel-lark |
| `src/channels/lark/webhook_tests.rs` | 171 | 任务 242 → channel-lark |
| `src/channels/lark/ws.rs` | 504 | 任务 242 → channel-lark |
| `src/channels/lark/ws_test_support.rs` | 256 | 任务 242 → channel-lark |
| `src/channels/lark/ws_tests.rs` | 578 | 任务 242 → channel-lark |
| `src/channels/linq/mod.rs` | 650 | 任务 243 → channel-linq |
| `src/channels/linq/tests.rs` | 654 | 任务 243 → channel-linq |
| `src/channels/manager/channels_config.rs` | 600 | 任务 248 → channel-runtime |
| `src/channels/manager/channels_config_tests.rs` | 516 | 任务 248 → channel-runtime |
| `src/channels/manager/command.rs` | 104 | 任务 248 → channel-runtime |
| `src/channels/manager/command_flow_tests.rs` | 72 | 任务 248 → channel-runtime |
| `src/channels/manager/command_tests.rs` | 130 | 任务 248 → channel-runtime |
| `src/channels/manager/dispatch.rs` | 70 | 任务 248 → channel-runtime |
| `src/channels/manager/dispatch_cancel_tests.rs` | 358 | 任务 248 → channel-runtime |
| `src/channels/manager/dispatch_tests.rs` | 356 | 任务 248 → channel-runtime |
| `src/channels/manager/errors.rs` | 98 | 任务 248 → channel-runtime |
| `src/channels/manager/errors_tests.rs` | 138 | 任务 248 → channel-runtime |
| `src/channels/manager/execution_binding.rs` | 164 | 任务 248 → channel-runtime |
| `src/channels/manager/execution_binding_tests.rs` | 156 | 任务 248 → channel-runtime |
| `src/channels/manager/execution_control.rs` | 149 | 任务 248 → channel-runtime |
| `src/channels/manager/execution_control_tests.rs` | 99 | 任务 248 → channel-runtime |
| `src/channels/manager/health.rs` | 112 | 任务 248 → channel-runtime |
| `src/channels/manager/health_tests.rs` | 140 | 任务 248 → channel-runtime |
| `src/channels/manager/listener.rs` | 87 | 任务 248 → channel-runtime |
| `src/channels/manager/listener_tests.rs` | 241 | 任务 248 → channel-runtime |
| `src/channels/manager/message.rs` | 368 | 任务 248 → channel-runtime |
| `src/channels/manager/message_execution.rs` | 219 | 任务 248 → channel-runtime |
| `src/channels/manager/message_execution_tests.rs` | 362 | 任务 248 → channel-runtime |
| `src/channels/manager/message_helpers.rs` | 439 | 任务 249 → channel-runtime |
| `src/channels/manager/message_helpers_tests.rs` | 540 | 任务 249 → channel-runtime |
| `src/channels/manager/message_response.rs` | 661 | 任务 249 → channel-runtime |
| `src/channels/manager/message_response_tests.rs` | 989 | 任务 249 → channel-runtime |
| `src/channels/manager/message_result.rs` | 86 | 任务 249 → channel-runtime |
| `src/channels/manager/message_result_tests.rs` | 30 | 任务 249 → channel-runtime |
| `src/channels/manager/message_tests.rs` | 367 | 任务 249 → channel-runtime |
| `src/channels/manager/mod.rs` | 44 | 任务 249 → channel-runtime |
| `src/channels/manager/provider.rs` | 172 | 任务 249 → channel-runtime |
| `src/channels/manager/provider_tests.rs` | 262 | 任务 249 → channel-runtime |
| `src/channels/manager/reload.rs` | 96 | 任务 249 → channel-runtime |
| `src/channels/manager/reload_tests.rs` | 224 | 任务 249 → channel-runtime |
| `src/channels/manager/start.rs` | 496 | 任务 249 → channel-runtime |
| `src/channels/manager/start_tests.rs` | 326 | 任务 249 → channel-runtime |
| `src/channels/manager/telegram.rs` | 76 | 任务 249 → channel-runtime |
| `src/channels/manager/telegram_tests.rs` | 139 | 任务 249 → channel-runtime |
| `src/channels/manager/tests.rs` | 8 | 任务 249 → channel-runtime |
| `src/channels/manager/typing.rs` | 87 | 任务 249 → channel-runtime |
| `src/channels/manager/typing_tests.rs` | 123 | 任务 249 → channel-runtime |
| `src/channels/markdown_chunk.rs` | 245 | 任务 92 → channel-support |
| `src/channels/markdown_chunk_tests.rs` | 137 | 任务 92 → channel-support |
| `src/channels/mattermost/mod.rs` | 827 | 任务 244 → channel-mattermost |
| `src/channels/mattermost/mod_tests.rs` | 310 | 任务 244 → channel-mattermost |
| `src/channels/mattermost/tests.rs` | 577 | 任务 244 → channel-mattermost |
| `src/channels/mod.rs` | 731 | 任务 253 → channel-runtime |
| `src/channels/mod_tests.rs` | 131 | 任务 250 → channel-runtime |
| `src/channels/nextcloud_talk/mod.rs` | 698 | 任务 245 → channel-nextcloud-talk |
| `src/channels/nextcloud_talk/mod_tests.rs` | 280 | 任务 245 → channel-nextcloud-talk |
| `src/channels/nextcloud_talk/tests.rs` | 377 | 任务 245 → channel-nextcloud-talk |
| `src/channels/nostr/mod.rs` | 609 | 任务 246 → channel-nostr |
| `src/channels/nostr/tests.rs` | 257 | 任务 246 → channel-nostr |
| `src/channels/outbound_delivery.rs` | 37 | 任务 92 → channel-support |
| `src/channels/outbound_delivery_tests.rs` | 25 | 任务 92 → channel-support |
| `src/channels/outbound_format.rs` | 207 | 任务 93 → channel-support |
| `src/channels/outbound_format_tests.rs` | 48 | 任务 93 → channel-support |
| `src/channels/prompt.rs` | 768 | 任务 93 → channel-support |
| `src/channels/prompt_tests.rs` | 580 | 任务 93 → channel-support |
| `src/channels/qq/auth.rs` | 106 | 任务 247 → channel-qq |
| `src/channels/qq/auth_tests.rs` | 311 | 任务 247 → channel-qq |
| `src/channels/qq/channel_impl.rs` | 40 | 任务 247 → channel-qq |
| `src/channels/qq/channel_impl_tests.rs` | 90 | 任务 247 → channel-qq |
| `src/channels/qq/content.rs` | 214 | 任务 247 → channel-qq |
| `src/channels/qq/content_tests.rs` | 177 | 任务 247 → channel-qq |
| `src/channels/qq/gateway.rs` | 188 | 任务 247 → channel-qq |
| `src/channels/qq/gateway_tests.rs` | 436 | 任务 247 → channel-qq |
| `src/channels/qq/mod.rs` | 252 | 任务 247 → channel-qq |
| `src/channels/qq/outbound.rs` | 324 | 任务 247 → channel-qq |
| `src/channels/qq/outbound_tests.rs` | 1012 | 任务 247 → channel-qq |
| `src/channels/qq/tests.rs` | 491 | 任务 247 → channel-qq |
| `src/channels/qq/webhook.rs` | 149 | 任务 247 → channel-qq |
| `src/channels/qq/webhook_tests.rs` | 300 | 任务 247 → channel-qq |
| `src/channels/routing.rs` | 437 | 任务 93 → channel-support |
| `src/channels/routing_tests.rs` | 267 | 任务 93 → channel-support |
| `src/channels/runtime_command/approval_commands.rs` | 577 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/approval_commands_tests.rs` | 669 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/approval_config.rs` | 327 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/approval_config_tests.rs` | 532 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/catalog.rs` | 420 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/command.rs` | 297 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/command_tests.rs` | 445 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/handler.rs` | 282 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/handler_tests.rs` | 620 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/history_commands.rs` | 209 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/history_commands_tests.rs` | 352 | 任务 251 → channel-runtime |
| `src/channels/runtime_command/project_commands.rs` | 89 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/project_commands_tests.rs` | 267 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/session_commands.rs` | 368 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/session_commands_tests.rs` | 786 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/session_lifecycle.rs` | 342 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/task_mode.rs` | 76 | 任务 252 → channel-runtime |
| `src/channels/runtime_command/task_mode_tests.rs` | 295 | 任务 252 → channel-runtime |
| `src/channels/runtime_command.rs` | 42 | 任务 253 → channel-runtime |
| `src/channels/runtime_command_tests.rs` | 8 | 任务 253 → channel-runtime |
| `src/channels/scope.rs` | 197 | 任务 93 → channel-support |
| `src/channels/scope_delivery_tests.rs` | 44 | 任务 93 → channel-support |
| `src/channels/session.rs` | 417 | 任务 93 → channel-support |
| `src/channels/session_tests.rs` | 473 | 任务 93 → channel-support |
| `src/channels/session_turn.rs` | 319 | 任务 94 → channel-support |
| `src/channels/signal/mod.rs` | 755 | 任务 254 → channel-signal |
| `src/channels/signal/tests.rs` | 966 | 任务 254 → channel-signal |
| `src/channels/slack/mod.rs` | 460 | 任务 255 → channel-slack |
| `src/channels/slack/tests.rs` | 369 | 任务 255 → channel-slack |
| `src/channels/telegram/attachments.rs` | 676 | 任务 256 → channel-telegram |
| `src/channels/telegram/attachments_incoming.rs` | 353 | 任务 256 → channel-telegram |
| `src/channels/telegram/attachments_incoming_tests.rs` | 466 | 任务 256 → channel-telegram |
| `src/channels/telegram/attachments_outbound.rs` | 102 | 任务 256 → channel-telegram |
| `src/channels/telegram/attachments_outbound_tests.rs` | 230 | 任务 256 → channel-telegram |
| `src/channels/telegram/attachments_tests.rs` | 299 | 任务 256 → channel-telegram |
| `src/channels/telegram/callbacks.rs` | 265 | 任务 256 → channel-telegram |
| `src/channels/telegram/callbacks_tests.rs` | 264 | 任务 256 → channel-telegram |
| `src/channels/telegram/channel_impl.rs` | 424 | 任务 256 → channel-telegram |
| `src/channels/telegram/channel_impl_tests.rs` | 302 | 任务 256 → channel-telegram |
| `src/channels/telegram/config.rs` | 487 | 任务 257 → channel-telegram |
| `src/channels/telegram/config_tests.rs` | 399 | 任务 257 → channel-telegram |
| `src/channels/telegram/core.rs` | 120 | 任务 257 → channel-telegram |
| `src/channels/telegram/core_tests.rs` | 98 | 任务 257 → channel-telegram |
| `src/channels/telegram/draft.rs` | 404 | 任务 257 → channel-telegram |
| `src/channels/telegram/draft_tests.rs` | 387 | 任务 257 → channel-telegram |
| `src/channels/telegram/file_io.rs` | 70 | 任务 257 → channel-telegram |
| `src/channels/telegram/file_io_tests.rs` | 177 | 任务 257 → channel-telegram |
| `src/channels/telegram/formatting.rs` | 172 | 任务 257 → channel-telegram |
| `src/channels/telegram/formatting_tests.rs` | 121 | 任务 257 → channel-telegram |
| `src/channels/telegram/incoming.rs` | 234 | 任务 257 → channel-telegram |
| `src/channels/telegram/incoming_tests.rs` | 322 | 任务 257 → channel-telegram |
| `src/channels/telegram/mentions.rs` | 119 | 任务 257 → channel-telegram |
| `src/channels/telegram/mentions_tests.rs` | 81 | 任务 257 → channel-telegram |
| `src/channels/telegram/message_utils.rs` | 223 | 任务 257 → channel-telegram |
| `src/channels/telegram/message_utils_tests.rs` | 103 | 任务 257 → channel-telegram |
| `src/channels/telegram/mod.rs` | 250 | 任务 261 → channel-telegram |
| `src/channels/telegram/outbound.rs` | 206 | 任务 257 → channel-telegram |
| `src/channels/telegram/outbound_tests.rs` | 394 | 任务 257 → channel-telegram |
| `src/channels/telegram/pairing.rs` | 16 | 任务 257 → channel-telegram |
| `src/channels/telegram/pairing_tests.rs` | 11 | 任务 257 → channel-telegram |
| `src/channels/telegram/polling.rs` | 247 | 任务 258 → channel-telegram |
| `src/channels/telegram/polling_tests.rs` | 672 | 任务 258 → channel-telegram |
| `src/channels/telegram/reactions.rs` | 106 | 任务 258 → channel-telegram |
| `src/channels/telegram/reactions_tests.rs` | 135 | 任务 258 → channel-telegram |
| `src/channels/telegram/sending_media.rs` | 639 | 任务 258 → channel-telegram |
| `src/channels/telegram/sending_media_tests.rs` | 1134 | 任务 258 → channel-telegram |
| `src/channels/telegram/sending_text.rs` | 115 | 任务 258 → channel-telegram |
| `src/channels/telegram/sending_text_tests.rs` | 237 | 任务 258 → channel-telegram |
| `src/channels/telegram/tests/ack_reaction.rs` | 55 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/approval_callback.rs` | 130 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/attachment_format.rs` | 483 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/attachments.rs` | 328 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/channel_basic.rs` | 270 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/file_sending.rs` | 333 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/helpers.rs` | 58 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/markdown_html.rs` | 110 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/mention_handling.rs` | 370 | 任务 259 → channel-telegram |
| `src/channels/telegram/tests/message_parsing.rs` | 233 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/message_splitting.rs` | 180 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/mod.rs` | 51 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/pairing_bind.rs` | 82 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/streaming_drafts.rs` | 115 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/tool_call_tags.rs` | 388 | 任务 260 → channel-telegram |
| `src/channels/telegram/tests/user_allowlist.rs` | 129 | 任务 260 → channel-telegram |
| `src/channels/telegram/tool_tags.rs` | 142 | 任务 261 → channel-telegram |
| `src/channels/telegram/tool_tags_tests.rs` | 96 | 任务 261 → channel-telegram |
| `src/channels/tests/approval_commands.rs` | 35 | 任务 262 → channel-tests |
| `src/channels/tests/command_parsing.rs` | 184 | 任务 262 → channel-tests |
| `src/channels/tests/config_loading.rs` | 32 | 任务 262 → channel-tests |
| `src/channels/tests/health_supervisor.rs` | 128 | 任务 262 → channel-tests |
| `src/channels/tests/history_management.rs` | 272 | 任务 262 → channel-tests |
| `src/channels/tests/identity.rs` | 19 | 任务 262 → channel-tests |
| `src/channels/tests/message_dispatch.rs` | 254 | 任务 262 → channel-tests |
| `src/channels/tests/mod.rs` | 90 | 任务 262 → channel-tests |
| `src/channels/tests/routing.rs` | 13 | 任务 262 → channel-tests |
| `src/channels/tests/session_history.rs` | 23 | 任务 262 → channel-tests |
| `src/channels/tests/system_prompt.rs` | 271 | 任务 262 → channel-tests |
| `src/channels/tests/timeout_budget.rs` | 114 | 任务 262 → channel-tests |
| `src/channels/tests/tool_execution.rs` | 111 | 任务 262 → channel-tests |
| `src/channels/tests/util.rs` | 21 | 任务 262 → channel-tests |
| `src/channels/tests/vision.rs` | 13 | 任务 262 → channel-tests |
| `src/channels/traits/mod.rs` | 727 | 任务 26 → channel-types |
| `src/channels/traits/mod_tests.rs` | 281 | 任务 26 → channel-types |
| `src/channels/wati/mod.rs` | 246 | 任务 263 → channel-wati |
| `src/channels/wati/tests.rs` | 671 | 任务 263 → channel-wati |
| `src/channels/wechat/mod.rs` | 285 | 任务 264 → channel-wechat |
| `src/channels/wechat/protocol.rs` | 27 | 任务 264 → channel-wechat |
| `src/channels/wechat/tests.rs` | 455 | 任务 264 → channel-wechat |
| `src/channels/wecom/mod.rs` | 285 | 任务 265 → channel-wecom |
| `src/channels/wecom/outbound.rs` | 230 | 任务 265 → channel-wecom |
| `src/channels/wecom/outbound_tests.rs` | 833 | 任务 265 → channel-wecom |
| `src/channels/wecom/runtime.rs` | 140 | 任务 265 → channel-wecom |
| `src/channels/wecom/runtime_tests.rs` | 200 | 任务 265 → channel-wecom |
| `src/channels/wecom/tests.rs` | 209 | 任务 265 → channel-wecom |
| `src/channels/wecom/webhook.rs` | 160 | 任务 265 → channel-wecom |
| `src/channels/wecom/webhook_tests.rs` | 253 | 任务 265 → channel-wecom |
| `src/channels/whatsapp/mod.rs` | 272 | 任务 266 → channel-whatsapp |
| `src/channels/whatsapp/mod_tests.rs` | 1228 | 任务 266 → channel-whatsapp |
| `src/channels/whatsapp_storage/app_sync_store.rs` | 157 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/app_sync_store_tests.rs` | 182 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/device_store.rs` | 213 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/device_store_tests.rs` | 295 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/mod.rs` | 115 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/mod_tests.rs` | 123 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/protocol_store.rs` | 717 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/protocol_store_tests.rs` | 562 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/schema.rs` | 330 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/schema_tests.rs` | 81 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/signal_store.rs` | 240 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/signal_store_tests.rs` | 190 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_storage/tests.rs` | 81 | 任务 267 → channel-whatsapp-storage |
| `src/channels/whatsapp_web/mod.rs` | 760 | 任务 268 → channel-whatsapp-web |
| `src/channels/whatsapp_web/mod_tests.rs` | 219 | 任务 268 → channel-whatsapp-web |
| `src/cli_commands.rs` | 943 | 任务 32 → agent-cli-types |
| `src/cli_commands_tests.rs` | 43 | 任务 32 → agent-cli-types |
| `src/command/command.rs` | 626 | 任务 68 → agent-commands |
| `src/command/command_tests.rs` | 266 | 任务 68 → agent-commands |
| `src/command/mod.rs` | 67 | 任务 68 → agent-commands |
| `src/command/template/initialize.txt` | 10 | 任务 68 → agent-commands |
| `src/command/template/review.txt` | 101 | 任务 68 → agent-commands |
| `src/computer_control/chrome/bridge.rs` | 367 | 任务 60 → computer-control |
| `src/computer_control/chrome/bridge_tests.rs` | 325 | 任务 60 → computer-control |
| `src/computer_control/chrome/mod.rs` | 7 | 任务 60 → computer-control |
| `src/computer_control/chrome/origin_policy.rs` | 49 | 任务 60 → computer-control |
| `src/computer_control/chrome/origin_policy_tests.rs` | 305 | 任务 60 → computer-control |
| `src/computer_control/host.rs` | 576 | 任务 60 → computer-control |
| `src/computer_control/host_tests.rs` | 333 | 任务 60 → computer-control |
| `src/computer_control/lifecycle_tests.rs` | 82 | 任务 60 → computer-control |
| `src/computer_control/mod.rs` | 19 | 任务 62 → computer-control |
| `src/computer_control/native/input_tests.rs` | 321 | 任务 61 → computer-control |
| `src/computer_control/native/locked_ax.rs` | 180 | 任务 61 → computer-control |
| `src/computer_control/native/locked_session.rs` | 133 | 任务 61 → computer-control |
| `src/computer_control/native/locked_session_tests.rs` | 47 | 任务 61 → computer-control |
| `src/computer_control/native/macos_input.rs` | 450 | 任务 61 → computer-control |
| `src/computer_control/native/macos_observation.rs` | 521 | 任务 61 → computer-control |
| `src/computer_control/native/mod.rs` | 421 | 任务 61 → computer-control |
| `src/computer_control/native/observation_tests.rs` | 328 | 任务 61 → computer-control |
| `src/computer_control/office/mod.rs` | 5 | 任务 62 → computer-control |
| `src/computer_control/office/transport.rs` | 543 | 任务 62 → computer-control |
| `src/computer_control/office/transport_tests.rs` | 319 | 任务 62 → computer-control |
| `src/computer_control/policy.rs` | 192 | 任务 62 → computer-control |
| `src/computer_control/policy_tests.rs` | 136 | 任务 62 → computer-control |
| `src/config/migration.rs` | 272 | 任务 33 → agent-config |
| `src/config/migration_tests.rs` | 197 | 任务 33 → agent-config |
| `src/config/mod.rs` | 826 | 任务 37 → agent-config |
| `src/config/mod_tests.rs` | 272 | 任务 33 → agent-config |
| `src/config/runtime_home.rs` | 9 | 任务 33 → agent-config |
| `src/config/runtime_home_tests.rs` | 28 | 任务 33 → agent-config |
| `src/config/schema/automation.rs` | 7 | 任务 34 → agent-config |
| `src/config/schema/channel_reload.rs` | 123 | 任务 34 → agent-config |
| `src/config/schema/channel_reload_tests.rs` | 127 | 任务 34 → agent-config |
| `src/config/schema/channels/chat.rs` | 199 | 任务 34 → agent-config |
| `src/config/schema/channels/chat_tests.rs` | 137 | 任务 34 → agent-config |
| `src/config/schema/channels/email.rs` | 18 | 任务 34 → agent-config |
| `src/config/schema/channels/email_tests.rs` | 22 | 任务 34 → agent-config |
| `src/config/schema/channels/helpers.rs` | 221 | 任务 34 → agent-config |
| `src/config/schema/channels/helpers_tests.rs` | 118 | 任务 34 → agent-config |
| `src/config/schema/channels/lark.rs` | 28 | 任务 34 → agent-config |
| `src/config/schema/channels/lark_tests.rs` | 33 | 任务 34 → agent-config |
| `src/config/schema/channels/mod.rs` | 80 | 任务 34 → agent-config |
| `src/config/schema/channels/other.rs` | 98 | 任务 34 → agent-config |
| `src/config/schema/channels/other_tests.rs` | 52 | 任务 34 → agent-config |
| `src/config/schema/channels/secrets.rs` | 542 | 任务 34 → agent-config |
| `src/config/schema/channels/secrets_isolation.rs` | 86 | 任务 34 → agent-config |
| `src/config/schema/channels/secrets_tests.rs` | 396 | 任务 34 → agent-config |
| `src/config/schema/channels/telegram_env.rs` | 107 | 任务 34 → agent-config |
| `src/config/schema/channels/telegram_env_tests.rs` | 276 | 任务 34 → agent-config |
| `src/config/schema/channels/types.rs` | 68 | 任务 34 → agent-config |
| `src/config/schema/channels/types_tests.rs` | 30 | 任务 34 → agent-config |
| `src/config/schema/config.rs` | 133 | 任务 34 → agent-config |
| `src/config/schema/config_env.rs` | 350 | 任务 34 → agent-config |
| `src/config/schema/config_env_tests.rs` | 512 | 任务 34 → agent-config |
| `src/config/schema/config_helpers.rs` | 146 | 任务 35 → agent-config |
| `src/config/schema/config_helpers_tests.rs` | 133 | 任务 35 → agent-config |
| `src/config/schema/config_io.rs` | 198 | 任务 35 → agent-config |
| `src/config/schema/config_io_tests.rs` | 283 | 任务 35 → agent-config |
| `src/config/schema/config_load.rs` | 425 | 任务 35 → agent-config |
| `src/config/schema/config_load_tests.rs` | 592 | 任务 35 → agent-config |
| `src/config/schema/config_save.rs` | 318 | 任务 35 → agent-config |
| `src/config/schema/config_save_tests.rs` | 396 | 任务 35 → agent-config |
| `src/config/schema/config_secrets.rs` | 188 | 任务 35 → agent-config |
| `src/config/schema/config_secrets_tests.rs` | 230 | 任务 35 → agent-config |
| `src/config/schema/config_tests.rs` | 294 | 任务 35 → agent-config |
| `src/config/schema/config_validate.rs` | 434 | 任务 35 → agent-config |
| `src/config/schema/config_validate_tests.rs` | 576 | 任务 35 → agent-config |
| `src/config/schema/mod.rs` | 103 | 任务 36 → agent-config |
| `src/config/schema/provider.rs` | 7 | 任务 35 → agent-config |
| `src/config/schema/proxy.rs` | 378 | 任务 35 → agent-config |
| `src/config/schema/proxy_tests.rs` | 429 | 任务 35 → agent-config |
| `src/config/schema/security.rs` | 6 | 任务 35 → agent-config |
| `src/config/schema/skills.rs` | 6 | 任务 35 → agent-config |
| `src/config/schema/workspace.rs` | 657 | 任务 36 → agent-config |
| `src/config/schema/workspace_tests.rs` | 442 | 任务 36 → agent-config |
| `src/config/store/postgres.rs` | 185 | 任务 37 → agent-config |
| `src/config/store/postgres_tests.rs` | 28 | 任务 37 → agent-config |
| `src/config/store.rs` | 245 | 任务 37 → agent-config |
| `src/config/store_tests.rs` | 165 | 任务 37 → agent-config |
| `src/config/tests.rs` | 153 | 任务 37 → agent-config |
| `src/config/traits.rs` | 21 | 任务 37 → agent-config |
| `src/config/traits_tests.rs` | 36 | 任务 37 → agent-config |
| `src/cron/active_runs.rs` | 76 | 任务 136 → agent-cron |
| `src/cron/active_runs_tests.rs` | 110 | 任务 136 → agent-cron |
| `src/cron/consolidation/mod.rs` | 86 | 任务 136 → agent-cron |
| `src/cron/consolidation/tests.rs` | 156 | 任务 136 → agent-cron |
| `src/cron/delivery.rs` | 195 | 任务 136 → agent-cron |
| `src/cron/delivery_tests.rs` | 214 | 任务 136 → agent-cron |
| `src/cron/mod.rs` | 284 | 任务 141 → agent-cron |
| `src/cron/mod_tests.rs` | 121 | 任务 136 → agent-cron |
| `src/cron/schedule/biweekly.rs` | 49 | 任务 136 → agent-cron |
| `src/cron/schedule/biweekly_tests.rs` | 119 | 任务 136 → agent-cron |
| `src/cron/schedule/mod.rs` | 265 | 任务 136 → agent-cron |
| `src/cron/schedule/tests.rs` | 227 | 任务 136 → agent-cron |
| `src/cron/scheduler.rs` | 1359 | 任务 136 → agent-cron |
| `src/cron/scheduler_tests.rs` | 68 | 任务 136 → agent-cron |
| `src/cron/store.rs` | 951 | 任务 137 → agent-cron |
| `src/cron/store_tests.rs` | 583 | 任务 137 → agent-cron |
| `src/cron/tests/commands.rs` | 298 | 任务 138 → agent-cron |
| `src/cron/tests/consolidation.rs` | 185 | 任务 138 → agent-cron |
| `src/cron/tests/mod.rs` | 11 | 任务 140 → agent-cron |
| `src/cron/tests/schedule.rs` | 194 | 任务 138 → agent-cron |
| `src/cron/tests/scheduler.rs` | 1795 | 任务 139 → agent-cron |
| `src/cron/tests/store.rs` | 480 | 任务 140 → agent-cron |
| `src/cron/tests/types.rs` | 43 | 任务 140 → agent-cron |
| `src/cron/tests.rs` | 485 | 任务 141 → agent-cron |
| `src/cron/types/mod.rs` | 514 | 任务 141 → agent-cron |
| `src/cron/types/tests.rs` | 227 | 任务 141 → agent-cron |
| `src/cron/wecom_acceptance_tests.rs` | 163 | 任务 141 → agent-cron |
| `src/daemon/mod.rs` | 797 | 任务 350 → agent-daemon |
| `src/daemon/tests.rs` | 726 | 任务 350 → agent-daemon |
| `src/device_sync/apply.rs` | 168 | 任务 95 → device-sync |
| `src/device_sync/auto_sync.rs` | 96 | 任务 95 → device-sync |
| `src/device_sync/cancel.rs` | 20 | 任务 95 → device-sync |
| `src/device_sync/change_events.rs` | 101 | 任务 95 → device-sync |
| `src/device_sync/export.rs` | 31 | 任务 95 → device-sync |
| `src/device_sync/import.rs` | 4 | 任务 95 → device-sync |
| `src/device_sync/lifecycle.rs` | 85 | 任务 95 → device-sync |
| `src/device_sync/mod.rs` | 46 | 任务 96 → device-sync |
| `src/device_sync/operation_lock.rs` | 106 | 任务 95 → device-sync |
| `src/device_sync/projection.rs` | 153 | 任务 95 → device-sync |
| `src/device_sync/s3_download.rs` | 123 | 任务 95 → device-sync |
| `src/device_sync/s3_settings.rs` | 264 | 任务 95 → device-sync |
| `src/device_sync/s3_upload.rs` | 112 | 任务 95 → device-sync |
| `src/device_sync/skill_receive.rs` | 51 | 任务 95 → device-sync |
| `src/device_sync/status.rs` | 173 | 任务 95 → device-sync |
| `src/device_sync/tasks.rs` | 255 | 任务 95 → device-sync |
| `src/device_sync/webdav_download.rs` | 112 | 任务 95 → device-sync |
| `src/device_sync/webdav_settings.rs` | 242 | 任务 95 → device-sync |
| `src/device_sync/webdav_upload.rs` | 111 | 任务 95 → device-sync |
| `src/doctor/config_checks.rs` | 220 | 任务 351 → agent-doctor |
| `src/doctor/config_checks_tests.rs` | 270 | 任务 351 → agent-doctor |
| `src/doctor/daemon_checks.rs` | 127 | 任务 351 → agent-doctor |
| `src/doctor/daemon_checks_tests.rs` | 405 | 任务 351 → agent-doctor |
| `src/doctor/environment_checks.rs` | 75 | 任务 351 → agent-doctor |
| `src/doctor/environment_checks_tests.rs` | 142 | 任务 351 → agent-doctor |
| `src/doctor/mod.rs` | 186 | 任务 351 → agent-doctor |
| `src/doctor/mod_tests.rs` | 114 | 任务 351 → agent-doctor |
| `src/doctor/model_probe.rs` | 125 | 任务 351 → agent-doctor |
| `src/doctor/model_probe_tests.rs` | 130 | 任务 351 → agent-doctor |
| `src/doctor/tests.rs` | 521 | 任务 351 → agent-doctor |
| `src/doctor/traces.rs` | 104 | 任务 351 → agent-doctor |
| `src/doctor/traces_tests.rs` | 123 | 任务 351 → agent-doctor |
| `src/doctor/utils.rs` | 31 | 任务 351 → agent-doctor |
| `src/doctor/utils_tests.rs` | 19 | 任务 351 → agent-doctor |
| `src/doctor/workspace_checks.rs` | 152 | 任务 351 → agent-doctor |
| `src/doctor/workspace_checks_tests.rs` | 197 | 任务 351 → agent-doctor |
| `src/env/mod.rs` | 121 | 任务 38 → agent-env |
| `src/env/tests.rs` | 91 | 任务 38 → agent-env |
| `src/file/event.rs` | 11 | 任务 39 → agent-files |
| `src/file/event_tests.rs` | 9 | 任务 39 → agent-files |
| `src/file/fs_ops.rs` | 134 | 任务 39 → agent-files |
| `src/file/fs_ops_tests.rs` | 344 | 任务 39 → agent-files |
| `src/file/git_status.rs` | 134 | 任务 39 → agent-files |
| `src/file/git_status_tests.rs` | 223 | 任务 39 → agent-files |
| `src/file/ignore.rs` | 131 | 任务 39 → agent-files |
| `src/file/ignore_tests.rs` | 54 | 任务 39 → agent-files |
| `src/file/mod.rs` | 52 | 任务 39 → agent-files |
| `src/file/pathing.rs` | 170 | 任务 39 → agent-files |
| `src/file/pathing_tests.rs` | 206 | 任务 39 → agent-files |
| `src/file/ripgrep.rs` | 260 | 任务 39 → agent-files |
| `src/file/ripgrep_tests.rs` | 322 | 任务 39 → agent-files |
| `src/file/search.rs` | 129 | 任务 39 → agent-files |
| `src/file/search_tests.rs` | 250 | 任务 39 → agent-files |
| `src/file/tests.rs` | 30 | 任务 39 → agent-files |
| `src/file/text.rs` | 94 | 任务 39 → agent-files |
| `src/file/text_tests.rs` | 89 | 任务 39 → agent-files |
| `src/file/time.rs` | 208 | 任务 39 → agent-files |
| `src/file/time_tests.rs` | 388 | 任务 39 → agent-files |
| `src/file/types.rs` | 101 | 任务 39 → agent-files |
| `src/file/types_tests.rs` | 216 | 任务 39 → agent-files |
| `src/file/watcher.rs` | 473 | 任务 39 → agent-files |
| `src/file/watcher_tests.rs` | 274 | 任务 39 → agent-files |
| `src/flag/mod.rs` | 3 | 既有 runtime-flags；保留薄导出 |
| `src/format/format/builtins.rs` | 246 | 任务 40 → agent-format |
| `src/format/format/builtins_tests.rs` | 128 | 任务 40 → agent-format |
| `src/format/format/detect.rs` | 359 | 任务 40 → agent-format |
| `src/format/format/detect_tests.rs` | 667 | 任务 40 → agent-format |
| `src/format/format/runtime.rs` | 167 | 任务 40 → agent-format |
| `src/format/format/runtime_tests.rs` | 393 | 任务 40 → agent-format |
| `src/format/format/state.rs` | 80 | 任务 40 → agent-format |
| `src/format/format/state_tests.rs` | 171 | 任务 40 → agent-format |
| `src/format/format.rs` | 21 | 任务 40 → agent-format |
| `src/format/mod.rs` | 20 | 任务 40 → agent-format |
| `src/gateway/agent.rs` | 255 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps/mod.rs` | 6 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps/profile_operations.rs` | 133 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps/profile_skills.rs` | 62 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps/profile_store.rs` | 453 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/mod.rs` | 4 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/model_check/response.rs` | 146 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/model_check/target.rs` | 345 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/model_check.rs` | 225 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/model_check_tests.rs` | 601 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/reachability.rs` | 310 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/reachability_tests.rs` | 315 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/speedtest.rs` | 144 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/speedtest_tests.rs` | 265 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/stream_check.rs` | 239 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_executors/stream_check_tests.rs` | 323 | 任务 272 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_jobs/endpoints.rs` | 70 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_jobs/endpoints_tests.rs` | 82 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_jobs.rs` | 554 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_diagnostic_jobs_tests.rs` | 458 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_failover_runtime.rs` | 111 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_failover_runtime_tests.rs` | 383 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_failover_timeouts_tests.rs` | 397 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_high_availability_service.rs` | 225 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_high_availability_service_tests.rs` | 549 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_high_availability_store.rs` | 106 | 任务 29 → app-settings-store |
| `src/gateway/agent_apps_high_availability_store_tests.rs` | 285 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_skill_migration.rs` | 205 | 任务 273 → agent-gateway |
| `src/gateway/agent_apps_skills_store.rs` | 1330 | 任务 29 → app-settings-store |
| `src/gateway/agent_apps_skills_store_tests.rs` | 197 | 任务 273 → agent-gateway |
| `src/gateway/agent_tests.rs` | 451 | 任务 273 → agent-gateway |
| `src/gateway/aggregation_desktop_record.rs` | 264 | 任务 273 → agent-gateway |
| `src/gateway/aggregation_lifecycle.rs` | 365 | 任务 273 → agent-gateway |
| `src/gateway/aggregation_lifecycle_tests.rs` | 416 | 任务 273 → agent-gateway |
| `src/gateway/aggregation_resync/codex.rs` | 229 | 任务 274 → agent-gateway |
| `src/gateway/aggregation_resync/exit.rs` | 235 | 任务 274 → agent-gateway |
| `src/gateway/aggregation_resync/publish.rs` | 119 | 任务 274 → agent-gateway |
| `src/gateway/aggregation_resync/route.rs` | 273 | 任务 274 → agent-gateway |
| `src/gateway/aggregation_resync.rs` | 584 | 任务 274 → agent-gateway |
| `src/gateway/api/auth.rs` | 53 | 任务 275 → agent-gateway |
| `src/gateway/api/auth_tests.rs` | 118 | 任务 275 → agent-gateway |
| `src/gateway/api/handlers/agent_app_prompt_files.rs` | 162 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompt_shared_import.rs` | 110 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompt_sync.rs` | 135 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompt_templates.rs` | 176 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompts_activation.rs` | 201 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompts_read.rs` | 143 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_app_prompts_write.rs` | 163 | 任务 282 → gateway-agent-app |
| `src/gateway/api/handlers/agent_apps/access.rs` | 2489 | 任务 283 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/aggregation_lifecycle_access.rs` | 70 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs` | 471 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/apps_usage_refresh_tests.rs` | 483 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/apps_usage_script.rs` | 511 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/apps_usage_script_tests.rs` | 325 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/import_tests.rs` | 39 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/launch.rs` | 222 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/launch_identities.rs` | 82 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/launch_sessions.rs` | 81 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/launch_tests.rs` | 404 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/lifecycle.rs` | 465 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/lifecycle_tests.rs` | 306 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/local_access_tests.rs` | 156 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/mod.rs` | 726 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/preferences.rs` | 132 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/preferences_tests.rs` | 137 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/read.rs` | 109 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/read_tests.rs` | 364 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/recovery_tests.rs` | 62 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/remote_tests_tests.rs` | 247 | 任务 284 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_codex.rs` | 113 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_native.rs` | 96 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_read.rs` | 245 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_read_tests.rs` | 194 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_refresh.rs` | 670 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps/usage_refresh_tests.rs` | 236 | 任务 285 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_acceptance_tests.rs` | 361 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_add_tests.rs` | 101 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_binding.rs` | 216 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_binding_tests.rs` | 463 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_cancel.rs` | 54 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_cancel_tests.rs` | 247 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_default.rs` | 115 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_default_tests.rs` | 340 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_integration_tests.rs` | 420 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_logout.rs` | 252 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_logout_tests.rs` | 598 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_poll.rs` | 43 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_poll_tests.rs` | 222 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_read.rs` | 549 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_read_tests.rs` | 325 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_redaction_tests.rs` | 323 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_remove.rs` | 107 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_remove_tests.rs` | 335 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_security_tests.rs` | 426 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_start/native.rs` | 357 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_start.rs` | 429 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_auth_start_tests.rs` | 346 | 任务 286 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_acceptance_tests.rs` | 195 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_capture.rs` | 67 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_crud_api.rs` | 205 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_crud_api_tests.rs` | 148 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_restore_api.rs` | 438 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/backup_restore_api_tests.rs` | 174 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/data_aux_api.rs` | 118 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/data_aux_api_tests.rs` | 90 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/migration_acceptance_tests.rs` | 95 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/migration_read_api.rs` | 306 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/migration_write_api.rs` | 461 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/migration_write_api_tests.rs` | 194 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/mod.rs` | 33 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_data/periodic.rs` | 64 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_delete_tests.rs` | 53 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_diagnostics.rs` | 474 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_diagnostics_acceptance_tests.rs` | 555 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_diagnostics_endpoint_drafts_tests.rs` | 142 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_diagnostics_tests.rs` | 874 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_error.rs` | 40 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_error_tests.rs` | 129 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs` | 210 | 任务 287 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_high_availability.rs` | 402 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_high_availability_health.rs` | 203 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_high_availability_tests.rs` | 585 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_read_tests.rs` | 88 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_rename_tests.rs` | 83 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/backups_api.rs` | 131 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/backups_api_tests.rs` | 54 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/bindings_api.rs` | 154 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/claude_api.rs` | 118 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/claude_api_tests.rs` | 104 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/discovery_api.rs` | 103 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/discovery_api_tests.rs` | 37 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/import_api.rs` | 155 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/install_api.rs` | 134 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/install_api_tests.rs` | 144 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/inventory_api.rs` | 81 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/jobs_api.rs` | 69 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/jobs_api_tests.rs` | 78 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/macos_api_tests.rs` | 328 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/macos_files_tests.rs` | 112 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/mod.rs` | 52 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/repos_api.rs` | 85 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/search_api.rs` | 78 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/search_api_tests.rs` | 24 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/storage_api.rs` | 106 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/uninstall_api.rs` | 96 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/uninstall_api_tests.rs` | 66 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/updates_api.rs` | 147 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_apps_skills/updates_api_tests.rs` | 52 | 任务 288 → gateway-agent-apps |
| `src/gateway/api/handlers/agent_scheduler.rs` | 2980 | 任务 289 → gateway-agent-scheduler |
| `src/gateway/api/handlers/agent_scheduler_tests.rs` | 2941 | 任务 290 → gateway-agent-scheduler |
| `src/gateway/api/handlers/aggregation_clients.rs` | 288 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_clients_tests.rs` | 431 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_codex_settings.rs` | 315 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_codex_settings_fixture.rs` | 36 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_codex_settings_tests.rs` | 501 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_desktop.rs` | 324 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_desktop_routing_tests.rs` | 345 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_desktop_tests.rs` | 385 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_desktop_write.rs` | 155 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_exit.rs` | 340 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_exit_operation_tests.rs` | 139 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_exit_tests.rs` | 300 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_members.rs` | 475 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_members_client_tests.rs` | 240 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_members_ordinary_tests.rs` | 256 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_members_tests.rs` | 515 | 任务 291 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_omo.rs` | 504 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_omo_tests.rs` | 434 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_omp.rs` | 308 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_omp_tests.rs` | 402 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_pi.rs` | 255 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_pi_tests.rs` | 438 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_publish.rs` | 387 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_publish_client_tests.rs` | 164 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_publish_failure_tests.rs` | 67 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_publish_operation_tests.rs` | 114 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_publish_tests.rs` | 450 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_read.rs` | 290 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_read_snapshot.rs` | 481 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_read_tests.rs` | 508 | 任务 292 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_restart.rs` | 277 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_restart_operation_tests.rs` | 50 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_restart_tests.rs` | 345 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_resync.rs` | 167 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_resync_codex_tests.rs` | 175 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/aggregation_resync_tests.rs` | 459 | 任务 293 → gateway-aggregation |
| `src/gateway/api/handlers/app_providers/agy.rs` | 232 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/agy_tests.rs` | 361 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/api_security_tests.rs` | 473 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/basic.rs` | 36 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/basic_tests.rs` | 190 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/catalog.rs` | 69 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/catalog_tests.rs` | 192 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude.rs` | 582 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_desktop.rs` | 283 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_desktop_tests.rs` | 276 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_form.rs` | 85 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_login.rs` | 206 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_login_tests.rs` | 138 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_managed_binding_tests.rs` | 214 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/claude_tests.rs` | 633 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/codex_catalog.rs` | 523 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/codex_catalog_tests.rs` | 515 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/codex_tests.rs` | 77 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/create.rs` | 55 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/create_tests.rs` | 206 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/delete.rs` | 55 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/delete_tests.rs` | 193 | 任务 294 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/desktop_mapping.rs` | 267 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/desktop_mapping_tests.rs` | 250 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/dsh.rs` | 362 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/dsh_tests.rs` | 312 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/duplicate.rs` | 27 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/duplicate_tests.rs` | 206 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/editor_merge.rs` | 322 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/editor_merge_tests.rs` | 424 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/endpoints.rs` | 40 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/endpoints_tests.rs` | 179 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_clean.rs` | 66 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_clean_tests.rs` | 176 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_diagnose.rs` | 58 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_diagnose_tests.rs` | 120 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_restore.rs` | 63 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/env_restore_tests.rs` | 161 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/gemini.rs` | 345 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/gemini_tests.rs` | 411 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/grok.rs` | 348 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/grok_tests.rs` | 362 | 任务 295 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/hermes.rs` | 421 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/hermes_tests.rs` | 363 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/icons.rs` | 38 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/icons_tests.rs` | 229 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/import.rs` | 84 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/import_tests.rs` | 302 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/kimi.rs` | 452 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/kimi_tests.rs` | 457 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mcode.rs` | 426 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mcode_tests.rs` | 397 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mode_apply.rs` | 379 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mode_apply_tests.rs` | 752 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mode_state.rs` | 115 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/mode_state_tests.rs` | 247 | 任务 296 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/omp.rs` | 422 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/omp_tests.rs` | 353 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/openclaw.rs` | 344 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/openclaw_tests.rs` | 345 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/opencode.rs` | 346 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/opencode_tests.rs` | 354 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/operations/aggregation.rs` | 226 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/operations.rs` | 340 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/operations_tests.rs` | 182 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/paths.rs` | 210 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/paths_tests.rs` | 125 | 任务 297 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/pi.rs` | 429 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/pi_tests.rs` | 387 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/presentation.rs` | 65 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/presentation_tests.rs` | 161 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/preview.rs` | 228 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/preview_tests.rs` | 261 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/request_options.rs` | 73 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/request_options_tests.rs` | 269 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/snippets.rs` | 119 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/snippets_tests.rs` | 352 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/universal.rs` | 60 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/universal_sync.rs` | 27 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/universal_sync_tests.rs` | 164 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/universal_tests.rs` | 200 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/update.rs` | 54 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/update_tests.rs` | 201 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers/validation_tests.rs` | 207 | 任务 298 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers.rs` | 396 | 任务 299 → gateway-app-providers |
| `src/gateway/api/handlers/app_providers_tests.rs` | 223 | 任务 299 → gateway-app-providers |
| `src/gateway/api/handlers/app_routing/client_host.rs` | 37 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/config_api.rs` | 164 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/config_api_tests.rs` | 238 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/lifecycle_api.rs` | 606 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/lifecycle_api_tests.rs` | 350 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/mod.rs` | 42 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/mode_attach.rs` | 105 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_api.rs` | 59 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_api_tests.rs` | 124 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_probe.rs` | 196 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_probe_tests.rs` | 270 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_scan.rs` | 241 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/outbound_scan_tests.rs` | 246 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/app_routing/request_status.rs` | 27 | 任务 300 → gateway-app-routing |
| `src/gateway/api/handlers/apps_usage_acceptance_tests.rs` | 341 | 任务 301 → gateway-apps-usage-acceptance |
| `src/gateway/api/handlers/auth.rs` | 41 | 任务 302 → gateway-auth |
| `src/gateway/api/handlers/auth_tests.rs` | 310 | 任务 302 → gateway-auth |
| `src/gateway/api/handlers/backup_inventory.rs` | 43 | 任务 303 → gateway-backup-inventory |
| `src/gateway/api/handlers/channel_qr/dingtalk.rs` | 138 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/dingtalk_tests.rs` | 139 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/feishu.rs` | 127 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/feishu_tests.rs` | 104 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/qq.rs` | 161 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/qq_tests.rs` | 115 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/wechat.rs` | 150 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/wechat_tests.rs` | 123 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/wecom.rs` | 154 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr/wecom_tests.rs` | 95 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr.rs` | 244 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/channel_qr_tests.rs` | 115 | 任务 304 → gateway-channel |
| `src/gateway/api/handlers/computer_control.rs` | 144 | 任务 305 → gateway-computer-control |
| `src/gateway/api/handlers/computer_control_tests.rs` | 58 | 任务 305 → gateway-computer-control |
| `src/gateway/api/handlers/config.rs` | 458 | 任务 306 → gateway-config |
| `src/gateway/api/handlers/config_tests.rs` | 1738 | 任务 306 → gateway-config |
| `src/gateway/api/handlers/cron.rs` | 783 | 任务 307 → gateway-cron |
| `src/gateway/api/handlers/cron_schedule.rs` | 306 | 任务 307 → gateway-cron |
| `src/gateway/api/handlers/cron_schedule_tests.rs` | 438 | 任务 307 → gateway-cron |
| `src/gateway/api/handlers/cron_tests.rs` | 1023 | 任务 307 → gateway-cron |
| `src/gateway/api/handlers/desktop.rs` | 983 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_acp_install.rs` | 29 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_acp_install_tests.rs` | 7 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_browser_install.rs` | 30 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_browser_install_tests.rs` | 9 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_skills.rs` | 1033 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_skills_tests.rs` | 828 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/desktop_tests.rs` | 679 | 任务 308 → gateway-desktop |
| `src/gateway/api/handlers/device_sync_config.rs` | 181 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/device_sync_download.rs` | 178 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/device_sync_jobs.rs` | 173 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/device_sync_probe.rs` | 320 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/device_sync_projection.rs` | 93 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/device_sync_tasks.rs` | 127 | 任务 309 → gateway-device-sync |
| `src/gateway/api/handlers/dynamic_tools.rs` | 130 | 任务 310 → gateway-dynamic-tools |
| `src/gateway/api/handlers/dynamic_tools_tests.rs` | 448 | 任务 310 → gateway-dynamic-tools |
| `src/gateway/api/handlers/external_sessions/catalog.rs` | 131 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/compression.rs` | 108 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/content.rs` | 76 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/delete.rs` | 197 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/images.rs` | 68 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/maintenance/history_settings.rs` | 78 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/maintenance/host.rs` | 364 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/maintenance.rs` | 211 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/markdown.rs` | 55 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/messages.rs` | 261 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/mod.rs` | 25 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/resume.rs` | 105 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/scan.rs` | 39 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/stream.rs` | 256 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/external_sessions/usage_scan.rs` | 232 | 任务 311 → gateway-external-sessions |
| `src/gateway/api/handlers/file.rs` | 962 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_drag_integration_tests.rs` | 404 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_duplicate.rs` | 79 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_duplicate_tests.rs` | 76 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_import.rs` | 387 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_import_tests.rs` | 249 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/file_tests.rs` | 1254 | 任务 312 → gateway-file |
| `src/gateway/api/handlers/git/history.rs` | 176 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git/remote.rs` | 147 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git/repository.rs` | 64 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git/repository_tests.rs` | 216 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git.rs` | 836 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git_command.rs` | 79 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git_command_tests.rs` | 175 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/git_tests.rs` | 1517 | 任务 313 → gateway-git |
| `src/gateway/api/handlers/global.rs` | 226 | 任务 314 → gateway-global |
| `src/gateway/api/handlers/global_tests.rs` | 807 | 任务 314 → gateway-global |
| `src/gateway/api/handlers/hermes_dashboard.rs` | 99 | 任务 315 → gateway-hermes-dashboard |
| `src/gateway/api/handlers/hermes_dashboard_tests.rs` | 7 | 任务 315 → gateway-hermes-dashboard |
| `src/gateway/api/handlers/hermes_memory.rs` | 95 | 任务 316 → gateway-hermes-memory |
| `src/gateway/api/handlers/hermes_memory_tests.rs` | 59 | 任务 316 → gateway-hermes-memory |
| `src/gateway/api/handlers/hooks.rs` | 238 | 任务 317 → gateway-hooks |
| `src/gateway/api/handlers/hooks_tests.rs` | 238 | 任务 317 → gateway-hooks |
| `src/gateway/api/handlers/instance.rs` | 168 | 任务 318 → gateway-instance |
| `src/gateway/api/handlers/instance_tests.rs` | 444 | 任务 318 → gateway-instance |
| `src/gateway/api/handlers/integrations.rs` | 256 | 任务 319 → gateway-integrations |
| `src/gateway/api/handlers/integrations_tests.rs` | 468 | 任务 319 → gateway-integrations |
| `src/gateway/api/handlers/knowledge.rs` | 555 | 任务 320 → gateway-knowledge |
| `src/gateway/api/handlers/knowledge_tests.rs` | 1105 | 任务 320 → gateway-knowledge |
| `src/gateway/api/handlers/mcp.rs` | 79 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_bindings.rs` | 199 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_bindings_tests.rs` | 177 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_create.rs` | 92 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_create_tests.rs` | 210 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_delete.rs` | 201 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_delete_tests.rs` | 239 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_import.rs` | 247 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_import_tests.rs` | 201 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_probe.rs` | 51 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_probe_tests.rs` | 331 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_read.rs` | 356 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_read_tests.rs` | 290 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_resync.rs` | 461 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_resync_tests.rs` | 249 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_tools.rs` | 150 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_tools_tests.rs` | 131 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_update.rs` | 72 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_management_update_tests.rs` | 208 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/mcp_tests.rs` | 179 | 任务 321 → gateway-mcp |
| `src/gateway/api/handlers/memory.rs` | 108 | 任务 322 → gateway-memory |
| `src/gateway/api/handlers/memory_tests.rs` | 514 | 任务 322 → gateway-memory |
| `src/gateway/api/handlers/misc.rs` | 148 | 任务 323 → gateway-misc |
| `src/gateway/api/handlers/misc_tests.rs` | 304 | 任务 323 → gateway-misc |
| `src/gateway/api/handlers/mod.rs` | 473 | 任务 324 → gateway-mod |
| `src/gateway/api/handlers/model_catalog/presets.rs` | 215 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_catalog.rs` | 239 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_catalog_acceptance_tests.rs` | 555 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_catalog_edits.rs` | 215 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_catalog_edits_tests.rs` | 247 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_catalog_tests.rs` | 304 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_discovery/source.rs` | 237 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_discovery.rs` | 269 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/model_discovery_tests.rs` | 466 | 任务 325 → gateway-model |
| `src/gateway/api/handlers/openclaw_settings.rs` | 150 | 任务 326 → gateway-openclaw-settings |
| `src/gateway/api/handlers/openclaw_settings_tests.rs` | 74 | 任务 326 → gateway-openclaw-settings |
| `src/gateway/api/handlers/permission.rs` | 218 | 任务 327 → gateway-permission |
| `src/gateway/api/handlers/permission_tests.rs` | 614 | 任务 327 → gateway-permission |
| `src/gateway/api/handlers/plugins.rs` | 372 | 任务 328 → gateway-plugins |
| `src/gateway/api/handlers/plugins_operations.rs` | 302 | 任务 328 → gateway-plugins |
| `src/gateway/api/handlers/plugins_tests.rs` | 165 | 任务 328 → gateway-plugins |
| `src/gateway/api/handlers/project.rs` | 516 | 任务 329 → gateway-project |
| `src/gateway/api/handlers/project_profiles.rs` | 96 | 任务 329 → gateway-project |
| `src/gateway/api/handlers/project_profiles_execution.rs` | 131 | 任务 329 → gateway-project |
| `src/gateway/api/handlers/project_profiles_maintenance.rs` | 139 | 任务 329 → gateway-project |
| `src/gateway/api/handlers/project_tests.rs` | 1133 | 任务 329 → gateway-project |
| `src/gateway/api/handlers/provider.rs` | 110 | 任务 330 → gateway-provider |
| `src/gateway/api/handlers/provider_tests.rs` | 264 | 任务 330 → gateway-provider |
| `src/gateway/api/handlers/pty.rs` | 169 | 任务 331 → gateway-pty |
| `src/gateway/api/handlers/pty_tests.rs` | 132 | 任务 331 → gateway-pty |
| `src/gateway/api/handlers/question.rs` | 46 | 任务 332 → gateway-question |
| `src/gateway/api/handlers/question_tests.rs` | 195 | 任务 332 → gateway-question |
| `src/gateway/api/handlers/resumable_stream.rs` | 440 | 任务 333 → gateway-resumable-stream |
| `src/gateway/api/handlers/resumable_stream_tests.rs` | 247 | 任务 333 → gateway-resumable-stream |
| `src/gateway/api/handlers/session/acp_options.rs` | 129 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/acp_source_tests.rs` | 253 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` | 1082 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/archived_delete.rs` | 160 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/archived_delete_tests.rs` | 330 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/fork_acceptance_tests.rs` | 506 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/fork_acp_tests.rs` | 288 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/message_ops.rs` | 164 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/message_ops_tests.rs` | 451 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/raw_commit.rs` | 192 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/raw_commit_tests.rs` | 62 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/session_ops.rs` | 740 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/session_ops_tests.rs` | 765 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/shared.rs` | 130 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/shared_tests.rs` | 273 | 任务 334 → gateway-session |
| `src/gateway/api/handlers/session/stream.rs` | 2790 | 任务 335 → gateway-session |
| `src/gateway/api/handlers/session/stream_tests.rs` | 3363 | 任务 336 → gateway-session |
| `src/gateway/api/handlers/session/turn_fork_tests.rs` | 817 | 任务 336 → gateway-session |
| `src/gateway/api/handlers/session/ui_ops.rs` | 576 | 任务 336 → gateway-session |
| `src/gateway/api/handlers/session/ui_ops_tests.rs` | 1677 | 任务 336 → gateway-session |
| `src/gateway/api/handlers/session.rs` | 135 | 任务 337 → gateway-session |
| `src/gateway/api/handlers/session_tests.rs` | 370 | 任务 337 → gateway-session |
| `src/gateway/api/handlers/shiju_account.rs` | 179 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_account_tests.rs` | 250 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_apps.rs` | 124 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_apps_tests.rs` | 391 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_connectors.rs` | 65 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_knowledge.rs` | 92 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_knowledge_gateway_tests.rs` | 248 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_skills.rs` | 277 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/shiju_skills_tests.rs` | 356 | 任务 338 → gateway-shiju |
| `src/gateway/api/handlers/skill_market.rs` | 599 | 任务 339 → gateway-skill-market |
| `src/gateway/api/handlers/skill_market_tests.rs` | 681 | 任务 339 → gateway-skill-market |
| `src/gateway/api/handlers/status.rs` | 141 | 任务 340 → gateway-status |
| `src/gateway/api/handlers/status_tests.rs` | 168 | 任务 340 → gateway-status |
| `src/gateway/api/handlers/task_lifecycle.rs` | 120 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/task_lifecycle_tests.rs` | 259 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/task_plan_run.rs` | 219 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/task_plan_run_tests.rs` | 142 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/task_pool.rs` | 167 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/task_pool_tests.rs` | 80 | 任务 341 → gateway-task |
| `src/gateway/api/handlers/tests.rs` | 12 | 任务 342 → gateway-tests |
| `src/gateway/api/handlers/tools.rs` | 57 | 任务 343 → gateway-tools |
| `src/gateway/api/handlers/tools_tests.rs` | 182 | 任务 343 → gateway-tools |
| `src/gateway/api/handlers/usage/events.rs` | 81 | 任务 344 → gateway-usage |
| `src/gateway/api/handlers/usage/mod.rs` | 6 | 任务 344 → gateway-usage |
| `src/gateway/api/handlers/usage/prices.rs` | 169 | 任务 344 → gateway-usage |
| `src/gateway/api/handlers/usage/read.rs` | 159 | 任务 344 → gateway-usage |
| `src/gateway/api/handlers/usage/scan.rs` | 183 | 任务 344 → gateway-usage |
| `src/gateway/api/handlers/voice.rs` | 216 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_handoff.rs` | 247 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_handoff_session_tests.rs` | 176 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_handoff_tests.rs` | 70 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_realtime.rs` | 531 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_realtime_http_tests.rs` | 90 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_realtime_tests.rs` | 412 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_requests.rs` | 133 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_requests_tests.rs` | 55 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_settings.rs` | 164 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_settings_tests.rs` | 50 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_tests.rs` | 321 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_upload.rs` | 53 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/voice_upload_tests.rs` | 80 | 任务 345 → gateway-voice |
| `src/gateway/api/handlers/workflow.rs` | 1165 | 任务 346 → gateway-workflow |
| `src/gateway/api/handlers/workflow_tests.rs` | 1597 | 任务 346 → gateway-workflow |
| `src/gateway/api/handlers/workspace_directory.rs` | 53 | 任务 347 → gateway-workspace |
| `src/gateway/api/handlers/workspace_memory.rs` | 148 | 任务 347 → gateway-workspace |
| `src/gateway/api/handlers/workspace_memory_tests.rs` | 35 | 任务 347 → gateway-workspace |
| `src/gateway/api/handlers/worktree_management.rs` | 158 | 任务 348 → gateway-worktree |
| `src/gateway/api/handlers/worktree_management_tests.rs` | 257 | 任务 348 → gateway-worktree |
| `src/gateway/api/integrations.rs` | 664 | 任务 275 → agent-gateway |
| `src/gateway/api/integrations_tests.rs` | 304 | 任务 275 → agent-gateway |
| `src/gateway/api/mod.rs` | 117 | 任务 275 → agent-gateway |
| `src/gateway/api/secrets.rs` | 612 | 任务 275 → agent-gateway |
| `src/gateway/api/secrets_tests.rs` | 557 | 任务 275 → agent-gateway |
| `src/gateway/api/tests.rs` | 242 | 任务 275 → agent-gateway |
| `src/gateway/api/types.rs` | 517 | 任务 275 → agent-gateway |
| `src/gateway/api/types_tests.rs` | 30 | 任务 275 → agent-gateway |
| `src/gateway/approval_state.rs` | 72 | 任务 275 → agent-gateway |
| `src/gateway/approval_state_tests.rs` | 158 | 任务 275 → agent-gateway |
| `src/gateway/bootstrap.rs` | 979 | 任务 276 → agent-gateway |
| `src/gateway/bootstrap_tests.rs` | 394 | 任务 276 → agent-gateway |
| `src/gateway/channel_reload.rs` | 176 | 任务 276 → agent-gateway |
| `src/gateway/channel_reload_tests.rs` | 49 | 任务 276 → agent-gateway |
| `src/gateway/channel_runtime.rs` | 75 | 任务 276 → agent-gateway |
| `src/gateway/chat.rs` | 405 | 任务 276 → agent-gateway |
| `src/gateway/chat_tests.rs` | 474 | 任务 276 → agent-gateway |
| `src/gateway/deepagents_window_tests.rs` | 112 | 任务 276 → agent-gateway |
| `src/gateway/error.rs` | 91 | 任务 269 → gateway-types |
| `src/gateway/error_tests.rs` | 80 | 任务 269 → gateway-types |
| `src/gateway/handlers.rs` | 567 | 任务 277 → agent-gateway |
| `src/gateway/handlers_tests.rs` | 1221 | 任务 277 → agent-gateway |
| `src/gateway/health.rs` | 181 | 任务 277 → agent-gateway |
| `src/gateway/health_tests.rs` | 159 | 任务 277 → agent-gateway |
| `src/gateway/instance.rs` | 140 | 任务 277 → agent-gateway |
| `src/gateway/instance_tests.rs` | 281 | 任务 277 → agent-gateway |
| `src/gateway/limits.rs` | 472 | 任务 269 → gateway-types |
| `src/gateway/limits_tests.rs` | 262 | 任务 269 → gateway-types |
| `src/gateway/middleware.rs` | 268 | 任务 277 → agent-gateway |
| `src/gateway/middleware_tests.rs` | 337 | 任务 277 → agent-gateway |
| `src/gateway/mod.rs` | 189 | 任务 281 → agent-gateway |
| `src/gateway/mod_tests.rs` | 357 | 任务 277 → agent-gateway |
| `src/gateway/node_control.rs` | 190 | 任务 277 → agent-gateway |
| `src/gateway/node_control_tests.rs` | 515 | 任务 277 → agent-gateway |
| `src/gateway/openai_compat/mod.rs` | 574 | 任务 277 → agent-gateway |
| `src/gateway/openai_compat/mod_tests.rs` | 547 | 任务 277 → agent-gateway |
| `src/gateway/openai_compat/tests.rs` | 372 | 任务 277 → agent-gateway |
| `src/gateway/options.rs` | 76 | 任务 269 → gateway-types |
| `src/gateway/options_tests.rs` | 13 | 任务 269 → gateway-types |
| `src/gateway/pairing.rs` | 300 | 任务 278 → agent-gateway |
| `src/gateway/pairing_tests.rs` | 271 | 任务 278 → agent-gateway |
| `src/gateway/recovery_window_tests.rs` | 168 | 任务 278 → agent-gateway |
| `src/gateway/router.rs` | 252 | 任务 278 → agent-gateway |
| `src/gateway/router_tests.rs` | 221 | 任务 278 → agent-gateway |
| `src/gateway/runtime.rs` | 124 | 任务 278 → agent-gateway |
| `src/gateway/runtime_tests.rs` | 189 | 任务 278 → agent-gateway |
| `src/gateway/settings_store/postgres.rs` | 358 | 任务 29 → app-settings-store |
| `src/gateway/settings_store/postgres_tests.rs` | 58 | 任务 29 → app-settings-store |
| `src/gateway/settings_store.rs` | 633 | 任务 29 → app-settings-store |
| `src/gateway/settings_store_tests.rs` | 125 | 任务 278 → agent-gateway |
| `src/gateway/sse.rs` | 330 | 任务 278 → agent-gateway |
| `src/gateway/sse_tests.rs` | 317 | 任务 278 → agent-gateway |
| `src/gateway/state.rs` | 169 | 任务 269 → gateway-types |
| `src/gateway/state_tests.rs` | 10 | 任务 269 → gateway-types |
| `src/gateway/tests/agent.rs` | 93 | 任务 279 → agent-gateway |
| `src/gateway/tests/allowlist.rs` | 45 | 任务 279 → agent-gateway |
| `src/gateway/tests/auth.rs` | 357 | 任务 279 → agent-gateway |
| `src/gateway/tests/basics.rs` | 104 | 任务 279 → agent-gateway |
| `src/gateway/tests/client_key.rs` | 169 | 任务 279 → agent-gateway |
| `src/gateway/tests/idempotency.rs` | 222 | 任务 279 → agent-gateway |
| `src/gateway/tests/memory_keys.rs` | 81 | 任务 279 → agent-gateway |
| `src/gateway/tests/metrics.rs` | 193 | 任务 279 → agent-gateway |
| `src/gateway/tests/mod.rs` | 441 | 任务 280 → agent-gateway |
| `src/gateway/tests/nextcloud.rs` | 224 | 任务 279 → agent-gateway |
| `src/gateway/tests/node_control.rs` | 178 | 任务 279 → agent-gateway |
| `src/gateway/tests/pairing.rs` | 302 | 任务 279 → agent-gateway |
| `src/gateway/tests/qq.rs` | 227 | 任务 279 → agent-gateway |
| `src/gateway/tests/rate_limiter.rs` | 341 | 任务 280 → agent-gateway |
| `src/gateway/tests/sanitize.rs` | 175 | 任务 280 → agent-gateway |
| `src/gateway/tests/state.rs` | 13 | 任务 280 → agent-gateway |
| `src/gateway/tests/webhook.rs` | 519 | 任务 280 → agent-gateway |
| `src/gateway/tests/whatsapp_signature.rs` | 256 | 任务 280 → agent-gateway |
| `src/gateway/types.rs` | 113 | 任务 269 → gateway-types |
| `src/gateway/types_tests.rs` | 15 | 任务 269 → gateway-types |
| `src/gateway/util.rs` | 359 | 任务 281 → agent-gateway |
| `src/gateway/util_tests.rs` | 141 | 任务 281 → agent-gateway |
| `src/gateway/webhook.rs` | 81 | 任务 281 → agent-gateway |
| `src/gateway/webhook_ingress/mod.rs` | 517 | 任务 281 → agent-gateway |
| `src/gateway/webhook_ingress/tests.rs` | 868 | 任务 281 → agent-gateway |
| `src/gateway/webhook_tests.rs` | 214 | 任务 281 → agent-gateway |
| `src/gateway/ws/mod.rs` | 694 | 任务 281 → agent-gateway |
| `src/gateway/ws/mod_tests.rs` | 301 | 任务 281 → agent-gateway |
| `src/gateway/ws/tests.rs` | 503 | 任务 281 → agent-gateway |
| `src/git_repository.rs` | 3 | 任务 356 → agent-composition |
| `src/global_database.rs` | 6 | 既有 agent-database；保留薄导出 |
| `src/global_database_tests.rs` | 79 | 任务 364 → 主包跨数据库集成验收（涉及三领域，不下沉数据库库） |
| `src/health/mod.rs` | 349 | 任务 41 → agent-health |
| `src/health/mod_tests.rs` | 99 | 任务 41 → agent-health |
| `src/health/tests.rs` | 28 | 任务 41 → agent-health |
| `src/heartbeat/engine/mod.rs` | 323 | 任务 352 → agent-heartbeat |
| `src/heartbeat/engine/mod_tests.rs` | 284 | 任务 352 → agent-heartbeat |
| `src/heartbeat/engine/tests.rs` | 190 | 任务 352 → agent-heartbeat |
| `src/heartbeat/mod.rs` | 52 | 任务 352 → agent-heartbeat |
| `src/heartbeat/tests.rs` | 85 | 任务 352 → agent-heartbeat |
| `src/hooks/builtin/command_logger/mod.rs` | 225 | 任务 69 → agent-hooks |
| `src/hooks/builtin/command_logger/mod_tests.rs` | 61 | 任务 69 → agent-hooks |
| `src/hooks/builtin/command_logger/tests.rs` | 148 | 任务 69 → agent-hooks |
| `src/hooks/builtin/mod.rs` | 43 | 任务 69 → agent-hooks |
| `src/hooks/mod.rs` | 96 | 任务 69 → agent-hooks |
| `src/hooks/runner/mod.rs` | 603 | 任务 69 → agent-hooks |
| `src/hooks/runner/tests.rs` | 713 | 任务 69 → agent-hooks |
| `src/hooks/runtime.rs` | 137 | 任务 69 → agent-hooks |
| `src/hooks/runtime_tests.rs` | 161 | 任务 69 → agent-hooks |
| `src/hooks/tests.rs` | 10 | 任务 69 → agent-hooks |
| `src/hooks/traits/mod.rs` | 496 | 任务 69 → agent-hooks |
| `src/hooks/traits/mod_tests.rs` | 170 | 任务 69 → agent-hooks |
| `src/hooks/traits/tests.rs` | 264 | 任务 69 → agent-hooks |
| `src/id/mod.rs` | 6 | 任务 356 → agent-composition |
| `src/identity/load.rs` | 78 | 任务 42 → agent-identity-loader |
| `src/identity/load_tests.rs` | 140 | 任务 42 → agent-identity-loader |
| `src/identity/mod.rs` | 46 | 任务 42 → agent-composition（宿主重导出接线，模型来自agent-identity，加载来自agent-identity-loader） |
| `src/identity/tests.rs` | 298 | 任务 42 → agent-identity-loader |
| `src/installation/mod.rs` | 571 | 任务 43 → agent-installation |
| `src/installation/mod_tests.rs` | 607 | 任务 43 → agent-installation |
| `src/installation/tests.rs` | 181 | 任务 43 → agent-installation |
| `src/integrations/mod.rs` | 364 | 任务 44 → agent-integrations |
| `src/integrations/mod_tests.rs` | 166 | 任务 44 → agent-integrations |
| `src/integrations/registry.rs` | 767 | 任务 44 → agent-integrations |
| `src/integrations/registry_tests.rs` | 600 | 任务 44 → agent-integrations |
| `src/integrations/tests.rs` | 193 | 任务 44 → agent-integrations |
| `src/knowledge/chunker/patterns.rs` | 40 | 任务 70 → agent-knowledge |
| `src/knowledge/chunker/patterns_tests.rs` | 27 | 任务 70 → agent-knowledge |
| `src/knowledge/chunker/profile.rs` | 72 | 任务 70 → agent-knowledge |
| `src/knowledge/chunker/profile_tests.rs` | 48 | 任务 70 → agent-knowledge |
| `src/knowledge/chunker.rs` | 416 | 任务 70 → agent-knowledge |
| `src/knowledge/chunker_tests.rs` | 213 | 任务 70 → agent-knowledge |
| `src/knowledge/fusion.rs` | 210 | 任务 70 → agent-knowledge |
| `src/knowledge/fusion_tests.rs` | 83 | 任务 70 → agent-knowledge |
| `src/knowledge/import.rs` | 282 | 任务 70 → agent-knowledge |
| `src/knowledge/import_tests.rs` | 244 | 任务 70 → agent-knowledge |
| `src/knowledge/mod.rs` | 21 | 任务 74 → agent-knowledge |
| `src/knowledge/store/postgres/cache.rs` | 95 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/cache_tests.rs` | 39 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/chunks.rs` | 427 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/chunks_tests.rs` | 122 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/datasets.rs` | 197 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/datasets_tests.rs` | 60 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/documents.rs` | 278 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/documents_tests.rs` | 129 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/indexing.rs` | 519 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/indexing_tests.rs` | 82 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/mod.rs` | 31 | 任务 72 → agent-knowledge |
| `src/knowledge/store/postgres/mod_tests.rs` | 28 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/retrieval.rs` | 518 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/retrieval_tests.rs` | 78 | 任务 71 → agent-knowledge |
| `src/knowledge/store/postgres/schema.rs` | 219 | 任务 72 → agent-knowledge |
| `src/knowledge/store/postgres/schema_tests.rs` | 41 | 任务 72 → agent-knowledge |
| `src/knowledge/store.rs` | 3924 | 任务 73 → agent-knowledge |
| `src/knowledge/store_tests.rs` | 3172 | 任务 74 → agent-knowledge |
| `src/knowledge/tests.rs` | 346 | 任务 74 → agent-knowledge |
| `src/lib.rs` | 420 | 任务 356 → agent-composition |
| `src/mcp/mod.rs` | 286 | 任务 75 → agent-mcp |
| `src/mcp/tests.rs` | 151 | 任务 75 → agent-mcp |
| `src/memory/backend/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/chunker/mod.rs` | 6 | 任务 76 → agent-memory |
| `src/memory/chunker/reexport_tests.rs` | 12 | 任务 76 → agent-memory |
| `src/memory/cli/mod.rs` | 621 | 任务 76 → agent-memory |
| `src/memory/cli/tests.rs` | 543 | 任务 76 → agent-memory |
| `src/memory/embeddings/alibaba.rs` | 104 | 任务 76 → agent-memory |
| `src/memory/embeddings/alibaba_tests.rs` | 156 | 任务 76 → agent-memory |
| `src/memory/embeddings/mod.rs` | 411 | 任务 76 → agent-memory |
| `src/memory/embeddings/tests.rs` | 421 | 任务 76 → agent-memory |
| `src/memory/hygiene/mod.rs` | 754 | 任务 76 → agent-memory |
| `src/memory/hygiene/mod_tests.rs` | 349 | 任务 76 → agent-memory |
| `src/memory/hygiene/tests.rs` | 321 | 任务 76 → agent-memory |
| `src/memory/lucid/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/mariadb/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/markdown/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/mod.rs` | 919 | 任务 77 → agent-memory |
| `src/memory/mod_tests.rs` | 517 | 任务 76 → agent-memory |
| `src/memory/none/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/paths.rs` | 5 | 任务 76 → agent-memory |
| `src/memory/postgres/mod.rs` | 3 | 任务 76 → agent-memory |
| `src/memory/qdrant/mod.rs` | 931 | 任务 77 → agent-memory |
| `src/memory/qdrant/tests.rs` | 845 | 任务 77 → agent-memory |
| `src/memory/response_cache/mod.rs` | 3 | 任务 77 → agent-memory |
| `src/memory/snapshot/mod.rs` | 3 | 任务 77 → agent-memory |
| `src/memory/sqlite/mod.rs` | 3 | 任务 77 → agent-memory |
| `src/memory/tests.rs` | 328 | 任务 77 → agent-memory |
| `src/memory/traits/mod.rs` | 3 | 任务 77 → agent-memory |
| `src/memory/vector/mod.rs` | 6 | 任务 77 → agent-memory |
| `src/memory/vector/reexport_tests.rs` | 17 | 任务 77 → agent-memory |
| `src/migration/mod.rs` | 918 | 任务 353 → agent-migration |
| `src/migration/mod_tests.rs` | 95 | 任务 353 → agent-migration |
| `src/migration/tests/backup.rs` | 77 | 任务 353 → agent-migration |
| `src/migration/tests/migration.rs` | 115 | 任务 353 → agent-migration |
| `src/migration/tests/mod.rs` | 174 | 任务 353 → agent-migration |
| `src/migration/tests/parse.rs` | 139 | 任务 353 → agent-migration |
| `src/migration/tests/sqlite.rs` | 102 | 任务 353 → agent-migration |
| `src/migration/tests/workspace.rs` | 89 | 任务 353 → agent-migration |
| `src/mod.rs` | 445 | 任务 356 → agent-composition |
| `src/model_catalog.rs` | 91 | 任务 98 → model-catalog |
| `src/model_relay/alpha_search.rs` | 24 | 任务 99 → model-protocol |
| `src/model_relay/binding.rs` | 78 | 任务 106 → model-transport |
| `src/model_relay/binding_tests.rs` | 40 | 任务 106 → model-transport |
| `src/model_relay/chat.rs` | 88 | 任务 99 → model-protocol |
| `src/model_relay/chat_tests.rs` | 259 | 任务 99 → model-protocol |
| `src/model_relay/codex_oauth_transport.rs` | 76 | 任务 106 → model-transport |
| `src/model_relay/compaction.rs` | 21 | 任务 99 → model-protocol |
| `src/model_relay/compaction_v2.rs` | 145 | 任务 99 → model-protocol |
| `src/model_relay/compaction_v2_tests.rs` | 63 | 任务 99 → model-protocol |
| `src/model_relay/copilot_transport.rs` | 76 | 任务 106 → model-transport |
| `src/model_relay/copilot_transport_tests.rs` | 15 | 任务 106 → model-transport |
| `src/model_relay/forced_sse.rs` | 251 | 任务 99 → model-protocol |
| `src/model_relay/gemini.rs` | 452 | 任务 99 → model-protocol |
| `src/model_relay/gemini_metadata.rs` | 71 | 任务 99 → model-protocol |
| `src/model_relay/gemini_metadata_tests.rs` | 34 | 任务 99 → model-protocol |
| `src/model_relay/gemini_oauth_transport.rs` | 68 | 任务 106 → model-transport |
| `src/model_relay/gemini_shadow.rs` | 89 | 任务 99 → model-protocol |
| `src/model_relay/gemini_shadow_tests.rs` | 41 | 任务 99 → model-protocol |
| `src/model_relay/gemini_stream.rs` | 239 | 任务 99 → model-protocol |
| `src/model_relay/gemini_tests.rs` | 616 | 任务 99 → model-protocol |
| `src/model_relay/generation.rs` | 175 | 任务 99 → model-protocol |
| `src/model_relay/hosted_search.rs` | 137 | 任务 99 → model-protocol |
| `src/model_relay/hosted_search_tests.rs` | 50 | 任务 99 → model-protocol |
| `src/model_relay/image_edit.rs` | 60 | 任务 99 → model-protocol |
| `src/model_relay/image_edit_tests.rs` | 27 | 任务 99 → model-protocol |
| `src/model_relay/image_generation.rs` | 34 | 任务 99 → model-protocol |
| `src/model_relay/inline_think.rs` | 46 | 任务 99 → model-protocol |
| `src/model_relay/kiro.rs` | 49 | 任务 99 → model-protocol |
| `src/model_relay/late_arguments.rs` | 163 | 任务 99 → model-protocol |
| `src/model_relay/late_arguments_tests.rs` | 57 | 任务 99 → model-protocol |
| `src/model_relay/messages.rs` | 515 | 任务 100 → model-protocol |
| `src/model_relay/messages_chat_request.rs` | 81 | 任务 100 → model-protocol |
| `src/model_relay/messages_chat_request_tests.rs` | 62 | 任务 100 → model-protocol |
| `src/model_relay/messages_chat_result.rs` | 141 | 任务 100 → model-protocol |
| `src/model_relay/messages_chat_result_tests.rs` | 82 | 任务 100 → model-protocol |
| `src/model_relay/messages_chat_stream_tests.rs` | 133 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_request.rs` | 77 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_request_tests.rs` | 43 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_result.rs` | 148 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_result_tests.rs` | 148 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_stream.rs` | 355 | 任务 100 → model-protocol |
| `src/model_relay/messages_gemini_stream_tests.rs` | 198 | 任务 100 → model-protocol |
| `src/model_relay/messages_request_core_tests.rs` | 144 | 任务 100 → model-protocol |
| `src/model_relay/messages_request_http_tests.rs` | 156 | 任务 100 → model-protocol |
| `src/model_relay/messages_request_tests.rs` | 23 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_request.rs` | 95 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_request_tests.rs` | 50 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_result.rs` | 182 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_result_tests.rs` | 103 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_stream.rs` | 541 | 任务 100 → model-protocol |
| `src/model_relay/messages_responses_stream_tests.rs` | 256 | 任务 100 → model-protocol |
| `src/model_relay/messages_stream.rs` | 427 | 任务 101 → model-protocol |
| `src/model_relay/messages_stream_tests.rs` | 198 | 任务 101 → model-protocol |
| `src/model_relay/messages_tests.rs` | 572 | 任务 101 → model-protocol |
| `src/model_relay/messages_thinking.rs` | 307 | 任务 101 → model-protocol |
| `src/model_relay/messages_thinking_config.rs` | 122 | 任务 101 → model-protocol |
| `src/model_relay/messages_thinking_core_tests.rs` | 256 | 任务 101 → model-protocol |
| `src/model_relay/messages_thinking_http_tests.rs` | 253 | 任务 101 → model-protocol |
| `src/model_relay/messages_thinking_tests.rs` | 16 | 任务 101 → model-protocol |
| `src/model_relay/mod.rs` | 216 | 任务 349 → model-relay-http |
| `src/model_relay/models.rs` | 29 | 任务 101 → model-protocol |
| `src/model_relay/native_chat.rs` | 10 | 任务 101 → model-protocol |
| `src/model_relay/native_chat_tests.rs` | 19 | 任务 101 → model-protocol |
| `src/model_relay/native_gemini.rs` | 56 | 任务 101 → model-protocol |
| `src/model_relay/native_gemini_tests.rs` | 32 | 任务 101 → model-protocol |
| `src/model_relay/native_messages.rs` | 5 | 任务 101 → model-protocol |
| `src/model_relay/native_messages_tests.rs` | 16 | 任务 101 → model-protocol |
| `src/model_relay/native_responses.rs` | 118 | 任务 101 → model-protocol |
| `src/model_relay/native_responses_tests.rs` | 68 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge.rs` | 232 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge_request.rs` | 293 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge_response.rs` | 181 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge_stream.rs` | 285 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge_stream_tests.rs` | 100 | 任务 101 → model-protocol |
| `src/model_relay/protocol_bridge_tests.rs` | 125 | 任务 101 → model-protocol |
| `src/model_relay/quota_gate.rs` | 74 | 任务 106 → model-transport |
| `src/model_relay/reasoning_bridge.rs` | 164 | 任务 102 → model-protocol |
| `src/model_relay/reasoning_bridge_tests.rs` | 38 | 任务 102 → model-protocol |
| `src/model_relay/responses.rs` | 592 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_request.rs` | 312 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_request_tests.rs` | 93 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_result.rs` | 89 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_result_tests.rs` | 60 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_stream.rs` | 243 | 任务 102 → model-protocol |
| `src/model_relay/responses_anthropic_stream_tests.rs` | 26 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_request.rs` | 85 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_request_tests.rs` | 30 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_result.rs` | 109 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_result_tests.rs` | 60 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_stream.rs` | 138 | 任务 102 → model-protocol |
| `src/model_relay/responses_chat_tools.rs` | 196 | 任务 102 → model-protocol |
| `src/model_relay/responses_codex_chat.rs` | 87 | 任务 102 → model-protocol |
| `src/model_relay/responses_compaction.rs` | 150 | 任务 102 → model-protocol |
| `src/model_relay/responses_compaction_tests.rs` | 95 | 任务 102 → model-protocol |
| `src/model_relay/responses_history.rs` | 188 | 任务 103 → model-protocol |
| `src/model_relay/responses_history_tests.rs` | 58 | 任务 103 → model-protocol |
| `src/model_relay/responses_http_tests.rs` | 904 | 任务 103 → model-protocol |
| `src/model_relay/responses_stream.rs` | 663 | 任务 103 → model-protocol |
| `src/model_relay/responses_tests.rs` | 281 | 任务 103 → model-protocol |
| `src/model_relay/routing.rs` | 389 | 任务 103 → model-protocol |
| `src/model_relay/search_citations.rs` | 289 | 任务 103 → model-protocol |
| `src/model_relay/search_citations_markdown.rs` | 758 | 任务 104 → model-protocol |
| `src/model_relay/search_citations_tests.rs` | 71 | 任务 104 → model-protocol |
| `src/model_relay/server.rs` | 447 | 任务 349 → model-relay-http |
| `src/model_relay/server_tests.rs` | 490 | 任务 349 → model-relay-http |
| `src/model_relay/signature_rectify.rs` | 86 | 任务 104 → model-protocol |
| `src/model_relay/signature_rectify_tests.rs` | 29 | 任务 104 → model-protocol |
| `src/model_relay/sse_framing.rs` | 88 | 任务 104 → model-protocol |
| `src/model_relay/sse_framing_tests.rs` | 116 | 任务 104 → model-protocol |
| `src/model_relay/stream_observer.rs` | 462 | 任务 106 → model-transport |
| `src/model_relay/stream_observer_tests.rs` | 242 | 任务 106 → model-transport |
| `src/model_relay/tool_media.rs` | 525 | 任务 104 → model-protocol |
| `src/model_relay/tool_media_request.rs` | 183 | 任务 104 → model-protocol |
| `src/model_relay/tool_namespace.rs` | 99 | 任务 104 → model-protocol |
| `src/model_relay/transport.rs` | 591 | 任务 106 → model-transport |
| `src/model_relay/transport_tests.rs` | 399 | 任务 106 → model-transport |
| `src/model_relay/upstream.rs` | 73 | 任务 106 → model-transport |
| `src/model_relay/xai_oauth_transport.rs` | 37 | 任务 106 → model-transport |
| `src/model_relay/xai_responses.rs` | 534 | 任务 105 → model-protocol |
| `src/model_relay/xai_responses_tests.rs` | 80 | 任务 105 → model-protocol |
| `src/multimodal/analysis.rs` | 420 | 任务 78 → agent-multimodal |
| `src/multimodal/analysis_tests.rs` | 354 | 任务 78 → agent-multimodal |
| `src/multimodal/assets.rs` | 564 | 任务 78 → agent-multimodal |
| `src/multimodal/assets_tests.rs` | 331 | 任务 78 → agent-multimodal |
| `src/multimodal/context.rs` | 503 | 任务 78 → agent-multimodal |
| `src/multimodal/context_tests.rs` | 392 | 任务 78 → agent-multimodal |
| `src/multimodal/image_compression.rs` | 151 | 任务 78 → agent-multimodal |
| `src/multimodal/image_compression_tests.rs` | 146 | 任务 78 → agent-multimodal |
| `src/multimodal/mod.rs` | 817 | 任务 79 → agent-multimodal |
| `src/multimodal/ocr.rs` | 235 | 任务 78 → agent-multimodal |
| `src/multimodal/ocr_tests.rs` | 304 | 任务 78 → agent-multimodal |
| `src/multimodal/tests.rs` | 616 | 任务 78 → agent-multimodal |
| `src/observability/mod.rs` | 243 | 任务 45 → agent-observability |
| `src/observability/runtime_trace/mod.rs` | 399 | 任务 45 → agent-observability |
| `src/observability/runtime_trace/tests.rs` | 534 | 任务 45 → agent-observability |
| `src/observability/tests.rs` | 113 | 任务 45 → agent-observability |
| `src/observability/traits/mod.rs` | 3 | 既有 observer-types；保留薄导出 |
| `src/package_boundaries/gateway_pairing_tests.rs` | 36 | 任务 356 → agent-composition |
| `src/package_boundaries/git_discovery_tests.rs` | 20 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_lucid_tests.rs` | 37 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_mariadb_tests.rs` | 24 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_markdown_tests.rs` | 31 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_postgres_tests.rs` | 24 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_response_cache_tests.rs` | 41 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_snapshot_tests.rs` | 49 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_sqlite_tests.rs` | 32 | 任务 356 → agent-composition |
| `src/package_boundaries/memory_types_tests.rs` | 20 | 任务 356 → agent-composition |
| `src/package_boundaries/mod.rs` | 18 | 任务 356 → agent-composition |
| `src/package_boundaries/process_sandbox_tests.rs` | 25 | 任务 356 → agent-composition |
| `src/package_boundaries/project_storage_paths_tests.rs` | 32 | 任务 356 → agent-composition |
| `src/package_boundaries/secret_store_tests.rs` | 21 | 任务 356 → agent-composition |
| `src/patch/mod.rs` | 6 | 任务 356 → agent-composition |
| `src/permission/mod.rs` | 19 | 任务 63 → permission-runtime |
| `src/permission/next.rs` | 493 | 任务 63 → permission-runtime |
| `src/permission/next_tests.rs` | 292 | 任务 63 → permission-runtime |
| `src/plugins/audit.rs` | 134 | 任务 80 → agent-plugins |
| `src/plugins/audit_tests.rs` | 192 | 任务 80 → agent-plugins |
| `src/plugins/cli.rs` | 219 | 任务 80 → agent-plugins |
| `src/plugins/cli_tests.rs` | 192 | 任务 80 → agent-plugins |
| `src/plugins/compat/agents.rs` | 268 | 任务 108 → plugin-install |
| `src/plugins/compat/closure_tests.rs` | 23 | 任务 108 → plugin-install |
| `src/plugins/compat/commands.rs` | 346 | 任务 108 → plugin-install |
| `src/plugins/compat/dependencies.rs` | 390 | 任务 108 → plugin-install |
| `src/plugins/compat/discovery.rs` | 66 | 任务 108 → plugin-install |
| `src/plugins/compat/git_source.rs` | 89 | 任务 108 → plugin-install |
| `src/plugins/compat/hook_async.rs` | 70 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_command.rs` | 181 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_compact.rs` | 92 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_manifest.rs` | 149 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_matcher.rs` | 44 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_mcp.rs` | 107 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_model.rs` | 48 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_permission.rs` | 21 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_post_tool.rs` | 44 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_pre_tool.rs` | 18 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_prompt.rs` | 82 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_session.rs` | 73 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_stop.rs` | 142 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_subagent.rs` | 129 | 任务 162 → plugin-hooks |
| `src/plugins/compat/hook_trust_content.rs` | 75 | 任务 162 → plugin-hooks |
| `src/plugins/compat/host_files.rs` | 9 | 任务 108 → plugin-install |
| `src/plugins/compat/identity.rs` | 102 | 任务 108 → plugin-install |
| `src/plugins/compat/installed_state.rs` | 195 | 任务 108 → plugin-install |
| `src/plugins/compat/legacy_manifest.rs` | 179 | 任务 108 → plugin-install |
| `src/plugins/compat/marketplace.rs` | 391 | 任务 108 → plugin-install |
| `src/plugins/compat/mcp_app_execution.rs` | 140 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_apps.rs` | 61 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_auth.rs` | 460 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_capabilities.rs` | 30 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_catalog.rs` | 46 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_catalog_refresh.rs` | 182 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_connection_sessions.rs` | 49 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_connection_settings.rs` | 161 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_connections.rs` | 417 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_content.rs` | 238 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_elicitation.rs` | 310 | 任务 163 → plugin-mcp |
| `src/plugins/compat/mcp_environment.rs` | 322 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_legacy.rs` | 234 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_notifications.rs` | 221 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_oauth_discovery.rs` | 162 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_oauth_flow.rs` | 389 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_oauth_tokens.rs` | 312 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_portable.rs` | 217 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_portable_overlay.rs` | 111 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_public_auth.rs` | 195 | 任务 164 → plugin-mcp |
| `src/plugins/compat/mcp_resources.rs` | 334 | 任务 165 → plugin-mcp |
| `src/plugins/compat/mod.rs` | 272 | 任务 109 → plugin-install |
| `src/plugins/compat/npm_archive.rs` | 121 | 任务 109 → plugin-install |
| `src/plugins/compat/npm_source.rs` | 196 | 任务 109 → plugin-install |
| `src/plugins/compat/onboarding.rs` | 42 | 任务 109 → plugin-install |
| `src/plugins/compat/path_audit.rs` | 34 | 任务 109 → plugin-install |
| `src/plugins/compat/platform_tests.rs` | 42 | 任务 109 → plugin-install |
| `src/plugins/compat/plugin_window_fixture.rs` | 114 | 任务 109 → plugin-install |
| `src/plugins/compat/portable_manifest.rs` | 228 | 任务 109 → plugin-install |
| `src/plugins/compat/session_snapshot.rs` | 107 | 任务 109 → plugin-install |
| `src/plugins/compat/skill_frontmatter.rs` | 133 | 任务 109 → plugin-install |
| `src/plugins/compat/skill_invocation.rs` | 81 | 任务 109 → plugin-install |
| `src/plugins/compat/skill_roots.rs` | 95 | 任务 109 → plugin-install |
| `src/plugins/compat/t02_tests.rs` | 89 | 任务 109 → plugin-install |
| `src/plugins/compat/t03_tests.rs` | 267 | 任务 109 → plugin-install |
| `src/plugins/compat/t04_tests.rs` | 296 | 任务 109 → plugin-install |
| `src/plugins/compat/t05_tests.rs` | 313 | 任务 109 → plugin-install |
| `src/plugins/compat/t06_tests.rs` | 291 | 任务 109 → plugin-install |
| `src/plugins/compat/t07_tests.rs` | 331 | 任务 109 → plugin-install |
| `src/plugins/compat/t08_tests.rs` | 334 | 任务 109 → plugin-install |
| `src/plugins/compat/t09_tests.rs` | 360 | 任务 109 → plugin-install |
| `src/plugins/compat/t10_tests.rs` | 370 | 任务 109 → plugin-install |
| `src/plugins/compat/t11_tests.rs` | 491 | 任务 109 → plugin-install |
| `src/plugins/compat/t12_tests.rs` | 330 | 任务 109 → plugin-install |
| `src/plugins/compat/t13_tests.rs` | 410 | 任务 109 → plugin-install |
| `src/plugins/compat/t14_tests.rs` | 358 | 任务 109 → plugin-install |
| `src/plugins/compat/t15_tests.rs` | 366 | 任务 109 → plugin-install |
| `src/plugins/compat/t16_tests.rs` | 300 | 任务 109 → plugin-install |
| `src/plugins/compat/t17_public_tests.rs` | 115 | 任务 109 → plugin-install |
| `src/plugins/compat/t17_tests.rs` | 426 | 任务 109 → plugin-install |
| `src/plugins/compat/t18_tests.rs` | 437 | 任务 109 → plugin-install |
| `src/plugins/compat/t19_tests.rs` | 204 | 任务 109 → plugin-install |
| `src/plugins/compat/t20_tests.rs` | 516 | 任务 109 → plugin-install |
| `src/plugins/compat/t21_tests.rs` | 278 | 任务 109 → plugin-install |
| `src/plugins/compat/t22_tests.rs` | 362 | 任务 109 → plugin-install |
| `src/plugins/compat/t23_tests.rs` | 191 | 任务 109 → plugin-install |
| `src/plugins/compat/t24_tests.rs` | 205 | 任务 109 → plugin-install |
| `src/plugins/compat/t26_tests.rs` | 90 | 任务 109 → plugin-install |
| `src/plugins/compat/t27_flow_tests.rs` | 381 | 任务 109 → plugin-install |
| `src/plugins/compat/t27_tests.rs` | 149 | 任务 109 → plugin-install |
| `src/plugins/compat/t28_tests.rs` | 238 | 任务 109 → plugin-install |
| `src/plugins/compat/t29_tests.rs` | 149 | 任务 109 → plugin-install |
| `src/plugins/compat/t30_tests.rs` | 125 | 任务 109 → plugin-install |
| `src/plugins/compat/t32_tests.rs` | 285 | 任务 109 → plugin-install |
| `src/plugins/compat/t33_tests.rs` | 112 | 任务 109 → plugin-install |
| `src/plugins/compat/t34_tests.rs` | 284 | 任务 109 → plugin-install |
| `src/plugins/compat/t35_tests.rs` | 225 | 任务 109 → plugin-install |
| `src/plugins/compat/t36_tests.rs` | 83 | 任务 109 → plugin-install |
| `src/plugins/compat/t37_tests.rs` | 141 | 任务 109 → plugin-install |
| `src/plugins/compat/t38_tests.rs` | 152 | 任务 109 → plugin-install |
| `src/plugins/compat/t39_tests.rs` | 96 | 任务 109 → plugin-install |
| `src/plugins/compat/t40_tests.rs` | 118 | 任务 109 → plugin-install |
| `src/plugins/compat/t41_tests.rs` | 97 | 任务 109 → plugin-install |
| `src/plugins/compat/t42_tests.rs` | 149 | 任务 109 → plugin-install |
| `src/plugins/compat/t43_tests.rs` | 276 | 任务 109 → plugin-install |
| `src/plugins/compat/t44_tests.rs` | 188 | 任务 109 → plugin-install |
| `src/plugins/compat/t45_tests.rs` | 139 | 任务 109 → plugin-install |
| `src/plugins/compat/t46_tests.rs` | 41 | 任务 109 → plugin-install |
| `src/plugins/compat/t47_tests.rs` | 90 | 任务 109 → plugin-install |
| `src/plugins/compat/t48_tests.rs` | 117 | 任务 109 → plugin-install |
| `src/plugins/compat/t49_tests.rs` | 90 | 任务 109 → plugin-install |
| `src/plugins/compat/t50_tests.rs` | 110 | 任务 109 → plugin-install |
| `src/plugins/compat/t51_tests.rs` | 92 | 任务 109 → plugin-install |
| `src/plugins/compat/t52_tests.rs` | 171 | 任务 109 → plugin-install |
| `src/plugins/compat/t53_tests.rs` | 80 | 任务 109 → plugin-install |
| `src/plugins/compat/t54_tests.rs` | 194 | 任务 109 → plugin-install |
| `src/plugins/compat/t57_tests.rs` | 356 | 任务 109 → plugin-install |
| `src/plugins/compat/t58_tests.rs` | 69 | 任务 109 → plugin-install |
| `src/plugins/compat/t59_tests.rs` | 135 | 任务 109 → plugin-install |
| `src/plugins/compat/t60_tests.rs` | 42 | 任务 109 → plugin-install |
| `src/plugins/compat/t61_tests.rs` | 429 | 任务 109 → plugin-install |
| `src/plugins/compat/t62_tests.rs` | 102 | 任务 109 → plugin-install |
| `src/plugins/compat/t64_tests.rs` | 115 | 任务 109 → plugin-install |
| `src/plugins/compat/t66_tests.rs` | 74 | 任务 109 → plugin-install |
| `src/plugins/compat/transaction_install.rs` | 243 | 任务 109 → plugin-install |
| `src/plugins/compat/upgrade.rs` | 180 | 任务 109 → plugin-install |
| `src/plugins/hook_handler.rs` | 1024 | 任务 80 → agent-plugins |
| `src/plugins/hook_handler_tests.rs` | 506 | 任务 80 → agent-plugins |
| `src/plugins/hook_trust.rs` | 160 | 任务 80 → agent-plugins |
| `src/plugins/hook_trust_tests.rs` | 145 | 任务 80 → agent-plugins |
| `src/plugins/hooks.rs` | 274 | 任务 80 → agent-plugins |
| `src/plugins/hooks_tests.rs` | 353 | 任务 80 → agent-plugins |
| `src/plugins/install.rs` | 390 | 任务 81 → agent-plugins |
| `src/plugins/install_tests.rs` | 406 | 任务 81 → agent-plugins |
| `src/plugins/loader.rs` | 258 | 任务 81 → agent-plugins |
| `src/plugins/loader_tests.rs` | 253 | 任务 81 → agent-plugins |
| `src/plugins/manifest.rs` | 221 | 任务 81 → agent-plugins |
| `src/plugins/manifest_tests.rs` | 204 | 任务 81 → agent-plugins |
| `src/plugins/mcp.rs` | 214 | 任务 81 → agent-plugins |
| `src/plugins/mcp_tests.rs` | 267 | 任务 81 → agent-plugins |
| `src/plugins/mod.rs` | 165 | 任务 81 → agent-plugins |
| `src/plugins/skills.rs` | 304 | 任务 81 → agent-plugins |
| `src/plugins/skills_tests.rs` | 199 | 任务 81 → agent-plugins |
| `src/plugins/types.rs` | 6 | 既有 plugin-types；保留薄导出 |
| `src/project/core.rs` | 419 | 任务 46 → agent-project |
| `src/project/core_tests.rs` | 503 | 任务 46 → agent-project |
| `src/project/icon_discovery.rs` | 109 | 任务 46 → agent-project |
| `src/project/icon_discovery_tests.rs` | 205 | 任务 46 → agent-project |
| `src/project/instance.rs` | 194 | 任务 46 → agent-project |
| `src/project/instance_tests.rs` | 186 | 任务 46 → agent-project |
| `src/project/migration.rs` | 105 | 任务 46 → agent-project |
| `src/project/migration_tests.rs` | 280 | 任务 46 → agent-project |
| `src/project/mod.rs` | 216 | 任务 46 → agent-project |
| `src/project/state.rs` | 121 | 任务 46 → agent-project |
| `src/project/state_tests.rs` | 365 | 任务 46 → agent-project |
| `src/project/store/postgres.rs` | 375 | 任务 46 → agent-project |
| `src/project/store/postgres_tests.rs` | 43 | 任务 46 → agent-project |
| `src/project/store.rs` | 521 | 任务 46 → agent-project |
| `src/project/store_tests.rs` | 445 | 任务 46 → agent-project |
| `src/project/tests.rs` | 139 | 任务 46 → agent-project |
| `src/project/vcs.rs` | 102 | 任务 46 → agent-project |
| `src/project/vcs_tests.rs` | 311 | 任务 46 → agent-project |
| `src/provider/auth.rs` | 308 | 任务 82 → agent-provider |
| `src/provider/auth_tests.rs` | 132 | 任务 82 → agent-provider |
| `src/provider/error.rs` | 213 | 任务 82 → agent-provider |
| `src/provider/error_tests.rs` | 17 | 任务 82 → agent-provider |
| `src/provider/mod.rs` | 43 | 任务 83 → agent-provider |
| `src/provider/model_catalog.rs` | 210 | 任务 82 → agent-provider |
| `src/provider/model_catalog_tests.rs` | 453 | 任务 82 → agent-provider |
| `src/provider/models.rs` | 24 | 任务 82 → agent-provider |
| `src/provider/models_tests.rs` | 9 | 任务 82 → agent-provider |
| `src/provider/provider.rs` | 795 | 任务 82 → agent-provider |
| `src/provider/provider_tests.rs` | 636 | 任务 82 → agent-provider |
| `src/provider/tests.rs` | 7 | 任务 82 → agent-provider |
| `src/provider/transform.rs` | 412 | 任务 82 → agent-provider |
| `src/provider/transform_tests.rs` | 248 | 任务 82 → agent-provider |
| `src/provider/voice/mod.rs` | 117 | 任务 83 → agent-provider |
| `src/provider/voice/realtime.rs` | 517 | 任务 83 → agent-provider |
| `src/provider/voice/realtime_events.rs` | 244 | 任务 83 → agent-provider |
| `src/provider/voice/realtime_handoff.rs` | 219 | 任务 83 → agent-provider |
| `src/provider/voice/realtime_handoff_tests.rs` | 149 | 任务 83 → agent-provider |
| `src/provider/voice/realtime_preview_tests.rs` | 65 | 任务 83 → agent-provider |
| `src/provider/voice/realtime_tests.rs` | 401 | 任务 83 → agent-provider |
| `src/provider/voice/transcription.rs` | 243 | 任务 83 → agent-provider |
| `src/provider/voice/transcription_tests.rs` | 321 | 任务 83 → agent-provider |
| `src/providers/mod.rs` | 686 | 任务 142 → agent-providers |
| `src/providers/native_search_capability_tests.rs` | 50 | 任务 142 → agent-providers |
| `src/providers/session_llm.rs` | 492 | 任务 142 → agent-providers |
| `src/providers/session_llm_output.rs` | 84 | 任务 142 → agent-providers |
| `src/providers/session_llm_tests.rs` | 454 | 任务 142 → agent-providers |
| `src/providers/tests.rs` | 173 | 任务 142 → agent-providers |
| `src/providers/traits/mod.rs` | 457 | 任务 24 → model-types |
| `src/providers/traits/tests.rs` | 929 | 任务 24 → model-types |
| `src/pty/mod.rs` | 9 | 任务 47 → agent-pty |
| `src/pty/pty.rs` | 823 | 任务 47 → agent-pty |
| `src/pty/pty_tests.rs` | 240 | 任务 47 → agent-pty |
| `src/question/mod.rs` | 275 | 任务 84 → agent-question |
| `src/question/mod_tests.rs` | 368 | 任务 84 → agent-question |
| `src/runtime/docker/mod.rs` | 311 | 任务 48 → agent-runtime |
| `src/runtime/docker/tests.rs` | 277 | 任务 48 → agent-runtime |
| `src/runtime/mod.rs` | 100 | 任务 48 → agent-runtime |
| `src/runtime/native/mod.rs` | 620 | 任务 48 → agent-runtime |
| `src/runtime/native/shell_tool_tests.rs` | 31 | 任务 48 → agent-runtime |
| `src/runtime/native/tests.rs` | 683 | 任务 48 → agent-runtime |
| `src/runtime/native/windows_commands_tests.rs` | 209 | 任务 48 → agent-runtime |
| `src/runtime/tests.rs` | 146 | 任务 48 → agent-runtime |
| `src/runtime/traits/mod.rs` | 178 | 任务 25 → runtime-types |
| `src/runtime/traits/tests.rs` | 192 | 任务 25 → runtime-types |
| `src/scheduler/mod.rs` | 483 | 任务 143 → agent-scheduler-core |
| `src/scheduler/tests.rs` | 246 | 任务 143 → agent-scheduler-core |
| `src/security/bubblewrap/mod.rs` | 3 | 任务 49 → agent-security |
| `src/security/canary_guard/mod.rs` | 3 | 既有 canary-guard；保留薄导出 |
| `src/security/detect/mod.rs` | 264 | 任务 49 → agent-security |
| `src/security/detect/tests.rs` | 151 | 任务 49 → agent-security |
| `src/security/docker/mod.rs` | 3 | 任务 49 → agent-security |
| `src/security/domain_matcher/mod.rs` | 6 | 任务 49 → agent-security |
| `src/security/domain_matcher/reexport_tests.rs` | 10 | 任务 49 → agent-security |
| `src/security/estop/mod.rs` | 696 | 任务 49 → agent-security |
| `src/security/estop/tests.rs` | 499 | 任务 49 → agent-security |
| `src/security/firejail/mod.rs` | 3 | 任务 49 → agent-security |
| `src/security/gateway_skey_store/postgres.rs` | 128 | 任务 49 → agent-security |
| `src/security/gateway_skey_store/postgres_tests.rs` | 64 | 任务 49 → agent-security |
| `src/security/gateway_skey_store.rs` | 237 | 任务 49 → agent-security |
| `src/security/gateway_skey_store_tests.rs` | 247 | 任务 49 → agent-security |
| `src/security/landlock/mod.rs` | 3 | 任务 49 → agent-security |
| `src/security/leak_detector.rs` | 5 | 任务 49 → agent-security |
| `src/security/leak_detector_reexport_tests.rs` | 18 | 任务 49 → agent-security |
| `src/security/mod.rs` | 223 | 任务 56 → agent-security |
| `src/security/otp/mod.rs` | 685 | 任务 49 → agent-security |
| `src/security/otp/tests.rs` | 353 | 任务 49 → agent-security |
| `src/security/pairing.rs` | 3 | 任务 49 → agent-security |
| `src/security/policy/action_tracker.rs` | 210 | 任务 50 → agent-security |
| `src/security/policy/action_tracker_tests.rs` | 36 | 任务 50 → agent-security |
| `src/security/policy/allowlist.rs` | 188 | 任务 50 → agent-security |
| `src/security/policy/allowlist_tests.rs` | 161 | 任务 50 → agent-security |
| `src/security/policy/mod.rs` | 769 | 任务 51 → agent-security |
| `src/security/policy/path_utils.rs` | 126 | 任务 50 → agent-security |
| `src/security/policy/path_utils_tests.rs` | 63 | 任务 50 → agent-security |
| `src/security/policy/risk.rs` | 231 | 任务 50 → agent-security |
| `src/security/policy/risk_tests.rs` | 61 | 任务 50 → agent-security |
| `src/security/policy/shell_lexer.rs` | 536 | 任务 50 → agent-security |
| `src/security/policy/shell_lexer_tests.rs` | 152 | 任务 50 → agent-security |
| `src/security/policy/shell_redirect.rs` | 469 | 任务 50 → agent-security |
| `src/security/policy/shell_redirect_tests.rs` | 134 | 任务 50 → agent-security |
| `src/security/policy/tests.rs` | 443 | 任务 50 → agent-security |
| `src/security/policy/types.rs` | 59 | 任务 50 → agent-security |
| `src/security/policy/types_tests.rs` | 27 | 任务 50 → agent-security |
| `src/security/prompt_guard.rs` | 5 | 任务 52 → agent-security |
| `src/security/prompt_guard_reexport_tests.rs` | 15 | 任务 52 → agent-security |
| `src/security/secrets.rs` | 3 | 任务 52 → agent-security |
| `src/security/semantic_guard.rs` | 838 | 任务 52 → agent-security |
| `src/security/semantic_guard_tests.rs` | 489 | 任务 52 → agent-security |
| `src/security/syscall_anomaly.rs` | 3 | 既有 syscall-anomaly；保留薄导出 |
| `src/security/tests/bubblewrap.rs` | 92 | 任务 53 → agent-security |
| `src/security/tests/detect.rs` | 39 | 任务 53 → agent-security |
| `src/security/tests/docker.rs` | 93 | 任务 53 → agent-security |
| `src/security/tests/domain_matcher.rs` | 53 | 任务 53 → agent-security |
| `src/security/tests/estop.rs` | 108 | 任务 53 → agent-security |
| `src/security/tests/firejail.rs` | 102 | 任务 53 → agent-security |
| `src/security/tests/landlock.rs` | 59 | 任务 53 → agent-security |
| `src/security/tests/leak_detector.rs` | 95 | 任务 53 → agent-security |
| `src/security/tests/otp.rs` | 78 | 任务 53 → agent-security |
| `src/security/tests/pairing.rs` | 359 | 任务 53 → agent-security |
| `src/security/tests/policy/autonomy.rs` | 85 | 任务 54 → agent-security |
| `src/security/tests/policy/command_policy.rs` | 203 | 任务 54 → agent-security |
| `src/security/tests/policy/config.rs` | 145 | 任务 54 → agent-security |
| `src/security/tests/policy/mod.rs` | 34 | 任务 54 → agent-security |
| `src/security/tests/policy/path_policy.rs` | 471 | 任务 54 → agent-security |
| `src/security/tests/policy/shell_safety.rs` | 351 | 任务 54 → agent-security |
| `src/security/tests/policy/tracker.rs` | 86 | 任务 54 → agent-security |
| `src/security/tests/prompt_guard.rs` | 75 | 任务 54 → agent-security |
| `src/security/tests/secrets.rs` | 541 | 任务 54 → agent-security |
| `src/security/tests/semantic_guard.rs` | 106 | 任务 54 → agent-security |
| `src/security/tests/traits.rs` | 35 | 任务 55 → agent-security |
| `src/security/tests.rs` | 28 | 任务 56 → agent-security |
| `src/security/traits.rs` | 3 | 任务 56 → agent-security |
| `src/service/common.rs` | 73 | 任务 354 → agent-service |
| `src/service/common_tests.rs` | 218 | 任务 354 → agent-service |
| `src/service/linux.rs` | 229 | 任务 354 → agent-service |
| `src/service/linux_tests.rs` | 119 | 任务 354 → agent-service |
| `src/service/macos.rs` | 151 | 任务 354 → agent-service |
| `src/service/macos_tests.rs` | 159 | 任务 354 → agent-service |
| `src/service/mod.rs` | 262 | 任务 354 → agent-service |
| `src/service/mod_tests.rs` | 84 | 任务 354 → agent-service |
| `src/service/openrc.rs` | 619 | 任务 354 → agent-service |
| `src/service/openrc_tests.rs` | 348 | 任务 354 → agent-service |
| `src/service/tests.rs` | 438 | 任务 354 → agent-service |
| `src/service/windows.rs` | 160 | 任务 354 → agent-service |
| `src/service/windows_tests.rs` | 202 | 任务 354 → agent-service |
| `src/session/README.md` | 35 | 任务 204 → session-engine |
| `src/session/agent_binding.rs` | 458 | 任务 204 → session-engine |
| `src/session/agent_binding_tests.rs` | 174 | 任务 204 → session-engine |
| `src/session/capability_policy.rs` | 223 | 任务 204 → session-engine |
| `src/session/capability_policy_tests.rs` | 353 | 任务 204 → session-engine |
| `src/session/child_agent/collaboration_tests.rs` | 228 | 任务 204 → session-engine |
| `src/session/child_agent/continuation.rs` | 76 | 任务 204 → session-engine |
| `src/session/child_agent/continuation_tests.rs` | 104 | 任务 204 → session-engine |
| `src/session/child_agent/coordinator.rs` | 866 | 任务 204 → session-engine |
| `src/session/child_agent/coordinator_tests.rs` | 330 | 任务 204 → session-engine |
| `src/session/child_agent/delivery.rs` | 55 | 任务 204 → session-engine |
| `src/session/child_agent/delivery_tests.rs` | 38 | 任务 204 → session-engine |
| `src/session/child_agent/entry_tests.rs` | 257 | 任务 204 → session-engine |
| `src/session/child_agent/history.rs` | 61 | 任务 204 → session-engine |
| `src/session/child_agent/history_tests.rs` | 30 | 任务 204 → session-engine |
| `src/session/child_agent/mailbox.rs` | 214 | 任务 204 → session-engine |
| `src/session/child_agent/mailbox_tests.rs` | 153 | 任务 204 → session-engine |
| `src/session/child_agent.rs` | 763 | 任务 205 → session-engine |
| `src/session/child_agent_tests.rs` | 704 | 任务 205 → session-engine |
| `src/session/claude_protocol_parity_tests.rs` | 109 | 任务 205 → session-engine |
| `src/session/codex_alignment_tests.rs` | 361 | 任务 205 → session-engine |
| `src/session/compaction.rs` | 724 | 任务 205 → session-engine |
| `src/session/compaction_checkpoint.rs` | 174 | 任务 205 → session-engine |
| `src/session/compaction_checkpoint_tests.rs` | 150 | 任务 205 → session-engine |
| `src/session/compaction_summary.rs` | 158 | 任务 205 → session-engine |
| `src/session/compaction_summary_input.rs` | 132 | 任务 205 → session-engine |
| `src/session/compaction_summary_tests.rs` | 402 | 任务 205 → session-engine |
| `src/session/compaction_tests.rs` | 1011 | 任务 205 → session-engine |
| `src/session/context_injection.rs` | 84 | 任务 205 → session-engine |
| `src/session/context_injection_tests.rs` | 63 | 任务 205 → session-engine |
| `src/session/dynamic_tool_acceptance_tests.rs` | 495 | 任务 205 → session-engine |
| `src/session/dynamic_tool_store.rs` | 166 | 任务 111 → session-store |
| `src/session/dynamic_tool_store_tests.rs` | 442 | 任务 111 → session-store |
| `src/session/fork_execution.rs` | 86 | 任务 205 → session-engine |
| `src/session/fork_execution_tests.rs` | 51 | 任务 205 → session-engine |
| `src/session/instruction.rs` | 480 | 任务 110 → session-instructions |
| `src/session/instruction_cache.rs` | 195 | 任务 110 → session-instructions |
| `src/session/instruction_cache_tests.rs` | 113 | 任务 110 → session-instructions |
| `src/session/instruction_discovery.rs` | 182 | 任务 110 → session-instructions |
| `src/session/instruction_discovery_tests.rs` | 83 | 任务 110 → session-instructions |
| `src/session/instruction_include.rs` | 163 | 任务 110 → session-instructions |
| `src/session/instruction_include_tests.rs` | 62 | 任务 110 → session-instructions |
| `src/session/instruction_rules.rs` | 71 | 任务 110 → session-instructions |
| `src/session/instruction_rules_tests.rs` | 51 | 任务 110 → session-instructions |
| `src/session/instruction_tests.rs` | 732 | 任务 110 → session-instructions |
| `src/session/llm/acp/approval.rs` | 104 | 任务 151 → llm-runtime |
| `src/session/llm/acp/config.rs` | 500 | 任务 151 → llm-runtime |
| `src/session/llm/acp/config_tests.rs` | 512 | 任务 151 → llm-runtime |
| `src/session/llm/acp/content.rs` | 81 | 任务 151 → llm-runtime |
| `src/session/llm/acp/content_tests.rs` | 84 | 任务 151 → llm-runtime |
| `src/session/llm/acp/controls.rs` | 590 | 任务 151 → llm-runtime |
| `src/session/llm/acp/controls_tests.rs` | 291 | 任务 151 → llm-runtime |
| `src/session/llm/acp/fork_replay.rs` | 112 | 任务 151 → llm-runtime |
| `src/session/llm/acp/fork_replay_tests.rs` | 83 | 任务 151 → llm-runtime |
| `src/session/llm/acp/lifecycle.rs` | 100 | 任务 151 → llm-runtime |
| `src/session/llm/acp/lifecycle_tests.rs` | 42 | 任务 151 → llm-runtime |
| `src/session/llm/acp/mod.rs` | 175 | 任务 157 → llm-runtime |
| `src/session/llm/acp/model_source/acceptance_tests.rs` | 639 | 任务 152 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/amp_acp.rs` | 84 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/amp_acp_tests.rs` | 53 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/auggie.rs` | 100 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/auggie_tests.rs` | 40 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/claude.rs` | 168 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/claude_tests.rs` | 372 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/codebuddy.rs` | 66 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/codebuddy_tests.rs` | 95 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/codex.rs` | 125 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/codex_tests.rs` | 676 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/copilot.rs` | 147 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/copilot_tests.rs` | 121 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/cursor.rs` | 96 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/cursor_tests.rs` | 40 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/dirac.rs` | 80 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/dirac_tests.rs` | 52 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/drivers_tests.rs` | 204 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/droid.rs` | 81 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/droid_tests.rs` | 55 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/gemini.rs` | 180 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/gemini_tests.rs` | 180 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/glm_acp_agent.rs` | 79 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/glm_acp_agent_tests.rs` | 52 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/goose.rs` | 55 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/goose_tests.rs` | 175 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/grok.rs` | 118 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/grok_tests.rs` | 73 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/hermes.rs` | 122 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/hermes_tests.rs` | 264 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/iflow.rs` | 99 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/iflow_tests.rs` | 123 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kilocode.rs` | 177 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kilocode_tests.rs` | 174 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kimi.rs` | 141 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kimi_tests.rs` | 150 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kiro.rs` | 146 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/kiro_tests.rs` | 218 | 任务 153 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/mod.rs` | 221 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/openclaw.rs` | 146 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/openclaw_tests.rs` | 168 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/opencode.rs` | 229 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/opencode_tests.rs` | 275 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/pi.rs` | 213 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/pi_tests.rs` | 258 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/qoder.rs` | 74 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/qoder_tests.rs` | 26 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/qwen.rs` | 163 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/qwen_tests.rs` | 92 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/sigit.rs` | 77 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/sigit_tests.rs` | 51 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/snow.rs` | 56 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/snow_tests.rs` | 90 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/trae.rs` | 63 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/trae_tests.rs` | 31 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/vibe.rs` | 93 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/drivers/vibe_tests.rs` | 116 | 任务 154 → llm-runtime |
| `src/session/llm/acp/model_source/gateway.rs` | 69 | 任务 155 → llm-runtime |
| `src/session/llm/acp/model_source/gateway_tests.rs` | 46 | 任务 155 → llm-runtime |
| `src/session/llm/acp/model_source/mod.rs` | 99 | 任务 155 → llm-runtime |
| `src/session/llm/acp/model_source/profile.rs` | 210 | 任务 155 → llm-runtime |
| `src/session/llm/acp/model_source/profile_tests.rs` | 198 | 任务 155 → llm-runtime |
| `src/session/llm/acp/recovery.rs` | 138 | 任务 156 → llm-runtime |
| `src/session/llm/acp/recovery_tests.rs` | 81 | 任务 156 → llm-runtime |
| `src/session/llm/acp/replay.rs` | 403 | 任务 156 → llm-runtime |
| `src/session/llm/acp/replay_tests.rs` | 487 | 任务 156 → llm-runtime |
| `src/session/llm/acp/request.rs` | 1020 | 任务 156 → llm-runtime |
| `src/session/llm/acp/request_tests.rs` | 648 | 任务 156 → llm-runtime |
| `src/session/llm/acp/session.rs` | 567 | 任务 156 → llm-runtime |
| `src/session/llm/acp/session_tests.rs` | 594 | 任务 156 → llm-runtime |
| `src/session/llm/acp/skill_mounts/acceptance_tests.rs` | 471 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/auggie.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/auggie_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/claude.rs` | 17 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/claude_tests.rs` | 65 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/codebuddy.rs` | 32 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/codebuddy_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/codex.rs` | 8 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/codex_tests.rs` | 35 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/copilot.rs` | 34 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/copilot_tests.rs` | 43 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/cursor.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/cursor_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/droid.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/droid_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/gemini.rs` | 30 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/gemini_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/goose.rs` | 47 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/goose_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/hermes.rs` | 30 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/hermes_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/iflow.rs` | 30 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/iflow_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kilocode.rs` | 30 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kilocode_tests.rs` | 43 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kimi.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kimi_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kiro.rs` | 31 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/kiro_tests.rs` | 43 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/lifecycle_tests.rs` | 109 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/mod.rs` | 169 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/openclaw.rs` | 28 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/openclaw_tests.rs` | 43 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/opencode.rs` | 31 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/opencode_tests.rs` | 43 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/outbound_tests.rs` | 18 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/pi.rs` | 26 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/pi_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/qoder.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/qoder_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/qwen.rs` | 28 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/qwen_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/snow.rs` | 28 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/snow_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/trae.rs` | 7 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/trae_tests.rs` | 5 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/vibe.rs` | 28 | 任务 157 → llm-runtime |
| `src/session/llm/acp/skill_mounts/vibe_tests.rs` | 39 | 任务 157 → llm-runtime |
| `src/session/llm/acp/source_selection.rs` | 93 | 任务 157 → llm-runtime |
| `src/session/llm/acp/tests.rs` | 151 | 任务 157 → llm-runtime |
| `src/session/llm/acp/updates.rs` | 496 | 任务 157 → llm-runtime |
| `src/session/llm/acp/updates_tests.rs` | 821 | 任务 157 → llm-runtime |
| `src/session/llm/aisdk/convert.rs` | 461 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/convert_tests.rs` | 666 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/driver.rs` | 245 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/driver_tests.rs` | 518 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/error.rs` | 216 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/error_tests.rs` | 284 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/mod.rs` | 26 | 任务 159 → llm-runtime |
| `src/session/llm/aisdk/model_catalog_tests.rs` | 294 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/native_search_acceptance_tests.rs` | 617 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/reasoning.rs` | 96 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/reasoning_tests.rs` | 176 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/request.rs` | 282 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/request_tests.rs` | 516 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/stream.rs` | 633 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/stream_tests.rs` | 1133 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/truncation_tests.rs` | 117 | 任务 158 → llm-runtime |
| `src/session/llm/aisdk/util.rs` | 540 | 任务 159 → llm-runtime |
| `src/session/llm/aisdk/util_tests.rs` | 806 | 任务 159 → llm-runtime |
| `src/session/llm/logging.rs` | 27 | 任务 160 → llm-runtime |
| `src/session/llm/logging_tests.rs` | 25 | 任务 160 → llm-runtime |
| `src/session/llm/messages.rs` | 437 | 任务 160 → llm-runtime |
| `src/session/llm/messages_tests.rs` | 538 | 任务 160 → llm-runtime |
| `src/session/llm/mod.rs` | 86 | 任务 161 → llm-runtime |
| `src/session/llm/model_selector.rs` | 310 | 任务 160 → llm-runtime |
| `src/session/llm/model_selector_tests.rs` | 289 | 任务 160 → llm-runtime |
| `src/session/llm/native_tools.rs` | 223 | 任务 160 → llm-runtime |
| `src/session/llm/native_tools_tests.rs` | 209 | 任务 160 → llm-runtime |
| `src/session/llm/options.rs` | 31 | 任务 160 → llm-runtime |
| `src/session/llm/options_tests.rs` | 144 | 任务 160 → llm-runtime |
| `src/session/llm/prompt.rs` | 1096 | 任务 160 → llm-runtime |
| `src/session/llm/prompt_tests.rs` | 584 | 任务 160 → llm-runtime |
| `src/session/llm/recovery_test_support.rs` | 62 | 任务 160 → llm-runtime |
| `src/session/llm/recovery_test_support_tests.rs` | 32 | 任务 160 → llm-runtime |
| `src/session/llm/stream.rs` | 839 | 任务 161 → llm-runtime |
| `src/session/llm/stream_tests.rs` | 727 | 任务 161 → llm-runtime |
| `src/session/llm/tests.rs` | 346 | 任务 161 → llm-runtime |
| `src/session/llm/transport_tests.rs` | 194 | 任务 161 → llm-runtime |
| `src/session/llm/types.rs` | 264 | 任务 161 → llm-runtime |
| `src/session/llm/types_tests.rs` | 337 | 任务 161 → llm-runtime |
| `src/session/llm_raw.rs` | 283 | 任务 206 → session-engine |
| `src/session/llm_raw_tests.rs` | 130 | 任务 206 → session-engine |
| `src/session/message.rs` | 255 | 任务 206 → session-engine |
| `src/session/message_tests.rs` | 274 | 任务 206 → session-engine |
| `src/session/mod.rs` | 87 | 任务 216 → session-engine |
| `src/session/native_defaults_mcp_fixture.py` | 60 | 任务 206 → session-engine |
| `src/session/processor/artifacts.rs` | 248 | 任务 207 → session-engine |
| `src/session/processor/artifacts_tests.rs` | 561 | 任务 207 → session-engine |
| `src/session/processor/context_governance.rs` | 332 | 任务 207 → session-engine |
| `src/session/processor/context_governance_tests.rs` | 358 | 任务 207 → session-engine |
| `src/session/processor/helpers.rs` | 106 | 任务 207 → session-engine |
| `src/session/processor/helpers_tests.rs` | 351 | 任务 207 → session-engine |
| `src/session/processor/llm_messages.rs` | 299 | 任务 207 → session-engine |
| `src/session/processor/llm_messages_tests.rs` | 213 | 任务 207 → session-engine |
| `src/session/processor/llm_runner.rs` | 790 | 任务 207 → session-engine |
| `src/session/processor/llm_runner_tests.rs` | 1803 | 任务 207 → session-engine |
| `src/session/processor/mcp_catalog_refresh_tests.rs` | 69 | 任务 207 → session-engine |
| `src/session/processor/mod.rs` | 78 | 任务 212 → session-engine |
| `src/session/processor/planning.rs` | 327 | 任务 207 → session-engine |
| `src/session/processor/planning_tests.rs` | 336 | 任务 207 → session-engine |
| `src/session/processor/prefetch.rs` | 374 | 任务 208 → session-engine |
| `src/session/processor/prefetch_tests.rs` | 224 | 任务 208 → session-engine |
| `src/session/processor/prompting.rs` | 69 | 任务 208 → session-engine |
| `src/session/processor/prompting_tests.rs` | 164 | 任务 208 → session-engine |
| `src/session/processor/recovery.rs` | 226 | 任务 208 → session-engine |
| `src/session/processor/recovery_tests.rs` | 98 | 任务 208 → session-engine |
| `src/session/processor/runner.rs` | 2255 | 任务 209 → session-engine |
| `src/session/processor/runner_tests.rs` | 3456 | 任务 210 → session-engine |
| `src/session/processor/tests.rs` | 479 | 任务 210 → session-engine |
| `src/session/processor/todo_updates.rs` | 181 | 任务 210 → session-engine |
| `src/session/processor/todo_updates_tests.rs` | 304 | 任务 210 → session-engine |
| `src/session/processor/todos.rs` | 200 | 任务 210 → session-engine |
| `src/session/processor/todos_tests.rs` | 183 | 任务 210 → session-engine |
| `src/session/processor/tools_exec.rs` | 1893 | 任务 211 → session-engine |
| `src/session/processor/tools_exec_tests.rs` | 2009 | 任务 211 → session-engine |
| `src/session/processor/turn_policy.rs` | 41 | 任务 211 → session-engine |
| `src/session/processor/turn_policy_tests.rs` | 60 | 任务 211 → session-engine |
| `src/session/processor/types.rs` | 264 | 任务 211 → session-engine |
| `src/session/processor/types_tests.rs` | 59 | 任务 211 → session-engine |
| `src/session/processor/utils/docs.rs` | 56 | 任务 212 → session-engine |
| `src/session/processor/utils/docs_tests.rs` | 135 | 任务 212 → session-engine |
| `src/session/processor/utils/file_links.rs` | 170 | 任务 212 → session-engine |
| `src/session/processor/utils/file_links_tests.rs` | 245 | 任务 212 → session-engine |
| `src/session/processor/utils/message_format.rs` | 55 | 任务 212 → session-engine |
| `src/session/processor/utils/message_format_tests.rs` | 186 | 任务 212 → session-engine |
| `src/session/processor/utils/output_compaction.rs` | 361 | 任务 212 → session-engine |
| `src/session/processor/utils/output_compaction_tests.rs` | 243 | 任务 212 → session-engine |
| `src/session/processor/utils/session_ingest.rs` | 118 | 任务 212 → session-engine |
| `src/session/processor/utils/session_ingest_tests.rs` | 392 | 任务 212 → session-engine |
| `src/session/processor/utils/tool_parsing.rs` | 109 | 任务 212 → session-engine |
| `src/session/processor/utils/tool_parsing_tests.rs` | 91 | 任务 212 → session-engine |
| `src/session/processor/utils.rs` | 35 | 任务 212 → session-engine |
| `src/session/processor/utils_tests.rs` | 8 | 任务 212 → session-engine |
| `src/session/prompt/max-steps.txt` | 16 | 任务 213 → session-engine |
| `src/session/prompt/unified-cn.txt` | 53 | 任务 213 → session-engine |
| `src/session/prompt/unified.txt` | 53 | 任务 213 → session-engine |
| `src/session/prompt.rs` | 268 | 任务 213 → session-engine |
| `src/session/prompt_tests.rs` | 163 | 任务 213 → session-engine |
| `src/session/retry.rs` | 322 | 任务 213 → session-engine |
| `src/session/retry_tests.rs` | 229 | 任务 213 → session-engine |
| `src/session/revert.rs` | 464 | 任务 213 → session-engine |
| `src/session/revert_tests.rs` | 379 | 任务 213 → session-engine |
| `src/session/runtime/dynamic_calls.rs` | 193 | 任务 213 → session-engine |
| `src/session/runtime/dynamic_calls_tests.rs` | 173 | 任务 213 → session-engine |
| `src/session/runtime/dynamic_hosts.rs` | 232 | 任务 213 → session-engine |
| `src/session/runtime/lifecycle.rs` | 285 | 任务 213 → session-engine |
| `src/session/runtime/lifecycle_tests.rs` | 199 | 任务 213 → session-engine |
| `src/session/runtime/mod.rs` | 52 | 任务 213 → session-engine |
| `src/session/runtime/prompt.rs` | 201 | 任务 213 → session-engine |
| `src/session/runtime/prompt_tests.rs` | 452 | 任务 213 → session-engine |
| `src/session/runtime/types.rs` | 389 | 任务 27 → session-port |
| `src/session/session.rs` | 1153 | 任务 214 → session-engine |
| `src/session/session_tests.rs` | 165 | 任务 214 → session-engine |
| `src/session/skill_checkpoint.rs` | 128 | 任务 214 → session-engine |
| `src/session/skill_checkpoint_tests.rs` | 198 | 任务 214 → session-engine |
| `src/session/status.rs` | 245 | 任务 214 → session-engine |
| `src/session/status_tests.rs` | 129 | 任务 214 → session-engine |
| `src/session/store/history.rs` | 176 | 任务 112 → session-store |
| `src/session/store/postgres.rs` | 2243 | 任务 113 → session-store |
| `src/session/store/postgres_tests.rs` | 212 | 任务 114 → session-store |
| `src/session/store.rs` | 3085 | 任务 115 → session-store |
| `src/session/store_tests.rs` | 1697 | 任务 116 → session-store |
| `src/session/summary.rs` | 718 | 任务 215 → session-engine |
| `src/session/summary_tests.rs` | 738 | 任务 215 → session-engine |
| `src/session/system.rs` | 658 | 任务 215 → session-engine |
| `src/session/system_tests.rs` | 382 | 任务 215 → session-engine |
| `src/session/task_session.rs` | 354 | 任务 215 → session-engine |
| `src/session/task_session_tests.rs` | 109 | 任务 215 → session-engine |
| `src/session/title.rs` | 397 | 任务 215 → session-engine |
| `src/session/title_tests.rs` | 167 | 任务 215 → session-engine |
| `src/session/todo.rs` | 167 | 任务 216 → session-engine |
| `src/session/todo_tests.rs` | 168 | 任务 216 → session-engine |
| `src/session/turn_fork.rs` | 408 | 任务 216 → session-engine |
| `src/session/ui_config.rs` | 101 | 任务 116 → session-store |
| `src/session/ui_config_tests.rs` | 251 | 任务 116 → session-store |
| `src/session/ui_store.rs` | 1691 | 任务 116 → session-store |
| `src/session/ui_store_tests.rs` | 739 | 任务 116 → session-store |
| `src/session/ui_types.rs` | 12 | 任务 27 → session-port |
| `src/shell/mod.rs` | 4 | 任务 356 → agent-composition |
| `src/shell/tests.rs` | 186 | 任务 356 → agent-composition |
| `src/shiju/account.rs` | 562 | 任务 117 → shiju-client |
| `src/shiju/account_tests.rs` | 247 | 任务 117 → shiju-client |
| `src/shiju/apps.rs` | 437 | 任务 117 → shiju-client |
| `src/shiju/apps_tests.rs` | 505 | 任务 117 → shiju-client |
| `src/shiju/connectors.rs` | 169 | 任务 117 → shiju-client |
| `src/shiju/connectors_tests.rs` | 125 | 任务 117 → shiju-client |
| `src/shiju/knowledge.rs` | 174 | 任务 117 → shiju-client |
| `src/shiju/knowledge_content.rs` | 188 | 任务 117 → shiju-client |
| `src/shiju/knowledge_content_tests.rs` | 206 | 任务 117 → shiju-client |
| `src/shiju/knowledge_tests.rs` | 181 | 任务 117 → shiju-client |
| `src/shiju/mod.rs` | 8 | 任务 117 → shiju-client |
| `src/shiju/model_token.rs` | 83 | 任务 117 → shiju-client |
| `src/shiju/model_token_tests.rs` | 64 | 任务 117 → shiju-client |
| `src/shiju/monthly_quota.rs` | 89 | 任务 117 → shiju-client |
| `src/skill/README.md` | 9 | 任务 118 → skill-core |
| `src/skill/audit.rs` | 1017 | 任务 118 → skill-core |
| `src/skill/audit_tests.rs` | 522 | 任务 118 → skill-core |
| `src/skill/constants.rs` | 136 | 任务 118 → skill-core |
| `src/skill/constants_tests.rs` | 33 | 任务 118 → skill-core |
| `src/skill/discovery.rs` | 367 | 任务 118 → skill-core |
| `src/skill/discovery_tests.rs` | 253 | 任务 118 → skill-core |
| `src/skill/install.rs` | 1025 | 任务 119 → skill-core |
| `src/skill/install_tests.rs` | 538 | 任务 119 → skill-core |
| `src/skill/loader.rs` | 373 | 任务 119 → skill-core |
| `src/skill/loader_tests.rs` | 273 | 任务 119 → skill-core |
| `src/skill/mod.rs` | 174 | 任务 121 → skill-core |
| `src/skill/policy.rs` | 448 | 任务 119 → skill-core |
| `src/skill/policy_tests.rs` | 226 | 任务 119 → skill-core |
| `src/skill/prompt.rs` | 142 | 任务 119 → skill-core |
| `src/skill/prompt_tests.rs` | 179 | 任务 119 → skill-core |
| `src/skill/skill.rs` | 650 | 任务 120 → skill-core |
| `src/skill/skill_tests.rs` | 302 | 任务 120 → skill-core |
| `src/skill/source.rs` | 278 | 任务 120 → skill-core |
| `src/skill/source_tests.rs` | 136 | 任务 120 → skill-core |
| `src/skill/symlink_tests/mod.rs` | 31 | 任务 120 → skill-core |
| `src/skill/symlink_tests/tests.rs` | 116 | 任务 120 → skill-core |
| `src/skill/tests/download_policy.rs` | 104 | 任务 120 → skill-core |
| `src/skill/tests/git_source.rs` | 84 | 任务 120 → skill-core |
| `src/skill/tests/init.rs` | 72 | 任务 120 → skill-core |
| `src/skill/tests/load_skills.rs` | 327 | 任务 120 → skill-core |
| `src/skill/tests/md_skill.rs` | 18 | 任务 120 → skill-core |
| `src/skill/tests/mod.rs` | 10 | 任务 120 → skill-core |
| `src/skill/tests/prompt.rs` | 160 | 任务 120 → skill-core |
| `src/skill/tests/skills_sh.rs` | 92 | 任务 120 → skill-core |
| `src/skill/tests/toml_skill.rs` | 155 | 任务 120 → skill-core |
| `src/skill/types.rs` | 298 | 任务 121 → skill-core |
| `src/skill/types_tests.rs` | 239 | 任务 121 → skill-core |
| `src/skillforge/evaluate/mod.rs` | 461 | 任务 122 → skill-forge |
| `src/skillforge/evaluate/tests.rs` | 235 | 任务 122 → skill-forge |
| `src/skillforge/integrate/mod.rs` | 406 | 任务 122 → skill-forge |
| `src/skillforge/integrate/tests.rs` | 280 | 任务 122 → skill-forge |
| `src/skillforge/mod.rs` | 546 | 任务 122 → skill-forge |
| `src/skillforge/scout/mod.rs` | 602 | 任务 122 → skill-forge |
| `src/skillforge/scout/tests.rs` | 209 | 任务 122 → skill-forge |
| `src/skillforge/tests.rs` | 353 | 任务 122 → skill-forge |
| `src/skills/agent_metadata.rs` | 38 | 任务 85 → agent-skills |
| `src/skills/audit/manifest.rs` | 82 | 任务 85 → agent-skills |
| `src/skills/audit/manifest_tests.rs` | 84 | 任务 85 → agent-skills |
| `src/skills/audit/markdown.rs` | 188 | 任务 85 → agent-skills |
| `src/skills/audit/markdown_tests.rs` | 107 | 任务 85 → agent-skills |
| `src/skills/audit/mod.rs` | 222 | 任务 85 → agent-skills |
| `src/skills/audit/report.rs` | 74 | 任务 85 → agent-skills |
| `src/skills/audit/report_tests.rs` | 19 | 任务 85 → agent-skills |
| `src/skills/audit/risk.rs` | 81 | 任务 85 → agent-skills |
| `src/skills/audit/risk_tests.rs` | 25 | 任务 85 → agent-skills |
| `src/skills/audit/scan.rs` | 137 | 任务 85 → agent-skills |
| `src/skills/audit/scan_tests.rs` | 87 | 任务 85 → agent-skills |
| `src/skills/audit/support.rs` | 136 | 任务 85 → agent-skills |
| `src/skills/audit/support_tests.rs` | 67 | 任务 85 → agent-skills |
| `src/skills/audit/tests.rs` | 670 | 任务 85 → agent-skills |
| `src/skills/cli.rs` | 304 | 任务 85 → agent-skills |
| `src/skills/cli_tests.rs` | 385 | 任务 85 → agent-skills |
| `src/skills/discovery.rs` | 34 | 任务 85 → agent-skills |
| `src/skills/discovery_tests.rs` | 155 | 任务 85 → agent-skills |
| `src/skills/init.rs` | 97 | 任务 85 → agent-skills |
| `src/skills/init_tests.rs` | 86 | 任务 85 → agent-skills |
| `src/skills/installer.rs` | 844 | 任务 86 → agent-skills |
| `src/skills/installer_tests.rs` | 595 | 任务 86 → agent-skills |
| `src/skills/invoked.rs` | 172 | 任务 86 → agent-skills |
| `src/skills/invoked_tests.rs` | 73 | 任务 86 → agent-skills |
| `src/skills/listing.rs` | 98 | 任务 86 → agent-skills |
| `src/skills/listing_tests.rs` | 114 | 任务 86 → agent-skills |
| `src/skills/loader.rs` | 421 | 任务 86 → agent-skills |
| `src/skills/loader_tests.rs` | 135 | 任务 86 → agent-skills |
| `src/skills/marketplace/install.rs` | 736 | 任务 87 → agent-skills |
| `src/skills/marketplace/install_tests.rs` | 631 | 任务 87 → agent-skills |
| `src/skills/marketplace/mod.rs` | 19 | 任务 87 → agent-skills |
| `src/skills/marketplace/provider.rs` | 221 | 任务 87 → agent-skills |
| `src/skills/marketplace/provider_tests.rs` | 205 | 任务 87 → agent-skills |
| `src/skills/marketplace/skillhub.rs` | 957 | 任务 87 → agent-skills |
| `src/skills/marketplace/skillhub_tests.rs` | 689 | 任务 87 → agent-skills |
| `src/skills/mod.rs` | 594 | 任务 90 → agent-skills |
| `src/skills/mod_tests.rs` | 36 | 任务 87 → agent-skills |
| `src/skills/policy.rs` | 291 | 任务 88 → agent-skills |
| `src/skills/policy_tests.rs` | 314 | 任务 88 → agent-skills |
| `src/skills/prompt.rs` | 376 | 任务 88 → agent-skills |
| `src/skills/prompt_tests.rs` | 108 | 任务 88 → agent-skills |
| `src/skills/session_mounts.rs` | 226 | 任务 88 → agent-skills |
| `src/skills/session_mounts_tests.rs` | 183 | 任务 88 → agent-skills |
| `src/skills/session_state.rs` | 187 | 任务 88 → agent-skills |
| `src/skills/session_state_tests.rs` | 162 | 任务 88 → agent-skills |
| `src/skills/shiju.rs` | 478 | 任务 88 → agent-skills |
| `src/skills/shiju_tests.rs` | 419 | 任务 88 → agent-skills |
| `src/skills/source.rs` | 215 | 任务 88 → agent-skills |
| `src/skills/source_tests.rs` | 36 | 任务 88 → agent-skills |
| `src/skills/symlink_tests/mod.rs` | 11 | 任务 88 → agent-skills |
| `src/skills/symlink_tests/tests.rs` | 177 | 任务 88 → agent-skills |
| `src/skills/tests/domain_trust.rs` | 41 | 任务 89 → agent-skills |
| `src/skills/tests/download_policy.rs` | 103 | 任务 89 → agent-skills |
| `src/skills/tests/git_source.rs` | 91 | 任务 89 → agent-skills |
| `src/skills/tests/helpers.rs` | 155 | 任务 89 → agent-skills |
| `src/skills/tests/load_skills.rs` | 467 | 任务 89 → agent-skills |
| `src/skills/tests/mod.rs` | 51 | 任务 89 → agent-skills |
| `src/skills/tests/skills_init.rs` | 168 | 任务 89 → agent-skills |
| `src/skills/tests/skills_prompt.rs` | 182 | 任务 89 → agent-skills |
| `src/skills/tests/skills_sh_source.rs` | 68 | 任务 89 → agent-skills |
| `src/skills/types.rs` | 450 | 任务 89 → agent-skills |
| `src/skills/types_tests.rs` | 69 | 任务 89 → agent-skills |
| `src/snapshot/common.rs` | 111 | 任务 57 → agent-snapshot |
| `src/snapshot/common_tests.rs` | 170 | 任务 57 → agent-snapshot |
| `src/snapshot/mod.rs` | 50 | 任务 57 → agent-snapshot |
| `src/snapshot/native.rs` | 489 | 任务 57 → agent-snapshot |
| `src/snapshot/native_tests.rs` | 263 | 任务 57 → agent-snapshot |
| `src/snapshot/wasm.rs` | 52 | 任务 57 → agent-snapshot |
| `src/snapshot/wasm_tests.rs` | 14 | 任务 57 → agent-snapshot |
| `src/storage/mod.rs` | 630 | 任务 58 → agent-storage |
| `src/storage/tests.rs` | 382 | 任务 58 → agent-storage |
| `src/task_lifecycle/builtin.rs` | 702 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/builtin_tests.rs` | 165 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/mod.rs` | 18 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/registry.rs` | 208 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/registry_tests.rs` | 277 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/store/postgres.rs` | 189 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/store/postgres_tests.rs` | 43 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/store.rs` | 457 | 任务 123 → task-lifecycle |
| `src/task_lifecycle/store_tests.rs` | 506 | 任务 123 → task-lifecycle |
| `src/task_plan_run/control.rs` | 245 | 任务 166 → task-plan-runner |
| `src/task_plan_run/control_tests.rs` | 152 | 任务 166 → task-plan-runner |
| `src/task_plan_run/goal.rs` | 236 | 任务 166 → task-plan-runner |
| `src/task_plan_run/goal_tests.rs` | 147 | 任务 166 → task-plan-runner |
| `src/task_plan_run/mod.rs` | 62 | 任务 166 → task-plan-runner |
| `src/task_plan_run/model.rs` | 64 | 任务 166 → task-plan-runner |
| `src/task_plan_run/model_tests.rs` | 35 | 任务 166 → task-plan-runner |
| `src/task_plan_run/prompts.rs` | 145 | 任务 166 → task-plan-runner |
| `src/task_plan_run/prompts_tests.rs` | 83 | 任务 166 → task-plan-runner |
| `src/task_plan_run/runner.rs` | 1113 | 任务 166 → task-plan-runner |
| `src/task_plan_run/runner_tests.rs` | 1869 | 任务 166 → task-plan-runner |
| `src/task_plan_run/schedule.rs` | 40 | 任务 166 → task-plan-runner |
| `src/task_plan_run/schedule_tests.rs` | 96 | 任务 166 → task-plan-runner |
| `src/task_plan_run/session_bind.rs` | 188 | 任务 166 → task-plan-runner |
| `src/task_plan_run/session_bind_tests.rs` | 413 | 任务 166 → task-plan-runner |
| `src/task_worktree_pool/mod.rs` | 494 | 任务 132 → task-worktree-pool |
| `src/task_worktree_pool/tests.rs` | 146 | 任务 132 → task-worktree-pool |
| `src/test_capabilities.rs` | 189 | 任务 356 → agent-composition |
| `src/test_capabilities_tests.rs` | 155 | 任务 356 → agent-composition |
| `src/tests.rs` | 7 | 任务 356 → agent-composition |
| `src/tools/agent_tool/communication.rs` | 138 | 任务 170 → tool-agents |
| `src/tools/agent_tool/communication_tests.rs` | 116 | 任务 170 → tool-agents |
| `src/tools/agent_tool/mod.rs` | 912 | 任务 170 → tool-agents |
| `src/tools/agent_tool/tests.rs` | 1521 | 任务 170 → tool-agents |
| `src/tools/apply_patch/apply_patch-cn.txt` | 33 | 任务 186 → tool-services |
| `src/tools/apply_patch/apply_patch.txt` | 33 | 任务 186 → tool-services |
| `src/tools/apply_patch/mod.rs` | 1635 | 任务 186 → tool-services |
| `src/tools/apply_patch/tests.rs` | 278 | 任务 186 → tool-services |
| `src/tools/batch/batch-cn.txt` | 22 | 任务 186 → tool-services |
| `src/tools/batch/batch.txt` | 11 | 任务 186 → tool-services |
| `src/tools/batch/mod.rs` | 262 | 任务 186 → tool-services |
| `src/tools/batch/tests.rs` | 213 | 任务 186 → tool-services |
| `src/tools/browser/actions.rs` | 524 | 任务 172 → tool-browser |
| `src/tools/browser/actions_tests.rs` | 298 | 任务 172 → tool-browser |
| `src/tools/browser/agent_browser.rs` | 406 | 任务 172 → tool-browser |
| `src/tools/browser/agent_browser_tests.rs` | 191 | 任务 172 → tool-browser |
| `src/tools/browser/backend.rs` | 217 | 任务 172 → tool-browser |
| `src/tools/browser/backend_tests.rs` | 42 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/config.rs` | 95 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/config_tests.rs` | 35 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/execution.rs` | 127 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/execution_tests.rs` | 372 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/mod.rs` | 94 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/response.rs` | 37 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/response_tests.rs` | 26 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/tests.rs` | 41 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/validation.rs` | 320 | 任务 172 → tool-browser |
| `src/tools/browser/computer_use/validation_tests.rs` | 446 | 任务 172 → tool-browser |
| `src/tools/browser/helpers.rs` | 384 | 任务 173 → tool-browser |
| `src/tools/browser/helpers_tests.rs` | 95 | 任务 173 → tool-browser |
| `src/tools/browser/installation.rs` | 137 | 任务 173 → tool-browser |
| `src/tools/browser/installation_tests.rs` | 57 | 任务 173 → tool-browser |
| `src/tools/browser/mod.rs` | 135 | 任务 174 → tool-browser |
| `src/tools/browser/native_backend/actions.rs` | 97 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/actions_tests.rs` | 415 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/mod.rs` | 501 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/session.rs` | 244 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/session_tests.rs` | 255 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/snapshot.rs` | 177 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/snapshot_tests.rs` | 33 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/tests.rs` | 766 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/webdriver.rs` | 372 | 任务 173 → tool-browser |
| `src/tools/browser/native_backend/webdriver_tests.rs` | 155 | 任务 173 → tool-browser |
| `src/tools/browser/tests.rs` | 600 | 任务 173 → tool-browser |
| `src/tools/browser/tool.rs` | 677 | 任务 174 → tool-browser |
| `src/tools/browser/tool_tests.rs` | 739 | 任务 174 → tool-browser |
| `src/tools/browser_open/mod.rs` | 635 | 任务 175 → tool-browser |
| `src/tools/browser_open/tests.rs` | 392 | 任务 175 → tool-browser |
| `src/tools/catalog.rs` | 274 | 任务 270 → tool-assembly |
| `src/tools/catalog_tests.rs` | 174 | 任务 270 → tool-assembly |
| `src/tools/client_tool.rs` | 80 | 任务 186 → tool-services |
| `src/tools/client_tool_tests.rs` | 162 | 任务 186 → tool-services |
| `src/tools/codesearch/codesearch-cn.txt` | 8 | 任务 196 → tool-web |
| `src/tools/codesearch/codesearch.txt` | 8 | 任务 196 → tool-web |
| `src/tools/codesearch/mod.rs` | 229 | 任务 196 → tool-web |
| `src/tools/codesearch/tests.rs` | 276 | 任务 196 → tool-web |
| `src/tools/computer_control/chrome.rs` | 192 | 任务 176 → tool-computer |
| `src/tools/computer_control/chrome_tests.rs` | 210 | 任务 176 → tool-computer |
| `src/tools/computer_control.rs` | 578 | 任务 176 → tool-computer |
| `src/tools/computer_control_tests.rs` | 424 | 任务 176 → tool-computer |
| `src/tools/config/mod.rs` | 820 | 任务 187 → tool-services |
| `src/tools/config/tests.rs` | 532 | 任务 187 → tool-services |
| `src/tools/content_search/mod.rs` | 810 | 任务 178 → tool-files |
| `src/tools/content_search/tests.rs` | 641 | 任务 178 → tool-files |
| `src/tools/context/mod.rs` | 995 | 任务 28 → tool-context |
| `src/tools/context/run_worktree.rs` | 86 | 任务 28 → tool-context |
| `src/tools/context/tests.rs` | 223 | 任务 28 → tool-context |
| `src/tools/cron_add/mod.rs` | 688 | 任务 177 → tool-cron |
| `src/tools/cron_add/tests.rs` | 535 | 任务 177 → tool-cron |
| `src/tools/cron_list/mod.rs` | 65 | 任务 177 → tool-cron |
| `src/tools/cron_list/tests.rs` | 126 | 任务 177 → tool-cron |
| `src/tools/cron_remove/mod.rs` | 235 | 任务 177 → tool-cron |
| `src/tools/cron_remove/tests.rs` | 234 | 任务 177 → tool-cron |
| `src/tools/cron_run/mod.rs` | 284 | 任务 177 → tool-cron |
| `src/tools/cron_run/tests.rs` | 173 | 任务 177 → tool-cron |
| `src/tools/cron_runs/mod.rs` | 122 | 任务 177 → tool-cron |
| `src/tools/cron_runs/tests.rs` | 137 | 任务 177 → tool-cron |
| `src/tools/cron_update/mod.rs` | 374 | 任务 177 → tool-cron |
| `src/tools/cron_update/tests.rs` | 363 | 任务 177 → tool-cron |
| `src/tools/decision/mod.rs` | 505 | 任务 28 → tool-context |
| `src/tools/decision/tests.rs` | 481 | 任务 28 → tool-context |
| `src/tools/delegate/agentic.rs` | 131 | 任务 170 → tool-agents |
| `src/tools/delegate/agentic_tests.rs` | 127 | 任务 170 → tool-agents |
| `src/tools/delegate/execution.rs` | 198 | 任务 170 → tool-agents |
| `src/tools/delegate/execution_tests.rs` | 239 | 任务 170 → tool-agents |
| `src/tools/delegate/mod.rs` | 227 | 任务 170 → tool-agents |
| `src/tools/delegate/support.rs` | 21 | 任务 170 → tool-agents |
| `src/tools/delegate/support_tests.rs` | 71 | 任务 170 → tool-agents |
| `src/tools/delegate/tests.rs` | 907 | 任务 170 → tool-agents |
| `src/tools/delegated_tools.rs` | 171 | 任务 187 → tool-services |
| `src/tools/delegated_tools_tests.rs` | 172 | 任务 187 → tool-services |
| `src/tools/discovery.rs` | 359 | 任务 270 → tool-assembly |
| `src/tools/discovery_tests.rs` | 297 | 任务 270 → tool-assembly |
| `src/tools/enter_worktree/mod.rs` | 148 | 任务 182 → tool-git |
| `src/tools/enter_worktree/tests.rs` | 166 | 任务 182 → tool-git |
| `src/tools/excel_control.rs` | 378 | 任务 185 → tool-office |
| `src/tools/excel_control_tests.rs` | 425 | 任务 185 → tool-office |
| `src/tools/executor/mod.rs` | 448 | 任务 28 → tool-context |
| `src/tools/executor/tests.rs` | 184 | 任务 28 → tool-context |
| `src/tools/exit_worktree/mod.rs` | 226 | 任务 182 → tool-git |
| `src/tools/exit_worktree/tests.rs` | 342 | 任务 182 → tool-git |
| `src/tools/external_directory/mod.rs` | 285 | 任务 178 → tool-files |
| `src/tools/external_directory/tests.rs` | 271 | 任务 178 → tool-files |
| `src/tools/file_edit/mod.rs` | 311 | 任务 178 → tool-files |
| `src/tools/file_edit/tests.rs` | 543 | 任务 178 → tool-files |
| `src/tools/file_edit/types.rs` | 53 | 任务 178 → tool-files |
| `src/tools/file_edit/types_tests.rs` | 8 | 任务 178 → tool-files |
| `src/tools/file_edit/utils.rs` | 371 | 任务 178 → tool-files |
| `src/tools/file_edit/utils_tests.rs` | 35 | 任务 178 → tool-files |
| `src/tools/file_read/mod.rs` | 1309 | 任务 179 → tool-files |
| `src/tools/file_read/tests.rs` | 1389 | 任务 179 → tool-files |
| `src/tools/file_write/mod.rs` | 337 | 任务 179 → tool-files |
| `src/tools/file_write/tests.rs` | 836 | 任务 179 → tool-files |
| `src/tools/file_write/write-cn.txt` | 9 | 任务 179 → tool-files |
| `src/tools/file_write/write.txt` | 11 | 任务 179 → tool-files |
| `src/tools/git_operations/merge.rs` | 105 | 任务 182 → tool-git |
| `src/tools/git_operations/mod.rs` | 521 | 任务 182 → tool-git |
| `src/tools/git_operations/tests.rs` | 293 | 任务 182 → tool-git |
| `src/tools/glob/glob-cn.txt` | 6 | 任务 179 → tool-files |
| `src/tools/glob/glob.txt` | 9 | 任务 179 → tool-files |
| `src/tools/glob/mod.rs` | 365 | 任务 179 → tool-files |
| `src/tools/glob/tests.rs` | 237 | 任务 179 → tool-files |
| `src/tools/glob_search/mod.rs` | 365 | 任务 180 → tool-files |
| `src/tools/glob_search/tests.rs` | 292 | 任务 180 → tool-files |
| `src/tools/grep/grep-cn.txt` | 9 | 任务 180 → tool-files |
| `src/tools/grep/grep.txt` | 12 | 任务 180 → tool-files |
| `src/tools/grep/mod.rs` | 707 | 任务 180 → tool-files |
| `src/tools/grep/tests.rs` | 571 | 任务 180 → tool-files |
| `src/tools/hooks/mod.rs` | 81 | 任务 28 → tool-context |
| `src/tools/hooks/tests.rs` | 66 | 任务 28 → tool-context |
| `src/tools/http_headers.rs` | 16 | 任务 196 → tool-web |
| `src/tools/http_headers_tests.rs` | 12 | 任务 196 → tool-web |
| `src/tools/http_request/mod.rs` | 478 | 任务 196 → tool-web |
| `src/tools/http_request/tests.rs` | 716 | 任务 196 → tool-web |
| `src/tools/image_asset_read/mod.rs` | 137 | 任务 180 → tool-files |
| `src/tools/image_asset_read/tests.rs` | 148 | 任务 180 → tool-files |
| `src/tools/ls/ls-cn.txt` | 1 | 任务 180 → tool-files |
| `src/tools/ls/ls.txt` | 1 | 任务 180 → tool-files |
| `src/tools/ls/mod.rs` | 549 | 任务 180 → tool-files |
| `src/tools/ls/tests.rs` | 257 | 任务 180 → tool-files |
| `src/tools/mcp_catalog_refresh_tests.rs` | 263 | 任务 187 → tool-services |
| `src/tools/mcp_common/mod.rs` | 1004 | 任务 97 → mcp-client |
| `src/tools/mcp_common/tests.rs` | 960 | 任务 97 → mcp-client |
| `src/tools/mcp_common/wire_check.rs` | 56 | 任务 97 → mcp-client |
| `src/tools/mcp_common/wire_http.rs` | 394 | 任务 97 → mcp-client |
| `src/tools/mcp_common/wire_stdio.rs` | 202 | 任务 97 → mcp-client |
| `src/tools/mcp_inspect.rs` | 221 | 任务 183 → tool-mcp |
| `src/tools/mcp_inspect_tests.rs` | 109 | 任务 183 → tool-mcp |
| `src/tools/mcp_resource.rs` | 51 | 任务 183 → tool-mcp |
| `src/tools/mcp_status/mod.rs` | 148 | 任务 183 → tool-mcp |
| `src/tools/mcp_status/tests.rs` | 285 | 任务 183 → tool-mcp |
| `src/tools/mcp_tool.rs` | 513 | 任务 183 → tool-mcp |
| `src/tools/mcp_tool_tests.rs` | 517 | 任务 183 → tool-mcp |
| `src/tools/memory_forget/mod.rs` | 159 | 任务 184 → tool-memory |
| `src/tools/memory_forget/tests.rs` | 84 | 任务 184 → tool-memory |
| `src/tools/memory_recall/mod.rs` | 90 | 任务 184 → tool-memory |
| `src/tools/memory_recall/tests.rs` | 158 | 任务 184 → tool-memory |
| `src/tools/memory_store/mod.rs` | 177 | 任务 184 → tool-memory |
| `src/tools/memory_store/tests.rs` | 218 | 任务 184 → tool-memory |
| `src/tools/mod.rs` | 308 | 任务 271 → tool-assembly |
| `src/tools/mod_tests.rs` | 36 | 任务 270 → tool-assembly |
| `src/tools/pdf_read/mod.rs` | 437 | 任务 181 → tool-files |
| `src/tools/pdf_read/render.rs` | 91 | 任务 181 → tool-files |
| `src/tools/pdf_read/render_tests.rs` | 146 | 任务 181 → tool-files |
| `src/tools/pdf_read/tests.rs` | 424 | 任务 181 → tool-files |
| `src/tools/powershell/mod.rs` | 157 | 任务 190 → tool-shell |
| `src/tools/powershell/tests.rs` | 64 | 任务 190 → tool-shell |
| `src/tools/question/mod.rs` | 782 | 任务 187 → tool-services |
| `src/tools/question/question-cn.txt` | 13 | 任务 187 → tool-services |
| `src/tools/question/question.txt` | 13 | 任务 187 → tool-services |
| `src/tools/question/tests.rs` | 670 | 任务 187 → tool-services |
| `src/tools/read_state/mod.rs` | 250 | 任务 187 → tool-services |
| `src/tools/read_state/tests.rs` | 92 | 任务 187 → tool-services |
| `src/tools/registry/mod.rs` | 363 | 任务 270 → tool-assembly |
| `src/tools/registry/tests.rs` | 84 | 任务 270 → tool-assembly |
| `src/tools/remote_trigger/mod.rs` | 93 | 任务 187 → tool-services |
| `src/tools/remote_trigger/tests.rs` | 122 | 任务 187 → tool-services |
| `src/tools/schedule/mod.rs` | 392 | 任务 188 → tool-services |
| `src/tools/schedule/tests.rs` | 546 | 任务 188 → tool-services |
| `src/tools/scheduler/mod.rs` | 103 | 任务 188 → tool-services |
| `src/tools/scheduler/tests.rs` | 113 | 任务 188 → tool-services |
| `src/tools/screenshot/mod.rs` | 866 | 任务 188 → tool-services |
| `src/tools/screenshot/tests.rs` | 668 | 任务 188 → tool-services |
| `src/tools/session_tool/mod.rs` | 519 | 任务 170 → tool-agents |
| `src/tools/session_tool/tests.rs` | 567 | 任务 170 → tool-agents |
| `src/tools/shell/ast/command_info/ast_walk.rs` | 405 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/ast_walk_tests.rs` | 149 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/mod.rs` | 62 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/scan.rs` | 724 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/scan_tests.rs` | 338 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/tests.rs` | 17 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/wrappers.rs` | 202 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info/wrappers_tests.rs` | 161 | 任务 191 → tool-shell |
| `src/tools/shell/ast/command_info_tests.rs` | 44 | 任务 191 → tool-shell |
| `src/tools/shell/ast/mod.rs` | 123 | 任务 191 → tool-shell |
| `src/tools/shell/ast/mod_tests.rs` | 28 | 任务 191 → tool-shell |
| `src/tools/shell/ast/nodes.rs` | 90 | 任务 191 → tool-shell |
| `src/tools/shell/ast/nodes_tests.rs` | 77 | 任务 191 → tool-shell |
| `src/tools/shell/ast/parser.rs` | 205 | 任务 191 → tool-shell |
| `src/tools/shell/ast/parser_tests.rs` | 147 | 任务 191 → tool-shell |
| `src/tools/shell/compound/mod.rs` | 533 | 任务 192 → tool-shell |
| `src/tools/shell/compound/semantics.rs` | 101 | 任务 192 → tool-shell |
| `src/tools/shell/compound/semantics_tests.rs` | 113 | 任务 192 → tool-shell |
| `src/tools/shell/compound/tests.rs` | 239 | 任务 192 → tool-shell |
| `src/tools/shell/environment_tests.rs` | 52 | 任务 192 → tool-shell |
| `src/tools/shell/long_running_tests.rs` | 239 | 任务 192 → tool-shell |
| `src/tools/shell/mod.rs` | 889 | 任务 195 → tool-shell |
| `src/tools/shell/output.rs` | 177 | 任务 192 → tool-shell |
| `src/tools/shell/output_tests.rs` | 91 | 任务 192 → tool-shell |
| `src/tools/shell/path/extractors.rs` | 363 | 任务 192 → tool-shell |
| `src/tools/shell/path/extractors_tests.rs` | 124 | 任务 192 → tool-shell |
| `src/tools/shell/path/mod.rs` | 378 | 任务 192 → tool-shell |
| `src/tools/shell/path/mod_tests.rs` | 256 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/mod.rs` | 117 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/mod_tests.rs` | 75 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/mode.rs` | 31 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/mode_tests.rs` | 20 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/rules.rs` | 272 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/rules_tests.rs` | 231 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/sandbox_allow.rs` | 77 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/sandbox_allow_tests.rs` | 49 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/warning.rs` | 74 | 任务 192 → tool-shell |
| `src/tools/shell/permissions/warning_tests.rs` | 22 | 任务 192 → tool-shell |
| `src/tools/shell/processes.rs` | 494 | 任务 193 → tool-shell |
| `src/tools/shell/processes_tests.rs` | 300 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/allowlist.rs` | 433 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/allowlist_tests.rs` | 102 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/flag_validator.rs` | 42 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/flag_validator_tests.rs` | 30 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/mod.rs` | 145 | 任务 193 → tool-shell |
| `src/tools/shell/readonly/mod_tests.rs` | 75 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/executor.rs` | 168 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/executor_tests.rs` | 100 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/mod.rs` | 247 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/mod_tests.rs` | 26 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/policy.rs` | 87 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/policy_tests.rs` | 23 | 任务 193 → tool-shell |
| `src/tools/shell/sandbox/sandbox_tests.rs` | 189 | 任务 193 → tool-shell |
| `src/tools/shell/security/injection.rs` | 34 | 任务 194 → tool-shell |
| `src/tools/shell/security/injection_tests.rs` | 38 | 任务 194 → tool-shell |
| `src/tools/shell/security/mod.rs` | 179 | 任务 194 → tool-shell |
| `src/tools/shell/security/mod_tests.rs` | 91 | 任务 194 → tool-shell |
| `src/tools/shell/security/obfuscation.rs` | 31 | 任务 194 → tool-shell |
| `src/tools/shell/security/obfuscation_tests.rs` | 11 | 任务 194 → tool-shell |
| `src/tools/shell/security/tests.rs` | 108 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/backslash.rs` | 37 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/backslash_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/brace_expansion.rs` | 35 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/brace_expansion_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/comment_quote_desync.rs` | 30 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/comment_quote_desync_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/control_chars.rs` | 28 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/control_chars_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/dangerous_vars.rs` | 58 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/dangerous_vars_tests.rs` | 44 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/empty_command.rs` | 37 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/empty_command_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/git_commit.rs` | 41 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/git_commit_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/hash_comment.rs` | 28 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/hash_comment_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/heredoc.rs` | 38 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/heredoc_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/ifs_injection.rs` | 30 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/ifs_injection_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/jq.rs` | 39 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/jq_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/malformed.rs` | 38 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/malformed_tests.rs` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/metacharacters.rs` | 39 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/metacharacters_tests.rs` | 28 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/mod.rs` | 312 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/newline_injection.rs` | 27 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/newline_injection_tests.rs` | 31 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/obfuscated_flags.rs` | 41 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/obfuscated_flags_tests.rs` | 24 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/proc_environ.rs` | 31 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/proc_environ_tests.rs` | 25 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/quoted_newline.rs` | 25 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/quoted_newline_tests.rs` | 24 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/redirection.rs` | 52 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/redirection_tests.rs` | 25 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/substitution.rs` | 76 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/substitution_tests.rs` | 49 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/tests.rs` | 161 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/unicode_whitespace.rs` | 25 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/unicode_whitespace_tests.rs` | 23 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/zsh_dangerous.rs` | 32 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators/zsh_dangerous_tests.rs` | 24 | 任务 194 → tool-shell |
| `src/tools/shell/security/validators_tests.rs` | 151 | 任务 194 → tool-shell |
| `src/tools/shell/sed/mod.rs` | 18 | 任务 194 → tool-shell |
| `src/tools/shell/sed/parser.rs` | 127 | 任务 194 → tool-shell |
| `src/tools/shell/sed/parser_tests.rs` | 115 | 任务 194 → tool-shell |
| `src/tools/shell/sed/validation.rs` | 192 | 任务 194 → tool-shell |
| `src/tools/shell/sed/validation_tests.rs` | 137 | 任务 194 → tool-shell |
| `src/tools/shell/session_tools.rs` | 301 | 任务 194 → tool-shell |
| `src/tools/shell/session_tools_tests.rs` | 234 | 任务 194 → tool-shell |
| `src/tools/shell/shell-cn.txt` | 26 | 任务 194 → tool-shell |
| `src/tools/shell/shell.txt` | 25 | 任务 194 → tool-shell |
| `src/tools/shell/termination.rs` | 93 | 任务 194 → tool-shell |
| `src/tools/shell/termination_tests.rs` | 153 | 任务 194 → tool-shell |
| `src/tools/shell/termination_windows.rs` | 83 | 任务 194 → tool-shell |
| `src/tools/shell/termination_windows_tests.rs` | 131 | 任务 194 → tool-shell |
| `src/tools/shell/tests.rs` | 1225 | 任务 194 → tool-shell |
| `src/tools/shell/timeout-audit.md` | 107 | 任务 194 → tool-shell |
| `src/tools/skill/invocation.rs` | 78 | 任务 189 → tool-services |
| `src/tools/skill/invocation_tests.rs` | 120 | 任务 189 → tool-services |
| `src/tools/skill/mod.rs` | 970 | 任务 189 → tool-services |
| `src/tools/skill/mod_tests.rs` | 500 | 任务 189 → tool-services |
| `src/tools/skill/prompt.rs` | 82 | 任务 189 → tool-services |
| `src/tools/skill/tests.rs` | 125 | 任务 189 → tool-services |
| `src/tools/sleep/mod.rs` | 115 | 任务 189 → tool-services |
| `src/tools/sleep/mod_tests.rs` | 52 | 任务 189 → tool-services |
| `src/tools/sleep/tests.rs` | 36 | 任务 189 → tool-services |
| `src/tools/tests/mod_tests.rs` | 611 | 任务 189 → tool-services |
| `src/tools/thread_tool/execution_tests.rs` | 126 | 任务 171 → tool-agents |
| `src/tools/thread_tool/mod.rs` | 572 | 任务 171 → tool-agents |
| `src/tools/thread_tool/tests.rs` | 485 | 任务 171 → tool-agents |
| `src/tools/todo/mod.rs` | 401 | 任务 189 → tool-services |
| `src/tools/todo/normalize.rs` | 100 | 任务 189 → tool-services |
| `src/tools/todo/normalize_tests.rs` | 44 | 任务 189 → tool-services |
| `src/tools/todo/schema.rs` | 53 | 任务 189 → tool-services |
| `src/tools/todo/schema_tests.rs` | 31 | 任务 189 → tool-services |
| `src/tools/todo/store.rs` | 226 | 任务 189 → tool-services |
| `src/tools/todo/store_tests.rs` | 145 | 任务 189 → tool-services |
| `src/tools/todo/tests.rs` | 172 | 任务 189 → tool-services |
| `src/tools/todo/todoread-cn.txt` | 13 | 任务 189 → tool-services |
| `src/tools/todo/todoread.txt` | 3 | 任务 189 → tool-services |
| `src/tools/todo/todowrite-cn.txt` | 178 | 任务 189 → tool-services |
| `src/tools/todo/todowrite.txt` | 16 | 任务 189 → tool-services |
| `src/tools/tool_search.rs` | 221 | 任务 270 → tool-assembly |
| `src/tools/tool_search_tests.rs` | 356 | 任务 270 → tool-assembly |
| `src/tools/toolset/mod.rs` | 1257 | 任务 271 → tool-assembly |
| `src/tools/toolset/mod_tests.rs` | 1056 | 任务 271 → tool-assembly |
| `src/tools/toolset/tests.rs` | 142 | 任务 271 → tool-assembly |
| `src/tools/traits/dynamic_tools_tests.rs` | 49 | 任务 23 → tool-types |
| `src/tools/traits/mod.rs` | 656 | 任务 23 → tool-types |
| `src/tools/traits/mod_tests.rs` | 144 | 任务 23 → tool-types |
| `src/tools/traits/tests.rs` | 177 | 任务 23 → tool-types |
| `src/tools/url_validation/mod.rs` | 592 | 任务 107 → network-policy |
| `src/tools/url_validation/mod_tests.rs` | 108 | 任务 107 → network-policy |
| `src/tools/url_validation/tests.rs` | 341 | 任务 107 → network-policy |
| `src/tools/web_fetch/mod.rs` | 1088 | 任务 196 → tool-web |
| `src/tools/web_fetch/mod_tests.rs` | 163 | 任务 196 → tool-web |
| `src/tools/web_fetch/tests.rs` | 762 | 任务 196 → tool-web |
| `src/tools/web_fetch/web_fetch-cn.txt` | 15 | 任务 196 → tool-web |
| `src/tools/web_fetch/web_fetch.txt` | 11 | 任务 196 → tool-web |
| `src/tools/web_search_tool/mod.rs` | 1054 | 任务 197 → tool-web |
| `src/tools/web_search_tool/mod_tests.rs` | 196 | 任务 197 → tool-web |
| `src/tools/web_search_tool/tests.rs` | 541 | 任务 197 → tool-web |
| `src/tools/websearch/mod.rs` | 727 | 任务 197 → tool-web |
| `src/tools/websearch/mod_tests.rs` | 111 | 任务 197 → tool-web |
| `src/tools/websearch/tests.rs` | 299 | 任务 197 → tool-web |
| `src/tools/websearch/websearch-cn.txt` | 14 | 任务 197 → tool-web |
| `src/tools/websearch/websearch.txt` | 14 | 任务 197 → tool-web |
| `src/tools/word_control.rs` | 329 | 任务 185 → tool-office |
| `src/tools/word_control_tests.rs` | 418 | 任务 185 → tool-office |
| `src/tunnel/cloudflare/mod.rs` | 252 | 任务 355 → agent-tunnel |
| `src/tunnel/cloudflare/tests.rs` | 98 | 任务 355 → agent-tunnel |
| `src/tunnel/custom/mod.rs` | 276 | 任务 355 → agent-tunnel |
| `src/tunnel/custom/tests.rs` | 122 | 任务 355 → agent-tunnel |
| `src/tunnel/mod.rs` | 142 | 任务 355 → agent-tunnel |
| `src/tunnel/ngrok/mod.rs` | 260 | 任务 355 → agent-tunnel |
| `src/tunnel/ngrok/tests.rs` | 203 | 任务 355 → agent-tunnel |
| `src/tunnel/none/mod.rs` | 34 | 任务 355 → agent-tunnel |
| `src/tunnel/none/tests.rs` | 100 | 任务 355 → agent-tunnel |
| `src/tunnel/tailscale/mod.rs` | 411 | 任务 355 → agent-tunnel |
| `src/tunnel/tailscale/tests.rs` | 633 | 任务 355 → agent-tunnel |
| `src/tunnel/tests.rs` | 434 | 任务 355 → agent-tunnel |
| `src/update.rs` | 4 | 任务 356 → agent-composition |
| `src/usage_bridge/context.rs` | 10 | 任务 91 → agent-usage |
| `src/usage_bridge/events.rs` | 48 | 任务 91 → agent-usage |
| `src/usage_bridge/mod.rs` | 5 | 任务 91 → agent-usage |
| `src/usage_bridge/native.rs` | 137 | 任务 91 → agent-usage |
| `src/usage_bridge/relay.rs` | 195 | 任务 91 → agent-usage |
| `src/util/archive.rs` | 62 | 任务 59 → agent-util |
| `src/util/archive_tests.rs` | 12 | 任务 59 → agent-util |
| `src/util/filesystem.rs` | 326 | 任务 59 → agent-util |
| `src/util/filesystem_tests.rs` | 50 | 任务 59 → agent-util |
| `src/util/format.rs` | 4 | 任务 59 → agent-util |
| `src/util/locale.rs` | 77 | 任务 59 → agent-util |
| `src/util/locale_tests.rs` | 35 | 任务 59 → agent-util |
| `src/util/log.rs` | 3 | 既有 agent-log；保留薄导出 |
| `src/util/mod.rs` | 250 | 任务 59 → agent-util |
| `src/util/rpc.rs` | 219 | 任务 59 → agent-util |
| `src/util/rpc_tests.rs` | 15 | 任务 59 → agent-util |
| `src/util/signal.rs` | 34 | 任务 59 → 既有 agent-async |
| `src/util/signal_tests.rs` | 25 | 任务 59 → 既有 agent-async |
| `src/util/tests.rs` | 66 | 任务 59 → agent-util |
| `src/workflow/code_runner.rs` | 370 | 任务 198 → workflow-engine |
| `src/workflow/code_runner_http.rs` | 433 | 任务 198 → workflow-engine |
| `src/workflow/code_runner_http_tests.rs` | 274 | 任务 198 → workflow-engine |
| `src/workflow/code_runner_tests.rs` | 132 | 任务 198 → workflow-engine |
| `src/workflow/conditions.rs` | 129 | 任务 133 → workflow-core |
| `src/workflow/conditions_tests.rs` | 297 | 任务 133 → workflow-core |
| `src/workflow/dsml_tests.rs` | 191 | 任务 198 → workflow-engine |
| `src/workflow/iteration_parallel.rs` | 228 | 任务 198 → workflow-engine |
| `src/workflow/iteration_parallel_tests.rs` | 415 | 任务 198 → workflow-engine |
| `src/workflow/mod.rs` | 42 | 任务 200 → workflow-engine |
| `src/workflow/model.rs` | 108 | 任务 133 → workflow-core |
| `src/workflow/model_tests.rs` | 89 | 任务 133 → workflow-core |
| `src/workflow/native.rs` | 349 | 任务 198 → workflow-engine |
| `src/workflow/native_agent.rs` | 167 | 任务 198 → workflow-engine |
| `src/workflow/native_tests.rs` | 506 | 任务 198 → workflow-engine |
| `src/workflow/native_video_progress.rs` | 172 | 任务 198 → workflow-engine |
| `src/workflow/native_video_progress_tests.rs` | 59 | 任务 198 → workflow-engine |
| `src/workflow/runner.rs` | 3476 | 任务 199 → workflow-engine |
| `src/workflow/runner_tests.rs` | 803 | 任务 200 → workflow-engine |
| `src/workflow/store/error.rs` | 26 | 任务 133 → workflow-core |
| `src/workflow/store/postgres.rs` | 265 | 任务 133 → workflow-core |
| `src/workflow/store/postgres_tests.rs` | 58 | 任务 133 → workflow-core |
| `src/workflow/store/template_records.rs` | 81 | 任务 133 → workflow-core |
| `src/workflow/store.rs` | 346 | 任务 133 → workflow-core |
| `src/workflow/store_tests.rs` | 450 | 任务 133 → workflow-core |
| `src/workflow/template.rs` | 49 | 任务 133 → workflow-core |
| `src/workflow/template_tests.rs` | 67 | 任务 133 → workflow-core |
| `src/workflow/templates.rs` | 106 | 任务 133 → workflow-core |
| `src/workflow/templates_tests.rs` | 332 | 任务 133 → workflow-core |
| `src/workflow/tests.rs` | 3491 | 任务 200 → workflow-engine |
| `src/workflow/variables.rs` | 110 | 任务 133 → workflow-core |
| `src/workflow/variables_tests.rs` | 115 | 任务 133 → workflow-core |
| `src/workflow/video_edit/apply.rs` | 427 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/apply_journal.rs` | 98 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/apply_test_cloud.rs` | 195 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/apply_tests.rs` | 340 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/assets.rs` | 423 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/assets_tests.rs` | 452 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/budget.rs` | 159 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/budget_tests.rs` | 23 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud.rs` | 503 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud_assets.rs` | 144 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud_assets_tests.rs` | 60 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud_caller.py` | 97 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud_delivery.rs` | 283 | 任务 201 → workflow-video-edit |
| `src/workflow/video_edit/cloud_delivery_read.rs` | 287 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_delivery_tests.rs` | 213 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_delivery_write.rs` | 108 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_failure.rs` | 86 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_memory.rs` | 170 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_memory_tests.rs` | 237 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_prepare.rs` | 297 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_prepare_tests.rs` | 116 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_probe.rs` | 37 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_tests.rs` | 384 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/cloud_transcript.rs` | 48 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/command.rs` | 97 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/command_tests.rs` | 130 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/e2e_test_cloud.rs` | 371 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/e2e_tests.rs` | 479 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/entry.rs` | 483 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/entry_contract.rs` | 127 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/entry_presentation_tests.rs` | 32 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/entry_test_transport.rs` | 257 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/entry_tests.rs` | 72 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/manifest.rs` | 245 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/manifest_tests.rs` | 134 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/merge.rs` | 180 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/merge_tests.rs` | 150 | 任务 202 → workflow-video-edit |
| `src/workflow/video_edit/mod.rs` | 76 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/planning.rs` | 137 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/planning_tests.rs` | 184 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/prepare.rs` | 445 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/prepare_tests.rs` | 446 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/preview.rs` | 86 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/preview_tests.rs` | 68 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/segments.rs` | 154 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/segments_tests.rs` | 109 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/transcript.rs` | 315 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/transcript_tests.rs` | 262 | 任务 203 → workflow-video-edit |
| `src/workflow/video_edit/types.rs` | 17 | 任务 203 → workflow-video-edit |
| `src/worktree/cleanup.rs` | 401 | 任务 134 → worktree-core |
| `src/worktree/cleanup_flow_tests.rs` | 651 | 任务 134 → worktree-core |
| `src/worktree/cleanup_host.rs` | 107 | 任务 134 → worktree-core |
| `src/worktree/cleanup_tests.rs` | 426 | 任务 134 → worktree-core |
| `src/worktree/creation_policy.rs` | 95 | 任务 134 → worktree-core |
| `src/worktree/creation_policy_tests.rs` | 168 | 任务 134 → worktree-core |
| `src/worktree/environment.rs` | 233 | 任务 134 → worktree-core |
| `src/worktree/environment_tests.rs` | 329 | 任务 134 → worktree-core |
| `src/worktree/management.rs` | 307 | 任务 134 → worktree-core |
| `src/worktree/management_e2e_tests.rs` | 260 | 任务 134 → worktree-core |
| `src/worktree/mod.rs` | 37 | 任务 135 → worktree-core |
| `src/worktree/recovery.rs` | 431 | 任务 134 → worktree-core |
| `src/worktree/recovery_restore.rs` | 326 | 任务 134 → worktree-core |
| `src/worktree/recovery_restore_tests.rs` | 317 | 任务 134 → worktree-core |
| `src/worktree/recovery_tests.rs` | 288 | 任务 134 → worktree-core |
| `src/worktree/registry.rs` | 391 | 任务 135 → worktree-core |
| `src/worktree/registry_tests.rs` | 167 | 任务 135 → worktree-core |
| `src/worktree/tests.rs` | 13 | 任务 135 → worktree-core |
| `src/worktree/worktree.rs` | 95 | 任务 135 → worktree-core |
| `src/worktree/worktree_event.rs` | 11 | 任务 135 → worktree-core |
| `src/worktree/worktree_event_tests.rs` | 10 | 任务 135 → worktree-core |
| `src/worktree/worktree_naming.rs` | 55 | 任务 135 → worktree-core |
| `src/worktree/worktree_naming_tests.rs` | 19 | 任务 135 → worktree-core |
| `src/worktree/worktree_native.rs` | 315 | 任务 135 → worktree-core |
| `src/worktree/worktree_native_tests.rs` | 514 | 任务 135 → worktree-core |
| `src/worktree/worktree_ops.rs` | 460 | 任务 135 → worktree-core |
| `src/worktree/worktree_ops_tests.rs` | 300 | 任务 135 → worktree-core |
| `src/worktree/worktree_types.rs` | 119 | 任务 135 → worktree-core |
| `src/worktree/worktree_types_tests.rs` | 164 | 任务 135 → worktree-core |
| `testdata/plugin_compat/README.md` | 62 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/e2e/README.md` | 44 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t02/baseline.json` | 29 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t02/complete.json` | 35 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t02/expected.json` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t02/package/.vibewindow-plugin/plugin.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t02/provenance.json` | 35 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t03/README.md` | 42 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t03/legacy.json` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t04/README.md` | 47 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t04/portable.json` | 17 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t05/README.md` | 34 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t05/root_aliases.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t06/README.md` | 34 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t06/plugin.json` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t07/README.md` | 22 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t07/legacy.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t07/native.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t07/portable.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t08/README.md` | 31 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t08/payload.txt` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t08/plugin.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t09/README.md` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t09/alpha/SKILL.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t09/beta/SKILL.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t09/plugin.json` | 4 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t10/README.md` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t10/SKILL.md` | 6 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t10/plugin.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t11/expected-skill.md` | 12 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t11/missing.md` | 2 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t11/plugin.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t11/review.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t11/sources.json` | 47 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t12/SKILL.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t12/openai.yaml` | 17 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t12/sources.json` | 100 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t13/SKILL.md` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t13/sources.json` | 36 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t14/sources.json` | 33 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t14/wrapped.json` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t15/mcp.json` | 23 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t15/sources.json` | 44 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t16/server.py` | 18 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t16/sources.json` | 40 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t17/sources.json` | 60 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t18/README.md` | 27 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t18/http_server.py` | 86 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t18/server.py` | 35 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t19/server.py` | 18 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t19/source.json` | 10 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t20/README.md` | 22 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t20/bridge.py` | 25 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t21/README.md` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t21/http_server.py` | 89 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t21/result.json` | 14 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t21/server.py` | 56 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t22/README.md` | 22 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t22/http_server.py` | 19 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t22/server.py` | 37 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t23/README.md` | 30 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t23/http_server.py` | 64 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t23/server.py` | 49 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t24/README.md` | 8 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t24/server.py` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t25/manifest.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t25/server.py` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t26/README.md` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t27/README.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t27/server.py` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t28/README.json` | 28 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t29/resource.json` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t29/server.py` | 20 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t29/source.json` | 16 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t30/README.md` | 10 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t30/bridge.html` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t30/server.py` | 22 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t31/README.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t31/app.html` | 6 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t31/server.py` | 12 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t32/hooks.json` | 254 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t32/provenance.json` | 54 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t33/hooks.json` | 46 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t33/source.json` | 10 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t34/README.md` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t34/capture.sh` | 3 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t35/README.md` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t36/README.md` | 12 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t37/source.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t38/source.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t39/source.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t40/README.md` | 18 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t41/source.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t42/source.json` | 11 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t43/source.json` | 24 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t44/README.md` | 20 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t44/source.json` | 33 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t45/README.md` | 11 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t45/server.py` | 22 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t46/README.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t47/README.md` | 10 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t48/README.md` | 6 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t49/README.md` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t50/README.md` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t51/README.md` | 5 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t52/README.md` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t53/README.md` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t54/README.md` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t55/README.md` | 7 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t56/README.md` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t57/README.md` | 13 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t58/README.md` | 11 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t59/README.md` | 13 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t60/source.json` | 9 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/package/mcp.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/package/plugin.json` | 1 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/package/server.py` | 19 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/context7-LICENSE` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/context7.tar.gz` | 90 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/notify-LICENSE` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/notify.tar.gz` | 11511 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/pi-LICENSE` | 21 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/pi-agent-plugins-smoke.tar.gz` | 48 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/sources.json` | 388 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/superpowers-6.3.0.tar.gz` | 5625 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/public/superpowers-7.0.1.tar.gz` | 14092 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t61/source.json` | 10 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t64/sources.json` | 44 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t66/baseline.json` | 29 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t66/complete.json` | 35 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/plugin_compat/t66/sources.json` | 17 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/workflow/talking_head.yaml` | 70 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/workflow/talking_head_cloud.yaml` | 159 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/workflow/video_edit/synthetic_300.json` | 1529 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/workflow/video_edit/synthetic_600.json` | 3029 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `testdata/workflow/video_edit/synthetic_cloud_receipt.json` | 6 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/acp_gemini_chat.rs` | 148 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/cc04_accept_transactions.rs` | 366 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/cc_route_030_outbound_scan.rs` | 273 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/model_protocol_014_messages.rs` | 60 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/model_protocol_015_messages.rs` | 44 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/shell_attack_vectors.rs` | 71 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |
| `tests/shell_integration.rs` | 237 | 主包清单/集成测试/资源；任务364收口保持发布入口及原测试调用 |

## 模块声明与资源路径证据

以下为全部模块声明、嵌入资源与共享状态所在文件的行级定位；迁移时依父模块解析相对路径，资源随生产所有者迁移。

### crates/vw-agent/src/acp/backend.rs

- `6: use std::sync::Mutex;`
- `31: #[path = "models.rs"]`
- `32: mod models;`
- `40: operation: Arc<tokio::sync::Mutex<()>>,`
- `41: model: Arc<tokio::sync::Mutex<String>>,`
- `47: sessions: Mutex<HashMap<String, BoundSession>>,`
- `53: Self { tenant, sessions: Mutex::new(HashMap::new()) }`
- `73: .unwrap_or_else(|| Arc::new(tokio::sync::Mutex::new(stored_model)));`

### crates/vw-agent/src/acp/mod.rs

- `3: mod approval;`
- `4: mod backend;`
- `5: mod mcp;`
- `6: mod mount_store;`
- `7: pub mod mounts;`
- `12: #[path = "backend_tests.rs"]`
- `13: mod backend_tests;`
- `16: #[path = "approval_tests.rs"]`
- `17: mod approval_tests;`
- `20: #[path = "mcp_tests.rs"]`
- `21: mod mcp_tests;`
- `24: mod mounts_tests;`
- `27: mod mount_update_tests;`
- `30: mod mount_recovery_tests;`
- `33: mod mount_acceptance_tests;`
- `35: mod catalog_mounts;`
- `38: mod catalog_mounts_tests;`

### crates/vw-agent/src/acp/models.rs

- `94: #[path = "model_tests.rs"]`
- `95: mod tests;`

### crates/vw-agent/src/agent/agent/mod.rs

- `11: mod builder;`
- `12: mod core;`
- `13: mod run;`
- `14: mod turn;`
- `17: #[path = "builder_tests.rs"]`
- `18: mod builder_tests;`
- `20: #[path = "core_tests.rs"]`
- `21: mod core_tests;`
- `23: #[path = "run_tests.rs"]`
- `24: mod run_tests;`
- `26: #[path = "turn_tests.rs"]`
- `27: mod turn_tests;`
- `38: #[path = "tests.rs"]`
- `39: mod tests;`

### crates/vw-agent/src/agent/agent/run_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `21: messages: Arc<Mutex<Vec<Vec<ChatMessage>>>>,`
- `26: Self { messages: Arc::new(Mutex::new(Vec::new())) }`
- `116: events: Mutex<Vec<ObserverEvent>>,`
- `121: Self { events: Mutex::new(Vec::new()) }`
- `220: let captured_config = Arc::new(Mutex::new(None::<Config>));`

### crates/vw-agent/src/agent/agent/tests.rs

- `21: //! 所有 Mock 对象使用 parking_lot::Mutex 保护内部状态，支持多线程测试场景。`
- `39: mod tests {`
- `41: use parking_lot::Mutex;`
- `69: /// 内部使用 parking_lot::Mutex 保护响应队列，支持异步上下文中的安全访问。`
- `77: ///     responses: Mutex::new(vec![`
- `98: responses: Mutex<Vec<crate::app::agent::providers::ChatResponse>>,`
- `209: /// let seen_models = Arc::new(Mutex::new(Vec::new()));`
- `213: ///     responses: Mutex::new(vec![...]),`
- `227: /// 与 MockProvider 相同，使用 parking_lot::Mutex 保护内部状态。`
- `228: /// seen_models 使用 Arc<Mutex<...>> 以便测试代码能够访问捕获的数据。`
- `233: responses: Mutex<Vec<crate::app::agent::providers::ChatResponse>>,`
- `244: seen_models: Arc<Mutex<Vec<String>>>,`
- `463: responses: Mutex::new(vec![crate::app::agent::providers::ChatResponse {`
- `568: responses: Mutex::new(vec![`
- `690: let seen_models = Arc::new(Mutex::new(Vec::new()));`
- `694: responses: Mutex::new(vec![crate::app::agent::providers::ChatResponse {`

### crates/vw-agent/src/agent/agent/turn_tests.rs

- `13: use parking_lot::Mutex;`
- `28: state: Arc<Mutex<ProviderState>>,`
- `34: state: Arc::new(Mutex::new(ProviderState {`
- `60: state: Arc<Mutex<ProviderState>>,`
- `108: stores: Mutex<Vec<(String, String, MemoryCategory, Option<String>)>>,`
- `565: #[path = "turn_skills_tests.rs"]`
- `566: mod skills_tests;`

### crates/vw-agent/src/agent/classifier/mod.rs

- `170: #[path = "tests.rs"]`
- `171: mod tests;`

### crates/vw-agent/src/agent/classifier/tests.rs

- `22: mod tests {`

### crates/vw-agent/src/agent/dispatcher/mod.rs

- `569: #[path = "mod_tests.rs"]`
- `570: mod tests;`

### crates/vw-agent/src/agent/dispatcher/tests.rs

- `23: mod tests {`

### crates/vw-agent/src/agent/loop_/approval.rs

- `68: #[path = "approval_tests.rs"]`
- `69: mod approval_tests;`

### crates/vw-agent/src/agent/loop_/context.rs

- `13: #[path = "context_tests.rs"]`
- `14: mod context_tests;`

### crates/vw-agent/src/agent/loop_/context_tests.rs

- `6: use std::sync::Mutex;`
- `13: recalls: Mutex<Vec<(String, usize, Option<String>)>>,`

### crates/vw-agent/src/agent/loop_/core/constants.rs

- `219: #[path = "constants_tests.rs"]`
- `220: mod constants_tests;`

### crates/vw-agent/src/agent/loop_/core/errors.rs

- `99: #[path = "errors_tests.rs"]`
- `100: mod errors_tests;`

### crates/vw-agent/src/agent/loop_/core/history.rs

- `25: #[path = "history_tests.rs"]`
- `26: mod history_tests;`

### crates/vw-agent/src/agent/loop_/core/mod.rs

- `21: mod constants;`
- `22: mod errors;`
- `23: mod history;`
- `24: pub(crate) mod skills;`
- `25: mod timeouts;`
- `26: mod tool_loop;`
- `27: mod turn;`

### crates/vw-agent/src/agent/loop_/core/timeouts.rs

- `37: #[path = "timeouts_tests.rs"]`
- `38: mod timeouts_tests;`

### crates/vw-agent/src/agent/loop_/core/tool_loop/detection.rs

- `2: use std::sync::LazyLock;`
- `5: #[path = "detection_tests.rs"]`
- `6: mod detection_tests;`
- `9: static ACTION_COMPLETION_CUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {`
- `17: static SIDE_EFFECT_ACTION_VERB_REGEX: LazyLock<Regex> = LazyLock::new(|| {`
- `25: static SIDE_EFFECT_ACTION_OBJECT_REGEX: LazyLock<Regex> = LazyLock::new(|| {`

### crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs

- `7: use std::sync::Mutex;`
- `82: responses: Arc<Mutex<VecDeque<crate::providers::ChatResponse>>>,`
- `83: requests: Arc<Mutex<Vec<DiscoveryRequest>>>,`
- `128: responses: Arc::new(Mutex::new(responses)),`

### crates/vw-agent/src/agent/loop_/core/tool_loop/llm_round.rs

- `28: #[path = "llm_round_tests.rs"]`
- `29: mod llm_round_tests;`

### crates/vw-agent/src/agent/loop_/core/tool_loop/llm_round_tests.rs

- `11: use std::sync::{Arc, Mutex};`
- `293: let hook_models = Arc::new(Mutex::new(Vec::new()));`
- `415: result: Mutex<Option<Result<ChatResponse>>>,`
- `417: calls: Mutex<usize>,`
- `418: last_tools_len: Mutex<Option<usize>>,`
- `419: last_messages: Mutex<Vec<ChatMessage>>,`
- `425: models: Arc<Mutex<Vec<String>>>,`
- `483: result: Mutex::new(Some(result)),`
- `485: calls: Mutex::new(0),`
- `486: last_tools_len: Mutex::new(None),`
- `487: last_messages: Mutex::new(Vec::new()),`
- `547: events: Arc<Mutex<Vec<ObserverEvent>>>,`

### crates/vw-agent/src/agent/loop_/core/tool_loop/output.rs

- `14: #[path = "output_tests.rs"]`
- `15: mod output_tests;`

### crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution.rs

- `31: #[path = "tool_execution_tests.rs"]`
- `32: mod tool_execution_tests;`

### crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs

- `23: received_args: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,`
- `617: let received_args = Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/agent/loop_/core/tool_loop.rs

- `6: mod collaboration;`
- `7: mod detection;`
- `8: mod llm_round;`
- `9: mod output;`
- `10: mod tool_execution;`
- `13: #[path = "tool_loop/discovery_tests.rs"]`
- `14: mod discovery_tests;`
- `17: #[path = "tool_loop_tests.rs"]`
- `18: mod tool_loop_tests;`

### crates/vw-agent/src/agent/loop_/core/tool_loop_tests.rs

- `9: use std::sync::Mutex;`
- `15: responses: Mutex<VecDeque<crate::app::agent::providers::ChatResponse>>,`
- `17: requested_tools: Arc<Mutex<Vec<Vec<String>>>>,`
- `39: responses: Mutex::new(responses.into()),`
- `41: requested_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/agent/loop_/core/turn.rs

- `33: #[path = "turn_tests.rs"]`
- `34: mod turn_tests;`

### crates/vw-agent/src/agent/loop_/core/turn_tests.rs

- `9: use std::sync::Mutex;`
- `13: responses: Mutex<VecDeque<crate::app::agent::providers::ChatResponse>>,`
- `15: requests: Arc<Mutex<Vec<RecordedRequest>>>,`
- `31: Self { responses: Mutex::new(responses.into()), calls, requests: Arc::default() }`

### crates/vw-agent/src/agent/loop_/cron.rs

- `31: #[path = "cron_tests.rs"]`
- `32: mod cron_tests;`

### crates/vw-agent/src/agent/loop_/execution.rs

- `31: #[path = "execution_tests.rs"]`
- `32: mod execution_tests;`

### crates/vw-agent/src/agent/loop_/execution_tests.rs

- `9: use std::sync::Mutex;`
- `13: events: Mutex<Vec<ObserverEvent>>,`
- `18: Self { events: Mutex::new(Vec::new()) }`

### crates/vw-agent/src/agent/loop_/history.rs

- `21: #[path = "history_tests.rs"]`
- `22: mod history_tests;`

### crates/vw-agent/src/agent/loop_/instructions.rs

- `11: #[path = "instructions_tests.rs"]`
- `12: mod instructions_tests;`

### crates/vw-agent/src/agent/loop_/parsing/deepseek_dsml.rs

- `6: use std::sync::LazyLock;`
- `8: static TAG_RE: LazyLock<Regex> =`
- `9: LazyLock::new(|| Regex::new(r"(?i)<(/?)[｜|]+DSML[｜|]+\s*([a-z_]+)\b([^>]*)>").unwrap());`
- `10: static PARAMETER_CLOSE_RE: LazyLock<Regex> =`
- `11: LazyLock::new(|| Regex::new(r"(?i)</[｜|]+DSML[｜|]+\s*parameter\s*>").unwrap());`
- `12: static ATTRIBUTE_RE: LazyLock<Regex> =`
- `13: LazyLock::new(|| Regex::new(r#"\s+([a-z_]+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap());`
- `16: static MARKER_RE: LazyLock<Regex> =`
- `17: LazyLock::new(|| Regex::new(r"(?i)</?[｜|]+DSML[｜|]+").unwrap());`

### crates/vw-agent/src/agent/loop_/parsing/json.rs

- `16: #[path = "json_tests.rs"]`
- `17: mod json_tests;`

### crates/vw-agent/src/agent/loop_/parsing/minimax_xml.rs

- `39: use std::sync::LazyLock;`
- `42: #[path = "minimax_xml_tests.rs"]`
- `43: mod minimax_xml_tests;`
- `68: static MINIMAX_INVOKE_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `95: static MINIMAX_PARAMETER_RE: LazyLock<Regex> = LazyLock::new(|| {`

### crates/vw-agent/src/agent/loop_/parsing/mod.rs

- `24: mod deepseek_dsml;`
- `25: mod json;`
- `26: mod minimax_xml;`
- `27: mod tool_call_formats;`
- `28: mod tool_calls;`
- `29: mod xml_helpers;`
- `62: #[path = "tests.rs"]`
- `63: mod tests;`
- `65: #[path = "tool_calls_tests.rs"]`
- `66: mod tool_calls_tests;`
- `68: #[path = "xml_helpers_tests.rs"]`
- `69: mod xml_helpers_tests;`

### crates/vw-agent/src/agent/loop_/parsing/tool_call_formats.rs

- `44: use std::sync::LazyLock;`
- `47: #[path = "tool_call_formats_tests.rs"]`
- `48: mod tool_call_formats_tests;`
- `410: static INVOKE_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `415: static PARAM_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `480: static PERL_RE: LazyLock<Regex> =`
- `481: LazyLock::new(|| Regex::new(r"(?s)TOOL_CALL\s*\{(.+?)\}\}\s*/TOOL_CALL").unwrap());`
- `484: static TOOL_NAME_RE: LazyLock<Regex> =`
- `485: LazyLock::new(|| Regex::new(r#"tool\s*=>\s*\"([^\"]+)\""#).unwrap());`
- `488: static ARGS_BLOCK_RE: LazyLock<Regex> =`
- `489: LazyLock::new(|| Regex::new(r"(?s)args\s*=>\s*\{(.+?)\}").unwrap());`
- `492: static ARGS_RE: LazyLock<Regex> =`
- `493: LazyLock::new(|| Regex::new(r#"--(\w+)\s+\"([^\"]+)\""#).unwrap());`
- `562: static FUNC_RE: LazyLock<Regex> = LazyLock::new(|| {`

### crates/vw-agent/src/agent/loop_/parsing/tool_calls.rs

- `24: use std::sync::LazyLock;`
- `312: static MD_TOOL_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `353: static MD_TOOL_NAME_RE: LazyLock<Regex> =`
- `354: LazyLock::new(|| Regex::new(r"(?s)tool\s+(\w+)\s*\n(.*?)(?:|$)").unwrap());`

### crates/vw-agent/src/agent/loop_/parsing/xml_helpers.rs

- `23: use std::sync::LazyLock;`
- `65: static XML_OPEN_TAG_RE: LazyLock<Regex> =`
- `66: LazyLock::new(|| Regex::new(r"<([a-zA-Z_][a-zA-Z0-9_-]*)>").unwrap());`

### crates/vw-agent/src/agent/loop_/progress.rs

- `342: #[path = "progress_tests.rs"]`
- `343: mod progress_tests;`

### crates/vw-agent/src/agent/loop_/query_engine.rs

- `23: use std::sync::{Arc, Mutex};`
- `113: state: Mutex<RecordingObserverState>,`
- `118: Self { inner, state: Mutex::new(RecordingObserverState::default()) }`
- `653: #[path = "query_engine_tests.rs"]`
- `654: mod query_engine_tests;`

### crates/vw-agent/src/agent/loop_/query_engine_tests.rs

- `17: use std::sync::{Arc, Mutex};`
- `99: steps: Mutex<VecDeque<ProviderStep>>,`
- `101: seen_messages: Mutex<Vec<Vec<ChatMessage>>>,`
- `102: seen_models: Mutex<Vec<String>>,`
- `103: seen_temperatures: Mutex<Vec<f64>>,`
- `104: seen_tools: Mutex<Vec<Vec<String>>>,`
- `116: steps: Mutex::new(steps.into_iter().collect()),`
- `188: events: Mutex<Vec<String>>,`
- `1030: #[path = "query_engine_skills_tests.rs"]`
- `1031: mod skills_tests;`

### crates/vw-agent/src/agent/loop_/runner.rs

- `69: #[path = "runner_tests.rs"]`
- `70: mod runner_tests;`

### crates/vw-agent/src/agent/loop_/runner_tests.rs

- `10: use std::sync::OnceLock;`
- `13: use tokio::sync::Mutex;`
- `16: fn env_lock() -> &'static Mutex<()> {`
- `17: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `18: LOCK.get_or_init(|| Mutex::new(()))`

### crates/vw-agent/src/agent/loop_/tests/loop_tools.rs

- `11: use std::sync::{Arc, Mutex};`
- `19: mod approvals;`
- `20: mod multimodal;`
- `21: mod native_mode;`
- `22: mod parallel_execution;`
- `23: mod recovery;`
- `92: responses: Arc<Mutex<VecDeque<ChatResponse>>>,`
- `109: responses: Arc::new(Mutex::new(scripted)),`

### crates/vw-agent/src/agent/loop_/tests/mod.rs

- `60: mod constants;`
- `61: mod cron_delivery;`
- `62: mod loop_tools;`
- `63: mod memory_context;`
- `64: mod parsing_edge_cases;`
- `65: mod parsing_extract_json;`
- `66: mod parsing_glm;`
- `67: mod parsing_protocols;`
- `68: mod parsing_recovery;`
- `69: mod parsing_structured;`
- `70: mod parsing_tool_calls;`
- `71: mod parsing_value;`
- `72: mod shell_policy;`
- `73: mod tool_instructions;`

### crates/vw-agent/src/agent/loop_/utils.rs

- `13: use std::sync::LazyLock;`
- `28: pub(crate) static SENSITIVE_KEY_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {`
- `54: pub(crate) static SENSITIVE_KV_REGEX: LazyLock<Regex> = LazyLock::new(|| {`
- `66: pub(crate) static TOOL_FILE_PATH_REGEX: LazyLock<Regex> =`
- `67: LazyLock::new(|| Regex::new(r#"(?i)(?:file_path|path)\s*[:=]\s*"([^"]+)""#).unwrap());`

### crates/vw-agent/src/agent/loop_.rs

- `39: pub mod approval;`
- `42: pub mod context;`
- `45: pub mod core;`
- `48: pub mod cron;`
- `51: pub mod execution;`
- `54: pub mod history;`
- `57: pub mod instructions;`
- `60: pub mod parsing;`
- `63: pub mod progress;`
- `66: pub mod query_engine;`
- `69: pub mod runner;`
- `72: pub mod utils;`
- `76: mod tests;`
- `78: mod utils_tests;`

### crates/vw-agent/src/agent/memory_loader.rs

- `213: #[path = "memory_loader_tests.rs"]`
- `214: mod memory_loader_tests;`
- `216: #[path = "tests/memory_loader.rs"]`
- `217: mod tests;`

### crates/vw-agent/src/agent/memory_loader_tests.rs

- `7: use std::sync::Mutex;`
- `12: queries: Mutex<Vec<String>>,`
- `13: limits: Mutex<Vec<usize>>,`
- `21: queries: Mutex::new(Vec::new()),`
- `22: limits: Mutex::new(Vec::new()),`
- `30: queries: Mutex::new(Vec::new()),`
- `31: limits: Mutex::new(Vec::new()),`

### crates/vw-agent/src/agent/mod.rs

- `28: pub mod agent;`
- `29: pub mod catalog;`
- `31: mod catalog_tests;`
- `32: pub mod classifier;`
- `33: pub mod defaults;`
- `35: mod defaults_tests;`
- `36: pub mod dispatcher;`
- `37: pub mod loop_;`
- `38: pub mod memory_loader;`
- `39: pub mod prompt;`
- `42: #[path = "mod_tests.rs"]`
- `43: mod mod_tests;`
- `45: mod tests;`
- `68: const PROMPT_GENERATE: &str = include_str!("generate.txt");`
- `73: const PROMPT_EXPLORE: &str = include_str!("prompt/explore.txt");`
- `76: const PROMPT_RESEARCHER: &str = include_str!("prompt/researcher.txt");`
- `79: const PROMPT_CODER: &str = include_str!("prompt/coder.txt");`
- `82: const PROMPT_REVIEWER: &str = include_str!("prompt/reviewer.txt");`

### crates/vw-agent/src/agent/prompt/mod.rs

- `49: pub(crate) mod multi_agent;`
- `660: #[path = "mod_tests.rs"]`
- `661: mod mod_tests;`
- `663: #[path = "tests.rs"]`
- `664: mod tests;`

### crates/vw-agent/src/agent/prompt/tests.rs

- `30: mod tests {`

### crates/vw-agent/src/agent/tests/helpers.rs

- `15: //! - 使用 Mutex 保证线程安全的可变状态`
- `27: use std::sync::{Arc, Mutex};`
- `50: responses: Mutex<Vec<ChatResponse>>,`
- `52: requests: Mutex<Vec<Vec<ChatMessage>>>,`
- `66: Self { responses: Mutex::new(responses), requests: Mutex::new(Vec::new()) }`
- `327: count: Arc<Mutex<usize>>,`
- `345: pub(super) fn new() -> (Self, Arc<Mutex<usize>>) {`
- `346: let count = Arc::new(Mutex::new(0));`

### crates/vw-agent/src/agent/tests/mod.rs

- `22: mod helpers;`
- `23: mod loop_history;`
- `24: mod loop_memory;`
- `25: mod loop_text;`
- `26: mod loop_tools;`
- `27: mod run_single;`
- `28: mod serialization;`
- `29: mod tool_dispatcher;`

### crates/vw-agent/src/agent_apps/auth_accounts.rs

- `43: mod defaults;`
- `45: mod logout;`
- `47: mod read;`
- `51: mod removal;`
- `961: #[path = "auth_accounts_tests.rs"]`
- `962: mod tests;`
- `1119: mod gemini;`

### crates/vw-agent/src/agent_apps/mcp/binding.rs

- `6: sync::{Mutex, MutexGuard},`
- `75: static LOCKS: [Mutex<()>; 64] = [const { Mutex::new(()) }; 64];`

### crates/vw-agent/src/agent_apps/mcp/credentials.rs

- `129: mod execution {`

### crates/vw-agent/src/agent_apps/mcp/mod.rs

- `15: mod catalog_create;`
- `16: mod catalog_delete;`
- `17: mod catalog_update;`
- `18: mod credentials;`
- `21: mod connection_probe_tests;`
- `23: mod connectivity;`
- `25: mod connectivity_tests;`
- `27: pub mod network_probe;`
- `29: mod network_probe_tests;`
- `31: pub mod probe_jobs;`
- `33: mod probe_jobs_tests;`
- `35: mod stdio_probe_tests;`
- `38: mod catalog_create_tests;`
- `40: mod catalog_delete_tests;`
- `42: mod catalog_update_tests;`
- `44: mod credentials_tests;`
- `158: mod import;`
- `160: mod import_tests;`
- `166: mod binding;`
- `168: mod binding_tests;`
- `174: mod bulk;`
- `176: mod bulk_tests;`
- `179: mod resync;`
- `181: mod resync_tests;`
- `187: pub(crate) mod revocation;`
- `189: mod revocation_tests;`
- `191: pub(crate) mod runtime;`
- `193: mod runtime_tests;`
- `195: pub(crate) mod acp_mount;`
- `197: mod acp_mount_tests;`
- `200: mod tool_details_tests;`
- `203: mod security_tests;`
- `206: mod host_scope_tests;`
- `209: pub(crate) mod profile_slot;`
- `212: mod data_migration;`
- `217: mod data_migration_tests;`

### crates/vw-agent/src/agent_apps/mcp/network_probe.rs

- `458: mod details;`

### crates/vw-agent/src/agent_apps/mcp/network_probe_tests.rs

- `175: methods: Arc<parking_lot::Mutex<Vec<String>>>,`

### crates/vw-agent/src/agent_apps/mcp/revocation_tests.rs

- `163: fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/http_server.py"))?;`

### crates/vw-agent/src/agent_apps/mod.rs

- `2: pub mod mcp;`
- `3: pub mod store;`
- `6: mod store_tests;`
- `9: mod sqlite_tests;`
- `11: mod sqlite;`
- `14: mod postgres_tests;`
- `16: mod postgres;`
- `18: pub mod auth_accounts;`
- `19: pub mod usage_settings;`
- `22: mod usage_settings_tests;`
- `24: pub mod prompts_store;`
- `27: pub mod shared_snapshot;`
- `29: pub mod shared_snapshot_import;`
- `31: pub mod preferences_import;`
- `34: pub(crate) mod data_import;`

### crates/vw-agent/src/agent_apps/prompts_store.rs

- `266: #[path = "prompt_activation_store.rs"]`
- `267: mod activation_store;`
- `270: #[path = "prompt_deactivation_store.rs"]`
- `271: mod deactivation_store;`
- `274: #[path = "prompts_bootstrap.rs"]`
- `275: mod bootstrap;`
- `281: #[path = "prompt_sync_store.rs"]`
- `282: mod sync_store;`
- `285: #[path = "prompt_toggle_store.rs"]`
- `286: mod toggle_store;`
- `289: #[path = "profile_prompt_store.rs"]`
- `290: mod profile_prompt_store;`
- `293: #[path = "prompt_data_migration.rs"]`
- `294: mod data_migration;`
- `298: #[path = "prompt_data_migration_tests.rs"]`
- `299: mod data_migration_tests;`

### crates/vw-agent/src/agent_scheduler/adapters/mod.rs

- `4: pub mod dispatch;`
- `5: pub mod local;`
- `8: #[path = "local_tests.rs"]`
- `9: mod local_tests;`

### crates/vw-agent/src/agent_scheduler/engine/mod.rs

- `4: pub mod dispatch;`
- `5: pub mod recovery;`

### crates/vw-agent/src/agent_scheduler/events.rs

- `138: #[path = "events_tests.rs"]`
- `139: mod tests;`

### crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs

- `21: requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,`
- `26: Self { output: output.to_string(), requests: std::sync::Mutex::new(Vec::new()) }`
- `99: requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,`

### crates/vw-agent/src/agent_scheduler/mod.rs

- `9: pub mod adapters;`
- `10: pub mod artifacts;`
- `11: pub mod domain;`
- `12: pub mod engine;`
- `13: pub mod events;`
- `14: pub mod lifecycle_bridge;`
- `15: pub mod outbox_dispatcher;`
- `16: pub mod policy;`
- `17: pub mod resources;`
- `18: pub mod runner;`
- `19: pub mod scheduler;`
- `20: pub mod state_machine;`
- `21: pub mod store;`
- `22: pub mod task_board;`
- `23: pub mod task_graph;`
- `26: #[path = "task_board_tests.rs"]`
- `27: mod task_board_tests;`
- `30: #[path = "domain_tests.rs"]`
- `31: mod domain_tests;`
- `34: #[path = "state_machine_tests.rs"]`
- `35: mod state_machine_tests;`
- `38: #[path = "store/schema_tests.rs"]`
- `39: mod schema_tests;`
- `42: #[path = "store/tasks_tests.rs"]`
- `43: mod tasks_tests;`
- `46: #[path = "store/runs_tests.rs"]`
- `47: mod runs_tests;`
- `50: #[path = "store/resources_tests.rs"]`
- `51: mod resources_tests;`
- `54: #[path = "store/events_tests.rs"]`
- `55: mod events_tests;`
- `58: #[path = "store/outbox_tests.rs"]`
- `59: mod store_outbox_tests;`
- `62: #[path = "store/logs_tests.rs"]`
- `63: mod store_logs_tests;`
- `66: #[path = "store/artifacts_tests.rs"]`
- `67: mod store_artifacts_tests;`
- `70: #[path = "policy_tests.rs"]`
- `71: mod policy_tests;`
- `74: #[path = "lifecycle_bridge_tests.rs"]`
- `75: mod lifecycle_bridge_tests;`
- `78: #[path = "artifacts_tests.rs"]`
- `79: mod artifacts_tests;`
- `82: #[path = "integration_tests.rs"]`
- `83: mod integration_tests;`
- `86: #[path = "local_e2e_tests.rs"]`
- `87: mod local_e2e_tests;`
- `90: #[path = "runner/local_tests.rs"]`
- `91: mod local_runner_tests;`
- `94: #[path = "task_graph_tests.rs"]`
- `95: mod task_graph_tests;`

### crates/vw-agent/src/agent_scheduler/policy_tests.rs

- `330: let source = include_str!("policy.rs");`

### crates/vw-agent/src/agent_scheduler/resources/mod.rs

- `11: pub mod worktree;`
- `41: #[path = "worktree_tests.rs"]`
- `42: mod worktree_tests;`

### crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs

- `639: #[path = "gateway_executor_tests.rs"]`
- `640: mod gateway_executor_tests;`

### crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs

- `8: use parking_lot::Mutex;`
- `655: results: Mutex<VecDeque<Result<GatewayChatStreamCollected, String>>>,`
- `656: payloads: Mutex<VecDeque<Vec<Value>>>,`
- `657: calls: Mutex<Vec<(String, Option<String>, Vec<Value>)>>,`
- `686: queue: &Mutex<VecDeque<Result<GatewayChatStreamCollected, String>>>,`
- `698: acquire_results: Mutex<VecDeque<Result<TaskPoolWorktreeAcquireResponse, String>>>,`
- `699: release_results: Mutex<VecDeque<Result<TaskPoolWorktreeReleaseResponse, String>>>,`
- `700: recycle_results: Mutex<VecDeque<Result<TaskPoolWorktreeRecycleResponse, String>>>,`
- `701: acquire_calls: Mutex<Vec<(String, String)>>,`
- `702: release_calls: Mutex<Vec<(String, String)>>,`
- `703: recycle_calls: Mutex<Vec<(String, String, Option<String>)>>,`
- `736: fn pop_pool_result<T>(queue: &Mutex<VecDeque<Result<T, String>>>, operation: &str) -> Result<T> {`

### crates/vw-agent/src/agent_scheduler/runner/local_tests.rs

- `633: requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,`
- `634: workspace_requests: std::sync::Mutex<Vec<LocalAgentWorkspaceRequest>>,`
- `639: recycled: std::sync::Mutex<Vec<LocalAgentWorkspaceRecycleRequest>>,`
- `655: requests: std::sync::Mutex::new(Vec::new()),`
- `656: workspace_requests: std::sync::Mutex::new(Vec::new()),`
- `664: requests: std::sync::Mutex::new(Vec::new()),`
- `665: workspace_requests: std::sync::Mutex::new(Vec::new()),`
- `673: requests: std::sync::Mutex::new(Vec::new()),`
- `674: workspace_requests: std::sync::Mutex::new(Vec::new()),`
- `681: Self { workspace_path, recycled: std::sync::Mutex::new(Vec::new()) }`

### crates/vw-agent/src/agent_scheduler/runner/mod.rs

- `6: pub(crate) mod gateway_executor;`
- `7: pub mod local;`
- `8: pub(crate) mod workspace_claim;`
- `10: mod workspace_claim_tests;`

### crates/vw-agent/src/agent_scheduler/runner/workspace_claim.rs

- `7: use std::sync::{LazyLock, Mutex};`
- `22: static STARTED: LazyLock<Mutex<HashMap<ClaimKey, WorkspaceClaim>>> =`
- `23: LazyLock::new(|| Mutex::new(HashMap::new()));`

### crates/vw-agent/src/agent_scheduler/scheduler/mod.rs

- `4: pub mod recurring;`
- `7: mod recurring_tests;`

### crates/vw-agent/src/agent_scheduler/store/mod.rs

- `5: use std::sync::{Arc, Mutex};`
- `17: pub mod artifacts;`
- `18: pub mod dependencies;`
- `19: pub mod events;`
- `20: pub mod logs;`
- `21: pub mod outbox;`
- `22: mod postgres;`
- `23: pub mod resources;`
- `24: pub mod runs;`
- `25: pub(super) mod schema;`
- `26: pub mod tasks;`
- `32: database: Arc<Mutex<Option<GlobalDatabase>>>,`
- `55: static TEST_HOME: std::sync::LazyLock<std::sync::Arc<tempfile::TempDir>> =`
- `56: std::sync::LazyLock::new(|| {`
- `122: database: Arc::new(Mutex::new(None)),`
- `134: database: Arc::new(Mutex::new(None)),`
- `147: database: Arc::new(Mutex::new(Some(database))),`
- `161: database: Arc::new(Mutex::new(None)),`
- `332: #[path = "dependencies_tests.rs"]`
- `333: mod dependencies_tests;`
- `335: #[path = "mod_tests.rs"]`
- `336: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/artifacts.rs

- `176: #[path = "artifacts_tests.rs"]`
- `177: mod artifacts_tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/dependencies.rs

- `229: #[path = "dependencies_tests.rs"]`
- `230: mod dependencies_tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/events.rs

- `179: #[path = "events_tests.rs"]`
- `180: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/logs.rs

- `363: #[path = "logs_tests.rs"]`
- `364: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/mod.rs

- `9: mod artifacts;`
- `10: mod dependencies;`
- `11: mod events;`
- `12: mod logs;`
- `13: mod outbox;`
- `14: mod resources;`
- `15: mod runs;`
- `16: mod schema;`
- `17: mod tasks;`
- `94: #[path = "mod_tests.rs"]`
- `95: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/outbox.rs

- `314: #[path = "outbox_tests.rs"]`
- `315: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/resources.rs

- `356: #[path = "resources_tests.rs"]`
- `357: mod tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/schema.rs

- `78: #[path = "schema_tests.rs"]`
- `79: mod schema_tests;`

### crates/vw-agent/src/agent_scheduler/store/postgres/tasks.rs

- `564: #[path = "tasks_tests.rs"]`
- `565: mod tasks_tests;`

### crates/vw-agent/src/agent_scheduler/store/resources.rs

- `563: #[path = "resources_unit_tests.rs"]`
- `564: mod unit_tests;`

### crates/vw-agent/src/agent_scheduler_policy_legacy_review_exact_tests.rs

- `14: let source = include_str!("agent_scheduler/policy.rs");`

### crates/vw-agent/src/app.rs

- `11: pub(crate) mod agent {`
- `72: pub(crate) mod config {`
- `107: pub(crate) mod task {`
- `696: #[path = "app_tests.rs"]`
- `697: mod app_tests;`

### crates/vw-agent/src/app_providers/model_edits.rs

- `98: #[path = "model_edits_tests.rs"]`
- `99: mod tests;`

### crates/vw-agent/src/app_providers.rs

- `649: #[path = "app_providers_tests.rs"]`
- `650: mod tests;`
- `652: mod universal;`
- `654: mod universal_sync;`
- `656: mod failover;`
- `659: mod failover_tests;`
- `694: mod snippets;`
- `696: mod claude;`
- `699: mod account_removal;`
- `704: mod claude_binding;`
- `711: mod codex_binding;`
- `713: mod codex_binding_tests;`
- `714: mod codex_catalog;`
- `717: mod model_edits;`
- `719: mod official_launch;`
- `721: mod gemini;`
- `723: mod claude_desktop;`
- `724: mod grok;`
- `726: mod desktop_mapping;`
- `728: mod hermes;`
- `729: mod openclaw;`
- `730: mod opencode;`
- `732: mod pi;`
- `734: mod mcode;`
- `735: mod omp;`
- `737: mod kimi;`
- `740: mod kimi_binding;`
- `742: mod kimi_binding_tests;`
- `744: mod dsh;`
- `746: mod agy;`
- `749: mod agy_binding;`
- `752: mod import;`
- `754: mod basic;`
- `757: mod agy_form;`
- `759: mod request_options;`
- `761: pub(crate) mod generation_target;`
- `763: mod record_publication;`
- `765: mod record_publication_tests;`
- `808: mod data_import;`

### crates/vw-agent/src/app_routing/cancellation.rs

- `3: use parking_lot::Mutex;`
- `35: pub(super) struct ClientReader<'a>(pub(super) &'a Mutex<OwnedReadHalf>);`

### crates/vw-agent/src/app_routing/decode_response_tests.rs

- `81: let cases = Arc::new(parking_lot::Mutex::new(VecDeque::new()));`

### crates/vw-agent/src/app_routing/failover_execution.rs

- `565: #[path = "failover_execution_tests.rs"]`
- `566: mod tests;`

### crates/vw-agent/src/app_routing/gateway_owner_tests.rs

- `204: use std::sync::{Mutex, mpsc};`
- `226: let connection = Mutex::new(Some((accepted, cleanup_start, cleanup_release)));`

### crates/vw-agent/src/app_routing/listener.rs

- `12: use parking_lot::Mutex;`
- `72: type ListenerLifecycle = Mutex<Lifecycle<RunningListener>>;`
- `81: control_closed: Arc<tokio::sync::Mutex<bool>>,`
- `85: Mutex<Option<crate::gateway::aggregation_lifecycle::AggregationLifecycle>>,`
- `87: aggregation_resume: Arc<tokio::sync::Mutex<()>>,`

### crates/vw-agent/src/app_routing/mod.rs

- `2: pub(crate) mod aggregation_routing;`
- `3: mod body_limits;`
- `4: mod browser_guard;`
- `5: mod cancellation;`
- `6: mod capacity;`
- `8: pub(crate) mod config_apply;`
- `9: mod decode_request;`
- `10: mod decode_response;`
- `11: pub(crate) mod dispatch;`
- `12: pub(crate) mod gateway_owner;`
- `13: mod header_wire;`
- `14: mod listener;`
- `16: pub(crate) mod outbound_config;`
- `17: mod outbound_custom;`
- `19: mod outbound_system;`
- `20: mod override_apply;`
- `21: mod protocol_errors;`
- `22: pub(crate) mod reference_scope;`
- `23: mod route_auth;`
- `24: mod shared_transport;`
- `25: mod shutdown;`
- `26: mod timeouts;`
- `27: mod transport_security;`
- `34: mod body_limits_tests;`
- `37: mod config_apply_tests;`
- `40: mod listener_tests;`
- `43: mod shared_transport_tests;`
- `46: mod outbound_config_tests;`
- `49: mod outbound_custom_tests;`
- `52: mod outbound_system_tests;`
- `55: mod reference_scope_tests;`
- `58: mod shutdown_tests;`
- `61: mod gateway_owner_tests;`
- `64: mod timeouts_tests;`
- `67: mod cancellation_tests;`
- `70: mod capacity_tests;`
- `73: mod protocol_errors_tests;`
- `76: mod transport_security_tests;`
- `79: mod compaction_tests;`
- `82: pub(crate) mod failover_routing;`
- `85: mod failover_routing_tests;`
- `88: pub(crate) mod provider_publication;`
- `91: pub(crate) mod desktop_routing;`
- `94: pub mod adapter_selection;`
- `97: mod failover_execution;`
- `99: pub(crate) mod request_status;`

### crates/vw-agent/src/app_routing/outbound_config.rs

- `6: use parking_lot::RwLock;`
- `126: pub(super) struct EgressState(RwLock<Option<ScopedEgress>>);`

### crates/vw-agent/src/app_routing/protocol_errors_tests.rs

- `17: sync::{Arc, Mutex},`
- `244: struct CapturedLogs(Arc<Mutex<String>>);`

### crates/vw-agent/src/app_routing/reference_scope.rs

- `36: pub(crate) struct References(parking_lot::Mutex<Vec<Reference>>);`

### crates/vw-agent/src/app_routing/request_status.rs

- `4: use parking_lot::Mutex;`
- `15: pub(crate) struct Requests(Mutex<VecDeque<Row>>);`

### crates/vw-agent/src/app_routing/route_auth.rs

- `17: use parking_lot::Mutex;`
- `115: pub(super) actual_route: Mutex<crate::gateway::agent_apps_failover_runtime::ActualRoute>,`
- `117: pub(super) publication: Arc<Mutex<()>>,`
- `385: active: Mutex<ActiveRoutes>,`
- `386: publication: Arc<Mutex<()>>,`
- `733: actual_route: Mutex::default(),`
- `828: let reader = Mutex::new(reader);`
- `832: let connection_state = Mutex::new(Some((send_cancel, wire)));`
- `893: #[path = "route_auth_tests.rs"]`
- `894: mod route_auth_tests;`
- `897: #[path = "browser_guard_tests.rs"]`
- `898: mod browser_guard_tests;`
- `901: #[path = "dispatch_tests.rs"]`
- `902: mod dispatch_tests;`
- `905: #[path = "aggregation_routing_tests.rs"]`
- `906: mod aggregation_routing_tests;`
- `909: #[path = "override_apply_tests.rs"]`
- `910: mod override_apply_tests;`
- `913: #[path = "header_wire_tests.rs"]`
- `914: mod header_wire_tests;`
- `917: #[path = "decode_request_tests.rs"]`
- `918: mod decode_request_tests;`
- `921: #[path = "decode_response_tests.rs"]`
- `922: mod decode_response_tests;`
- `925: #[path = "adapter_selection_tests.rs"]`
- `926: mod adapter_selection_tests;`

### crates/vw-agent/src/app_routing/shutdown_tests.rs

- `130: let fixture = std::sync::Mutex::new(Some((entered, blocked)));`

### crates/vw-agent/src/app_routing/timeouts.rs

- `431: #[path = "timeouts_deadline_tests.rs"]`
- `432: mod deadline_tests;`

### crates/vw-agent/src/app_routing/transport_security.rs

- `5: use std::sync::{Arc, OnceLock};`
- `10: static CONNECTOR: OnceLock<Option<TlsConnector>> = OnceLock::new();`

### crates/vw-agent/src/approval/manager.rs

- `4: use parking_lot::{Mutex, RwLock};`
- `18: /// 所有内部状态都使用 Mutex 或 RwLock 保护，支持多线程并发访问。`
- `32: pub(super) auto_approve: RwLock<HashSet<String>>,`
- `36: pub(super) always_ask: RwLock<HashSet<String>>,`
- `40: pub(super) session_allowlist: Mutex<HashSet<String>>,`
- `42: pub(super) non_cli_allowlist: Mutex<HashSet<String>>,`
- `46: pub(super) non_cli_allow_all_once_remaining: Mutex<u32>,`
- `48: pub(super) non_cli_approval_approvers: RwLock<HashSet<String>>,`
- `50: pub(super) non_cli_natural_language_approval_mode: RwLock<NonCliNaturalLanguageApprovalMode>,`
- `53: RwLock<HashMap<String, NonCliNaturalLanguageApprovalMode>>,`
- `55: pub(super) pending_non_cli_requests: Mutex<HashMap<String, PendingNonCliApprovalRequest>>,`
- `59: pub(super) resolved_non_cli_requests: Mutex<HashMap<String, ApprovalResponse>>,`
- `61: pub(super) audit_log: Mutex<Vec<ApprovalLogEntry>>,`

### crates/vw-agent/src/approval/mod.rs

- `42: mod cli;`
- `43: mod manager;`
- `44: mod pending;`
- `45: mod policy;`
- `46: mod summary;`
- `47: mod types;`
- `70: #[path = "cli_tests.rs"]`
- `71: mod cli_tests;`
- `73: #[path = "manager_tests.rs"]`
- `74: mod manager_tests;`
- `76: #[path = "pending_tests.rs"]`
- `77: mod pending_tests;`
- `79: #[path = "policy_tests.rs"]`
- `80: mod policy_tests;`
- `82: #[path = "summary_tests.rs"]`
- `83: mod summary_tests;`
- `85: pub(crate) mod test_support;`
- `87: #[path = "tests.rs"]`
- `88: mod tests;`
- `90: #[path = "types_tests.rs"]`
- `91: mod types_tests;`

### crates/vw-agent/src/approval/policy.rs

- `40: auto_approve: parking_lot::RwLock::new(config.auto_approve.iter().cloned().collect()),`
- `41: always_ask: parking_lot::RwLock::new(config.always_ask.iter().cloned().collect()),`
- `43: session_allowlist: parking_lot::Mutex::new(HashSet::new()),`
- `44: non_cli_allowlist: parking_lot::Mutex::new(HashSet::new()),`
- `45: non_cli_allow_all_once_remaining: parking_lot::Mutex::new(0),`
- `46: non_cli_approval_approvers: parking_lot::RwLock::new(`
- `49: non_cli_natural_language_approval_mode: parking_lot::RwLock::new(`
- `52: non_cli_natural_language_approval_mode_by_channel: parking_lot::RwLock::new(`
- `57: pending_non_cli_requests: parking_lot::Mutex::new(HashMap::new()),`
- `58: resolved_non_cli_requests: parking_lot::Mutex::new(HashMap::new()),`
- `59: audit_log: parking_lot::Mutex::new(Vec::new()),`

### crates/vw-agent/src/approval/tests.rs

- `30: mod tests {`

### crates/vw-agent/src/auth/mod.rs

- `7: pub mod openai_oauth;`
- `9: #[path = "openai_oauth_tests.rs"]`
- `10: mod openai_oauth_tests;`
- `27: #[path = "mod_tests.rs"]`
- `28: mod mod_tests;`

### crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs

- `118: let requests = Arc::new(Mutex::new(Vec::<Value>::new()));`

### crates/vw-agent/src/channels/clawdtalk/mod.rs

- `597: #[path = "tests.rs"]`
- `598: mod tests;`

### crates/vw-agent/src/channels/clawdtalk/tests.rs

- `19: mod tests {`

### crates/vw-agent/src/channels/cli/mod.rs

- `325: #[path = "tests.rs"]`
- `326: mod tests;`

### crates/vw-agent/src/channels/cli/tests.rs

- `28: mod tests {`

### crates/vw-agent/src/channels/config.rs

- `29: #[path = "config_tests.rs"]`
- `30: mod config_tests;`
- `100: /// 使用 OnceLock 确保存储只初始化一次。`
- `104: /// 返回配置存储的静态引用，类型为 Mutex<HashMap<PathBuf, RuntimeConfigState>>`
- `108: /// - 使用 OnceLock 保证单例初始化的线程安全`
- `109: /// - 使用 Mutex 保护内部 HashMap 的并发访问`
- `118: pub(crate) fn runtime_config_store() -> &'static Mutex<HashMap<PathBuf, RuntimeConfigState>> {`
- `119: static STORE: OnceLock<Mutex<HashMap<PathBuf, RuntimeConfigState>>> = OnceLock::new();`
- `120: STORE.get_or_init(|| Mutex::new(HashMap::new()))`

### crates/vw-agent/src/channels/config_tests.rs

- `13: use std::sync::{Arc, Mutex, MutexGuard, OnceLock};`
- `42: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `43: LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|error| error.into_inner())`
- `113: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `114: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `115: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `128: non_cli_excluded_tools: Arc::new(Mutex::new(excluded_tools)),`

### crates/vw-agent/src/channels/dingtalk/mod.rs

- `54: use tokio::sync::RwLock;`
- `58: mod outbound;`
- `98: session_webhooks: Arc<RwLock<HashMap<String, String>>>,`
- `143: session_webhooks: Arc::new(RwLock::new(HashMap::new())),`
- `554: #[path = "tests.rs"]`
- `555: mod tests;`
- `558: mod outbound_tests;`

### crates/vw-agent/src/channels/dingtalk/outbound_tests.rs

- `22: cache: Option<Arc<RwLock<HashMap<String, String>>>>,`

### crates/vw-agent/src/channels/dingtalk/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/channels/discord/content.rs

- `196: #[path = "content_tests.rs"]`
- `197: mod content_tests;`

### crates/vw-agent/src/channels/discord/gateway.rs

- `267: #[path = "gateway_tests.rs"]`
- `268: mod gateway_tests;`

### crates/vw-agent/src/channels/discord/ids.rs

- `88: #[path = "ids_tests.rs"]`
- `89: mod ids_tests;`

### crates/vw-agent/src/channels/discord/message_split.rs

- `131: #[path = "message_split_tests.rs"]`
- `132: mod message_split_tests;`

### crates/vw-agent/src/channels/discord/mod.rs

- `57: mod attachments;`
- `58: mod attachments_outgoing;`
- `59: mod client;`
- `60: mod content;`
- `61: mod gateway;`
- `62: mod ids;`
- `63: mod message_split;`
- `64: mod permissions;`
- `65: mod reactions;`
- `66: mod typing;`
- `69: #[path = "attachments_outgoing_tests.rs"]`
- `70: mod attachments_outgoing_tests;`
- `72: #[path = "attachments_tests.rs"]`
- `73: mod attachments_tests;`
- `75: #[path = "client_tests.rs"]`
- `76: mod client_tests;`
- `78: #[path = "mod_tests.rs"]`
- `79: mod mod_tests;`
- `882: mod tests;`

### crates/vw-agent/src/channels/discord/permissions.rs

- `106: #[path = "permissions_tests.rs"]`
- `107: mod permissions_tests;`

### crates/vw-agent/src/channels/discord/reactions.rs

- `176: #[path = "reactions_tests.rs"]`
- `177: mod reactions_tests;`

### crates/vw-agent/src/channels/discord/tests/mod.rs

- `18: mod attachments_incoming;`
- `19: mod attachments_outgoing;`
- `20: mod basics;`
- `21: mod message_ids;`
- `22: mod message_split;`
- `23: mod reactions;`
- `24: mod typing;`

### crates/vw-agent/src/channels/discord/typing.rs

- `19: use parking_lot::Mutex;`
- `30: pub(super) type TypingHandles = Mutex<HashMap<String, tokio::task::JoinHandle<()>>>;`
- `39: /// 返回一个空的 TypingHandles 实例，包装在 Mutex 中以支持跨线程共享。`
- `48: Mutex::new(HashMap::new())`
- `182: #[path = "typing_tests.rs"]`
- `183: mod typing_tests;`

### crates/vw-agent/src/channels/email_channel/mod.rs

- `83: use tokio::sync::{Mutex, mpsc};`
- `106: /// - seen_messages：已处理邮件 ID 集合，使用 Arc<Mutex> 实现线程安全的去重机制`
- `110: /// seen_messages 字段使用 Arc<Mutex<HashSet<String>> 包装，支持多任务并发访问，`
- `128: seen_messages: Arc<Mutex<HashSet<String>>>,`
- `155: Self { config, seen_messages: Arc::new(Mutex::new(HashSet::new())) }`
- `884: #[path = "mod_tests.rs"]`
- `885: mod mod_tests;`
- `887: #[path = "tests.rs"]`
- `888: mod tests;`

### crates/vw-agent/src/channels/email_channel/tests.rs

- `22: mod tests {`

### crates/vw-agent/src/channels/format.rs

- `960: #[path = "format_tests.rs"]`
- `961: mod format_tests;`

### crates/vw-agent/src/channels/history.rs

- `208: #[path = "history_tests.rs"]`
- `209: mod history_tests;`

### crates/vw-agent/src/channels/history_tests.rs

- `11: use std::sync::{Arc, Mutex};`
- `60: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `61: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `62: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `72: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/http.rs

- `24: #[path = "http_tests.rs"]`
- `25: mod tests;`

### crates/vw-agent/src/channels/imessage/mod.rs

- `683: #[path = "mod_tests.rs"]`
- `684: mod mod_tests;`
- `686: #[path = "tests.rs"]`
- `687: mod tests;`

### crates/vw-agent/src/channels/imessage/tests.rs

- `28: mod tests {`

### crates/vw-agent/src/channels/irc/mod.rs

- `11: use tokio::sync::{Mutex, mpsc};`
- `40: writer: Arc<Mutex<Option<WriteHalf>>>,`
- `260: writer: Arc::new(Mutex::new(None)),`
- `606: #[path = "mod_tests.rs"]`
- `607: mod mod_tests;`
- `609: #[path = "tests.rs"]`
- `610: mod tests;`

### crates/vw-agent/src/channels/irc/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/channels/lark/ack.rs

- `506: #[path = "ack_tests.rs"]`
- `507: mod ack_tests;`

### crates/vw-agent/src/channels/lark/constants.rs

- `195: #[path = "constants_tests.rs"]`
- `196: mod constants_tests;`

### crates/vw-agent/src/channels/lark/mod.rs

- `40: mod ack;`
- `41: mod constants;`
- `42: mod formatting;`
- `43: mod parsing;`
- `44: mod platform;`
- `45: mod token;`
- `46: mod types;`
- `47: mod webhook;`
- `48: mod ws;`
- `54: use std::sync::{Arc, RwLock as StdRwLock};`
- `56: use tokio::sync::RwLock;`
- `158: tenant_token: Arc<RwLock<Option<CachedTenantToken>>>,`
- `161: ws_seen_ids: Arc<RwLock<HashMap<String, std::time::Instant>>>,`
- `253: tenant_token: Arc::new(RwLock::new(None)),`
- `254: ws_seen_ids: Arc::new(RwLock::new(HashMap::new())),`
- `587: #[path = "mod_tests.rs"]`
- `588: mod mod_tests;`
- `590: #[path = "tests.rs"]`
- `591: mod tests;`
- `594: mod formatting_tests;`

### crates/vw-agent/src/channels/lark/parsing.rs

- `692: #[path = "parsing_tests.rs"]`
- `693: mod parsing_tests;`

### crates/vw-agent/src/channels/lark/platform.rs

- `143: #[path = "platform_tests.rs"]`
- `144: mod platform_tests;`

### crates/vw-agent/src/channels/lark/tests.rs

- `5: #[path = "tests/ack_tests.rs"]`
- `6: mod ack_tests;`
- `7: #[path = "tests/config_tests.rs"]`
- `8: mod config_tests;`
- `9: #[path = "tests/parsing_tests.rs"]`
- `10: mod parsing_tests;`
- `11: #[path = "tests/token_tests.rs"]`
- `12: mod token_tests;`
- `13: #[path = "tests/ws_tests.rs"]`
- `14: mod ws_tests;`

### crates/vw-agent/src/channels/lark/token.rs

- `662: #[path = "token_tests.rs"]`
- `663: mod token_tests;`

### crates/vw-agent/src/channels/lark/token_tests.rs

- `148: #[path = "token_bot_tests.rs"]`
- `149: mod token_bot_tests;`
- `150: #[path = "token_http_tests.rs"]`
- `151: mod token_http_tests;`
- `152: #[path = "token_test_support.rs"]`
- `153: mod token_test_support;`

### crates/vw-agent/src/channels/lark/types.rs

- `198: #[path = "types_tests.rs"]`
- `199: mod types_tests;`

### crates/vw-agent/src/channels/lark/webhook.rs

- `201: #[path = "webhook_tests.rs"]`
- `202: mod webhook_tests;`

### crates/vw-agent/src/channels/lark/ws.rs

- `503: #[path = "ws_tests.rs"]`
- `504: mod ws_tests;`

### crates/vw-agent/src/channels/lark/ws_tests.rs

- `6: #[path = "ws_test_support.rs"]`
- `7: mod ws_test_support;`

### crates/vw-agent/src/channels/linq/mod.rs

- `649: #[path = "tests.rs"]`
- `650: mod tests;`

### crates/vw-agent/src/channels/linq/tests.rs

- `33: mod tests {`

### crates/vw-agent/src/channels/manager/command.rs

- `103: #[path = "command_tests.rs"]`
- `104: mod command_tests;`

### crates/vw-agent/src/channels/manager/dispatch_cancel_tests.rs

- `202: other_ctx.project_state = Arc::new(Mutex::new(other_state));`

### crates/vw-agent/src/channels/manager/dispatch_tests.rs

- `10: use std::sync::{Arc, Mutex};`
- `32: pub(super) sent: tokio::sync::Mutex<Vec<SendMessage>>,`
- `45: sent: tokio::sync::Mutex::new(Vec::new()),`
- `58: sent: tokio::sync::Mutex::new(Vec::new()),`
- `157: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `158: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `159: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `169: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/manager/errors.rs

- `97: #[path = "errors_tests.rs"]`
- `98: mod errors_tests;`

### crates/vw-agent/src/channels/manager/errors_tests.rs

- `9: use std::sync::{Arc, Mutex};`
- `45: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `46: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `47: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `57: non_cli_excluded_tools: Arc::new(Mutex::new(excluded)),`

### crates/vw-agent/src/channels/manager/execution_binding.rs

- `163: #[path = "execution_binding_tests.rs"]`
- `164: mod tests;`

### crates/vw-agent/src/channels/manager/execution_control.rs

- `148: #[path = "execution_control_tests.rs"]`
- `149: mod tests;`

### crates/vw-agent/src/channels/manager/health.rs

- `111: #[path = "health_tests.rs"]`
- `112: mod health_tests;`

### crates/vw-agent/src/channels/manager/listener.rs

- `86: #[path = "listener_tests.rs"]`
- `87: mod listener_tests;`

### crates/vw-agent/src/channels/manager/listener_tests.rs

- `5: use std::sync::Mutex;`
- `42: outcomes: Mutex<VecDeque<ListenOutcome>>,`
- `56: outcomes: Mutex::new(outcomes.into_iter().collect()),`

### crates/vw-agent/src/channels/manager/message.rs

- `367: #[path = "message_tests.rs"]`
- `368: mod message_tests;`

### crates/vw-agent/src/channels/manager/message_execution.rs

- `218: #[path = "message_execution_tests.rs"]`
- `219: mod message_execution_tests;`

### crates/vw-agent/src/channels/manager/message_execution_tests.rs

- `11: use std::sync::{Arc, Mutex};`
- `48: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `49: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `50: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `60: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/manager/message_helpers.rs

- `358: /// 使用 Mutex 保护共享状态，锁 poisoned 时会恢复并返回 false。`
- `438: #[path = "message_helpers_tests.rs"]`
- `439: mod message_helpers_tests;`

### crates/vw-agent/src/channels/manager/message_helpers_tests.rs

- `23: stored_records: Mutex<Vec<StoredMemoryRecord>>,`
- `24: recall_queries: Mutex<Vec<String>>,`
- `104: sent: tokio::sync::Mutex<Vec<SendMessage>>,`
- `230: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `231: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `232: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `242: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/manager/message_response.rs

- `660: #[path = "message_response_tests.rs"]`
- `661: mod message_response_tests;`

### crates/vw-agent/src/channels/manager/message_response_tests.rs

- `15: sent: tokio::sync::Mutex<Vec<String>>,`
- `16: finalized: tokio::sync::Mutex<Vec<String>>,`
- `17: cancelled: tokio::sync::Mutex<Vec<String>>,`
- `18: reactions: tokio::sync::Mutex<Vec<String>>,`
- `22: routes: tokio::sync::Mutex<Vec<(String, Option<String>)>>,`
- `204: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `205: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `206: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `216: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/manager/message_result.rs

- `85: #[path = "message_result_tests.rs"]`
- `86: mod message_result_tests;`

### crates/vw-agent/src/channels/manager/message_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `50: sent: tokio::sync::Mutex<Vec<String>>,`
- `51: destinations: tokio::sync::Mutex<Vec<(String, Option<String>)>>,`
- `52: draft_placeholders: tokio::sync::Mutex<Vec<String>>,`
- `53: draft_updates: tokio::sync::Mutex<Vec<String>>,`
- `54: reactions: tokio::sync::Mutex<Vec<String>>,`
- `63: sent: tokio::sync::Mutex::new(Vec::new()),`
- `64: destinations: tokio::sync::Mutex::new(Vec::new()),`
- `65: draft_placeholders: tokio::sync::Mutex::new(Vec::new()),`
- `66: draft_updates: tokio::sync::Mutex::new(Vec::new()),`
- `67: reactions: tokio::sync::Mutex::new(Vec::new()),`
- `176: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `177: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `178: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `188: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`
- `366: #[path = "command_flow_tests.rs"]`
- `367: mod command_flow_tests;`

### crates/vw-agent/src/channels/manager/mod.rs

- `8: mod channels_config;`
- `9: mod command;`
- `10: mod dispatch;`
- `11: mod errors;`
- `12: mod execution_binding;`
- `13: mod execution_control;`
- `14: mod health;`
- `15: mod listener;`
- `16: mod message;`
- `17: mod message_execution;`
- `18: mod message_helpers;`
- `19: mod message_response;`
- `20: mod message_result;`
- `21: mod provider;`
- `22: mod reload;`
- `23: mod start;`
- `24: mod telegram;`
- `25: mod typing;`
- `44: mod tests;`

### crates/vw-agent/src/channels/manager/provider.rs

- `171: #[path = "provider_tests.rs"]`
- `172: mod provider_tests;`

### crates/vw-agent/src/channels/manager/provider_tests.rs

- `12: use std::sync::{Arc, Mutex};`
- `76: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `77: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `78: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `88: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/manager/reload.rs

- `10: project_state: Arc<Mutex<ChannelProjectState>>,`
- `17: ) -> Arc<Mutex<ChannelProjectState>> {`
- `95: #[path = "reload_tests.rs"]`
- `96: mod tests;`

### crates/vw-agent/src/channels/manager/start.rs

- `326: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `327: provider_cache: Arc::new(Mutex::new(provider_cache_seed)),`
- `328: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `338: non_cli_excluded_tools: Arc::new(Mutex::new(`
- `495: #[path = "start_tests.rs"]`
- `496: mod start_tests;`

### crates/vw-agent/src/channels/manager/telegram.rs

- `75: #[path = "telegram_tests.rs"]`
- `76: mod telegram_tests;`

### crates/vw-agent/src/channels/manager/typing.rs

- `86: #[path = "typing_tests.rs"]`
- `87: mod typing_tests;`

### crates/vw-agent/src/channels/markdown_chunk.rs

- `244: #[path = "markdown_chunk_tests.rs"]`
- `245: mod tests;`

### crates/vw-agent/src/channels/mattermost/mod.rs

- `40: use parking_lot::Mutex;`
- `79: typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,`
- `130: typing_handle: Mutex::new(None),`
- `823: #[path = "mod_tests.rs"]`
- `824: mod mod_tests;`
- `826: #[path = "tests.rs"]`
- `827: mod tests;`

### crates/vw-agent/src/channels/mattermost/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/channels/mod.rs

- `22: pub mod clawdtalk;`
- `24: pub mod cli;`
- `27: pub mod dingtalk;`
- `30: pub mod discord;`
- `33: pub mod email_channel;`
- `36: pub mod imessage;`
- `39: pub mod irc;`
- `42: pub mod lark;`
- `45: pub mod linq;`
- `46: pub(crate) mod markdown_chunk;`
- `49: pub mod mattermost;`
- `52: pub mod nextcloud_talk;`
- `55: pub mod nostr;`
- `56: pub(crate) mod outbound_delivery;`
- `59: pub mod qq;`
- `62: pub mod signal;`
- `65: pub mod slack;`
- `68: pub mod telegram;`
- `70: pub mod traits;`
- `73: pub mod wati;`
- `76: pub mod wechat;`
- `79: pub mod wecom;`
- `82: pub mod whatsapp;`
- `85: pub mod whatsapp_storage;`
- `88: pub mod whatsapp_web;`
- `142: pub(crate) mod config;`
- `144: pub(crate) mod format;`
- `146: pub(crate) mod history;`
- `147: mod http;`
- `149: pub(crate) mod manager;`
- `151: pub(crate) mod outbound_format;`
- `153: pub(crate) mod prompt;`
- `155: pub(crate) mod routing;`
- `157: pub(crate) mod runtime_command;`
- `158: pub(crate) mod scope;`
- `160: pub(crate) mod session;`
- `179: #[path = "mod_tests.rs"]`
- `180: mod mod_tests;`
- `182: #[path = "outbound_format_tests.rs"]`
- `183: mod outbound_format_tests;`
- `206: use std::sync::{Arc, Mutex, OnceLock};`
- `217: /// 内层 Mutex 保证并发安全访问，`
- `219: type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>;`
- `223: type ProviderCacheMap = Arc<Mutex<HashMap<String, Arc<dyn Provider>>>>;`
- `227: type RouteSelectionMap = Arc<Mutex<HashMap<String, ChannelRouteSelection>>>;`
- `474: pub(crate) project_state: Arc<Mutex<ChannelProjectState>>,`
- `527: pub(crate) non_cli_excluded_tools: Arc<Mutex<Vec<String>>>,`
- `693: mod tests;`
- `696: #[path = "manager/channels_config_tests.rs"]`
- `697: mod channel_project_binding_tests;`
- `706: #[path = "runtime_command/project_commands_tests.rs"]`
- `707: mod channel_project_select_tests;`
- `710: #[path = "runtime_command/command_tests.rs"]`
- `711: mod channel_command_catalog_tests;`
- `717: #[path = "runtime_command/history_commands_tests.rs"]`
- `718: mod channel_session_history_tests;`
- `721: #[path = "manager/dispatch_tests.rs"]`
- `722: mod dispatch_tests;`
- `725: #[path = "manager/dispatch_cancel_tests.rs"]`
- `726: mod channel_command_cancel_dispatch_tests;`
- `731: mod session_turn;`

### crates/vw-agent/src/channels/nextcloud_talk/mod.rs

- `694: #[path = "mod_tests.rs"]`
- `695: mod mod_tests;`
- `697: #[path = "tests.rs"]`
- `698: mod tests;`

### crates/vw-agent/src/channels/nextcloud_talk/tests.rs

- `14: mod tests {`

### crates/vw-agent/src/channels/nostr/mod.rs

- `50: use tokio::sync::RwLock;`
- `185: /// - sender_protocols：使用 Arc<RwLock> 包装，支持并发读取和独占写入`
- `208: /// 使用 Arc<RwLock> 实现线程安全的共享访问`
- `209: sender_protocols: Arc<RwLock<HashMap<PublicKey, NostrProtocol>>>,`
- `286: sender_protocols: Arc::new(RwLock::new(HashMap::new())),`
- `608: #[path = "tests.rs"]`
- `609: mod tests;`

### crates/vw-agent/src/channels/nostr/tests.rs

- `22: mod tests {`

### crates/vw-agent/src/channels/outbound_delivery.rs

- `36: #[path = "outbound_delivery_tests.rs"]`
- `37: mod tests;`

### crates/vw-agent/src/channels/prompt.rs

- `767: #[path = "prompt_tests.rs"]`
- `768: mod prompt_tests;`

### crates/vw-agent/src/channels/qq/auth.rs

- `105: #[path = "auth_tests.rs"]`
- `106: mod auth_tests;`

### crates/vw-agent/src/channels/qq/channel_impl.rs

- `39: #[path = "channel_impl_tests.rs"]`
- `40: mod channel_impl_tests;`

### crates/vw-agent/src/channels/qq/content.rs

- `4: use std::sync::OnceLock;`
- `181: static URL: OnceLock<Regex> = OnceLock::new();`
- `182: static BARE_DOMAIN: OnceLock<Regex> = OnceLock::new();`
- `213: #[path = "content_tests.rs"]`
- `214: mod content_tests;`

### crates/vw-agent/src/channels/qq/gateway.rs

- `187: #[path = "gateway_tests.rs"]`
- `188: mod gateway_tests;`

### crates/vw-agent/src/channels/qq/mod.rs

- `36: use tokio::sync::RwLock;`
- `38: mod auth;`
- `39: mod channel_impl;`
- `40: mod content;`
- `41: mod gateway;`
- `42: mod outbound;`
- `43: mod webhook;`
- `85: token_cache: Arc<RwLock<Option<(String, u64)>>>,`
- `87: dedup: Arc<RwLock<HashSet<String>>>,`
- `93: outbound_recorder: Option<Arc<tokio::sync::Mutex<Vec<super::traits::SendMessage>>>>,`
- `123: token_cache: Arc::new(RwLock::new(None)),`
- `124: dedup: Arc::new(RwLock::new(HashSet::new())),`
- `175: recorder: Arc<tokio::sync::Mutex<Vec<super::traits::SendMessage>>>,`
- `251: #[path = "tests.rs"]`
- `252: mod tests;`

### crates/vw-agent/src/channels/qq/outbound.rs

- `323: #[path = "outbound_tests.rs"]`
- `324: mod outbound_tests;`

### crates/vw-agent/src/channels/qq/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/channels/qq/webhook.rs

- `148: #[path = "webhook_tests.rs"]`
- `149: mod webhook_tests;`

### crates/vw-agent/src/channels/routing.rs

- `162: /// 此函数通过 Mutex 访问共享的路由覆盖映射表。如果 Mutex 被污染`
- `176: // 获取 Mutex 锁，如果被污染则恢复并继续`
- `269: /// 此函数通过 Mutex 访问共享的路由覆盖映射表。如果 Mutex 被污染，`
- `285: // 获取 Mutex 锁，处理可能的污染情况`
- `436: #[path = "routing_tests.rs"]`
- `437: mod routing_tests;`

### crates/vw-agent/src/channels/routing_tests.rs

- `15: use std::sync::{Arc, Mutex};`
- `59: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `60: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `61: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `71: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/runtime_command/approval_commands.rs

- `576: #[path = "approval_commands_tests.rs"]`
- `577: mod approval_commands_tests;`

### crates/vw-agent/src/channels/runtime_command/approval_commands_tests.rs

- `14: use std::sync::{Arc, Mutex};`
- `99: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `100: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `101: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `114: non_cli_excluded_tools: Arc::new(Mutex::new(`

### crates/vw-agent/src/channels/runtime_command/approval_config.rs

- `326: #[path = "approval_config_tests.rs"]`
- `327: mod approval_config_tests;`

### crates/vw-agent/src/channels/runtime_command/approval_config_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `71: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `72: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `73: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `86: non_cli_excluded_tools: Arc::new(Mutex::new(`

### crates/vw-agent/src/channels/runtime_command/command_tests.rs

- `301: struct CatalogChannel(tokio::sync::Mutex<Vec<SendMessage>>);`

### crates/vw-agent/src/channels/runtime_command/handler_tests.rs

- `17: use std::sync::{Arc, Mutex};`
- `72: sent: tokio::sync::Mutex<Vec<SendMessage>>,`
- `78: Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: false }`
- `82: Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: true }`
- `153: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `154: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `155: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `165: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/runtime_command/history_commands.rs

- `13: fn confirmations() -> &'static Mutex<HashMap<ChannelMessageScope, Confirmation>> {`
- `14: static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, Confirmation>>> = OnceLock::new();`
- `15: STORE.get_or_init(|| Mutex::new(HashMap::new()))`

### crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs

- `186: ) -> (ModelServer, Arc<Mutex<Vec<serde_json::Value>>>) {`
- `187: let requests = Arc::new(Mutex::new(Vec::new()));`
- `299: foreign.project_state = Arc::new(Mutex::new(Default::default()));`

### crates/vw-agent/src/channels/runtime_command/project_commands.rs

- `75: candidate.project_state = Arc::new(Mutex::new(state));`

### crates/vw-agent/src/channels/runtime_command/project_commands_tests.rs

- `139: struct ProjectReplyChannel(Mutex<Vec<SendMessage>>);`

### crates/vw-agent/src/channels/runtime_command/session_commands_tests.rs

- `16: use std::sync::{Arc, Mutex};`
- `64: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `65: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `66: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `76: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`
- `606: struct LifecycleChannel(tokio::sync::Mutex<Vec<SendMessage>>);`
- `692: foreign.project_state = Arc::new(Mutex::new(Default::default()));`

### crates/vw-agent/src/channels/runtime_command/session_lifecycle.rs

- `39: fn archive_confirmations() -> &'static Mutex<HashMap<ChannelMessageScope, ArchiveConfirmation>> {`
- `40: static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, ArchiveConfirmation>>> =`
- `41: OnceLock::new();`
- `42: STORE.get_or_init(|| Mutex::new(HashMap::new()))`

### crates/vw-agent/src/channels/runtime_command/task_mode_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `32: sent: tokio::sync::Mutex<Vec<SendMessage>>,`
- `38: Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: true }`
- `44: Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: false }`
- `97: conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),`
- `98: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `99: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `109: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/runtime_command.rs

- `6: mod approval_commands;`
- `7: mod approval_config;`
- `8: pub(crate) mod catalog;`
- `9: mod command;`
- `10: mod handler;`
- `11: mod history_commands;`
- `12: pub(crate) mod project_commands;`
- `13: mod session_commands;`
- `14: mod session_lifecycle;`
- `15: mod task_mode;`
- `18: mod handler_tests;`
- `20: pub(crate) mod session_commands_tests;`
- `22: mod task_mode_tests;`
- `39: #[path = "runtime_command_tests.rs"]`
- `40: mod runtime_command_tests;`

### crates/vw-agent/src/channels/scope.rs

- `70: gates: HashMap<ChannelChatScope, std::sync::Weak<tokio::sync::RwLock<()>>>,`
- `187: ) -> Arc<tokio::sync::RwLock<()>> {`
- `194: let gate = Arc::new(tokio::sync::RwLock::new(()));`

### crates/vw-agent/src/channels/scope_delivery_tests.rs

- `2: struct ScopeRecordingChannel(tokio::sync::Mutex<Vec<SendMessage>>);`

### crates/vw-agent/src/channels/session.rs

- `4: pub(crate) fn sender_session_store() -> &'static Mutex<HashMap<ChannelMessageScope, String>> {`
- `5: static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, String>>> = OnceLock::new();`
- `6: STORE.get_or_init(|| Mutex::new(HashMap::new()))`
- `416: #[path = "session_tests.rs"]`
- `417: pub(crate) mod session_tests;`

### crates/vw-agent/src/channels/session_tests.rs

- `9: use std::sync::{Arc, Mutex};`
- `58: conversation_histories: Arc::new(Mutex::new(HashMap::new())),`
- `59: provider_cache: Arc::new(Mutex::new(HashMap::new())),`
- `60: route_overrides: Arc::new(Mutex::new(HashMap::new())),`
- `70: non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),`

### crates/vw-agent/src/channels/signal/mod.rs

- `754: #[path = "tests.rs"]`
- `755: mod tests;`

### crates/vw-agent/src/channels/signal/tests.rs

- `19: mod tests {`
- `29: use tokio::sync::{Mutex, oneshot};`
- `55: requests: Mutex<Vec<RecordedRequest>>,`
- `56: responses: Mutex<VecDeque<ResponseSpec>>,`
- `68: requests: Mutex::new(Vec::new()),`
- `69: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/slack/mod.rs

- `459: #[path = "tests.rs"]`
- `460: mod tests;`

### crates/vw-agent/src/channels/slack/tests.rs

- `6: mod tests {`

### crates/vw-agent/src/channels/telegram/attachments_incoming_tests.rs

- `14: use tokio::sync::{Mutex, oneshot};`
- `51: requests: Mutex<Vec<RecordedRequest>>,`
- `52: responses: Mutex<VecDeque<ResponseSpec>>,`
- `64: requests: Mutex::new(Vec::new()),`
- `65: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/attachments_outbound_tests.rs

- `16: use tokio::sync::{Mutex, oneshot};`
- `26: paths: Mutex<Vec<String>>,`
- `27: responses: Mutex<VecDeque<ResponseSpec>>,`
- `39: paths: Mutex::new(Vec::new()),`
- `40: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/callbacks_tests.rs

- `12: use tokio::sync::{Mutex, oneshot};`
- `26: requests: Mutex<Vec<RecordedRequest>>,`
- `37: let state = Arc::new(TestServerState { requests: Mutex::new(Vec::new()) });`

### crates/vw-agent/src/channels/telegram/channel_impl_tests.rs

- `14: use tokio::sync::{Mutex, oneshot};`
- `31: requests: Mutex<Vec<RecordedRequest>>,`
- `43: let state = Arc::new(TestServerState { requests: Mutex::new(Vec::new()), status });`

### crates/vw-agent/src/channels/telegram/config.rs

- `21: use parking_lot::Mutex;`
- `22: use std::sync::{Arc, RwLock};`
- `65: allowed_users: Arc::new(RwLock::new(normalized_allowed)),`
- `70: last_draft_edit: Mutex::new(std::collections::HashMap::new()),`
- `71: typing_handle: Mutex::new(None),`
- `74: bot_username: Mutex::new(None),`
- `462: /// 使用 Mutex 确保缓存访问的线程安全性`

### crates/vw-agent/src/channels/telegram/config_tests.rs

- `17: use tokio::sync::{Mutex, oneshot};`
- `80: requests: Mutex<usize>,`
- `81: responses: Mutex<VecDeque<(StatusCode, &'static str)>>,`
- `93: requests: Mutex::new(0),`
- `94: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/core.rs

- `39: static BOT_TOKEN_RE: std::sync::LazyLock<regex::Regex> =`
- `40: std::sync::LazyLock::new(|| regex::Regex::new(r"bot[^/\s]+").unwrap());`

### crates/vw-agent/src/channels/telegram/draft_tests.rs

- `15: use tokio::sync::{Mutex, oneshot};`
- `51: requests: Mutex<Vec<RecordedRequest>>,`
- `52: responses: Mutex<VecDeque<ResponseSpec>>,`
- `64: requests: Mutex::new(Vec::new()),`
- `65: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/file_io_tests.rs

- `13: use tokio::sync::{Mutex, oneshot};`
- `23: paths: Mutex<Vec<String>>,`
- `24: responses: Mutex<VecDeque<ResponseSpec>>,`
- `36: paths: Mutex::new(Vec::new()),`
- `37: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/mod.rs

- `54: use parking_lot::Mutex;`
- `55: use std::sync::{Arc, RwLock};`
- `57: mod attachments;`
- `58: mod attachments_incoming;`
- `59: mod attachments_outbound;`
- `60: mod callbacks;`
- `61: mod channel_impl;`
- `62: mod config;`
- `63: mod core;`
- `64: mod draft;`
- `65: mod file_io;`
- `66: mod formatting;`
- `67: mod incoming;`
- `68: mod mentions;`
- `69: mod message_utils;`
- `70: mod outbound;`
- `71: mod pairing;`
- `72: mod polling;`
- `73: mod reactions;`
- `74: mod sending_media;`
- `75: mod sending_text;`
- `76: mod tool_tags;`
- `92: /// 使用 Mutex 和 RwLock 保证多线程环境下的安全访问：`
- `93: /// - allowed_users: 使用 RwLock 允许多个读取者同时访问`
- `94: /// - typing_handle: 使用 Mutex 保护异步任务句柄`
- `95: /// - last_draft_edit: 使用 Mutex 保护草稿编辑时间戳`
- `96: /// - bot_username: 使用 Mutex 缓存机器人用户名`
- `104: ///     allowed_users: Arc::new(RwLock::new(vec!["user1".to_string()])),`
- `117: /// 使用 RwLock 包装以支持动态更新和并发读取。`
- `120: allowed_users: Arc<RwLock<Vec<String>>>,`
- `137: /// 使用 Mutex 保证线程安全的访问和更新。`
- `138: typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,`
- `156: last_draft_edit: Mutex<std::collections::HashMap<String, std::time::Instant>>,`
- `173: /// 使用 Mutex 实现懒加载和线程安全。`
- `174: bot_username: Mutex<Option<String>>,`
- `190: #[path = "attachments_incoming_tests.rs"]`
- `191: mod attachments_incoming_tests;`
- `193: #[path = "attachments_outbound_tests.rs"]`
- `194: mod attachments_outbound_tests;`
- `196: #[path = "attachments_tests.rs"]`
- `197: mod attachments_tests;`
- `199: #[path = "callbacks_tests.rs"]`
- `200: mod callbacks_tests;`
- `202: #[path = "channel_impl_tests.rs"]`
- `203: mod channel_impl_tests;`
- `205: #[path = "config_tests.rs"]`
- `206: mod config_tests;`
- `208: #[path = "core_tests.rs"]`
- `209: mod core_tests;`
- `211: #[path = "draft_tests.rs"]`
- `212: mod draft_tests;`
- `214: #[path = "file_io_tests.rs"]`
- `215: mod file_io_tests;`
- `217: #[path = "formatting_tests.rs"]`
- `218: mod formatting_tests;`
- `220: #[path = "incoming_tests.rs"]`
- `221: mod incoming_tests;`
- `223: #[path = "mentions_tests.rs"]`
- `224: mod mentions_tests;`
- `226: #[path = "message_utils_tests.rs"]`
- `227: mod message_utils_tests;`
- `229: #[path = "outbound_tests.rs"]`
- `230: mod outbound_tests;`
- `232: #[path = "pairing_tests.rs"]`
- `233: mod pairing_tests;`
- `235: #[path = "polling_tests.rs"]`
- `236: mod polling_tests;`
- `238: #[path = "reactions_tests.rs"]`
- `239: mod reactions_tests;`
- `241: #[path = "sending_media_tests.rs"]`
- `242: mod sending_media_tests;`
- `244: #[path = "sending_text_tests.rs"]`
- `245: mod sending_text_tests;`
- `247: mod tests;`
- `249: #[path = "tool_tags_tests.rs"]`
- `250: mod tool_tags_tests;`

### crates/vw-agent/src/channels/telegram/outbound_tests.rs

- `14: sync::{Arc, Mutex},`
- `19: requests: Arc<Mutex<Vec<CapturedRequest>>>,`
- `32: requests: Arc<Mutex<Vec<CapturedRequest>>>,`
- `38: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/channels/telegram/polling_tests.rs

- `16: sync::{Mutex, Notify},`
- `46: inner: Arc<Mutex<MockTelegramData>>,`

### crates/vw-agent/src/channels/telegram/reactions_tests.rs

- `7: use std::sync::{Arc, Mutex};`
- `14: requests: Arc<Mutex<Vec<Value>>>,`
- `30: ) -> (String, Arc<Mutex<Vec<Value>>>, oneshot::Sender<()>, JoinHandle<()>) {`
- `31: let requests = Arc::new(Mutex::new(Vec::new()));`
- `56: async fn wait_for_request(requests: &Arc<Mutex<Vec<Value>>>) -> Value {`

### crates/vw-agent/src/channels/telegram/sending_media_tests.rs

- `14: use tokio::sync::{Mutex, oneshot};`
- `53: requests: Mutex<Vec<RecordedRequest>>,`
- `54: responses: Mutex<VecDeque<ResponseSpec>>,`
- `66: requests: Mutex::new(Vec::new()),`
- `67: responses: Mutex::new(VecDeque::from(responses)),`

### crates/vw-agent/src/channels/telegram/tests/mod.rs

- `37: mod ack_reaction;`
- `38: mod approval_callback;`
- `39: mod attachment_format;`
- `40: mod attachments;`
- `41: mod channel_basic;`
- `42: mod file_sending;`
- `43: mod helpers;`
- `44: mod markdown_html;`
- `45: mod mention_handling;`
- `46: mod message_parsing;`
- `47: mod message_splitting;`
- `48: mod pairing_bind;`
- `49: mod streaming_drafts;`
- `50: mod tool_call_tags;`
- `51: mod user_allowlist;`

### crates/vw-agent/src/channels/tests/message_dispatch.rs

- `201: foreign.project_state = Arc::new(Mutex::new(Default::default()));`
- `208: struct FlowSummaryServer(tokio::task::JoinHandle<()>, Arc<Mutex<Vec<serde_json::Value>>>, String);`
- `213: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/channels/tests/mod.rs

- `77: mod approval_commands; // 审批命令处理测试`
- `78: mod command_parsing; // 命令解析逻辑测试`
- `79: mod config_loading; // 配置加载验证测试`
- `80: mod health_supervisor; // 健康监督器功能测试`
- `81: mod history_management; // 历史记录管理测试`
- `82: mod identity; // 身份验证和管理测试`
- `84: mod routing; // 路由算法和策略测试`
- `85: mod session_history; // 会话历史角色映射测试`
- `86: mod system_prompt; // 系统提示词构建测试`
- `87: mod timeout_budget; // 超时预算分配测试`
- `88: mod tool_execution; // 工具执行流程测试`
- `89: mod util; // 测试辅助工具函数`
- `90: mod vision; // 视觉功能集成测试`

### crates/vw-agent/src/channels/traits/mod.rs

- `726: #[path = "mod_tests.rs"]`
- `727: mod tests;`

### crates/vw-agent/src/channels/traits/mod_tests.rs

- `2: use std::sync::{Arc, Mutex};`
- `35: sent: Arc<Mutex<Vec<SendMessage>>>,`

### crates/vw-agent/src/channels/wati/mod.rs

- `245: #[path = "tests.rs"]`
- `246: mod tests;`

### crates/vw-agent/src/channels/wati/tests.rs

- `6: mod tests {`
- `15: use tokio::sync::{Mutex, oneshot};`
- `34: requests: Arc<Mutex<Vec<RecordedRequest>>>,`
- `35: responses: Arc<Mutex<VecDeque<ResponseSpec>>>,`
- `40: requests: Arc<Mutex<Vec<RecordedRequest>>>,`
- `46: let requests = Arc::new(Mutex::new(Vec::new()));`
- `47: let responses = Arc::new(Mutex::new(VecDeque::from(responses)));`

### crates/vw-agent/src/channels/wechat/mod.rs

- `12: use tokio::sync::{RwLock, mpsc};`
- `16: pub(crate) mod protocol;`
- `25: context_tokens: Arc<RwLock<HashMap<String, String>>>,`
- `51: context_tokens: Arc::new(RwLock::new(HashMap::new())),`
- `284: #[path = "tests.rs"]`
- `285: mod tests;`

### crates/vw-agent/src/channels/wechat/tests.rs

- `146: ) -> (WeChatChannel, std::sync::Arc<std::sync::Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {`
- `154: let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/channels/wecom/mod.rs

- `3: mod outbound;`
- `4: pub mod runtime;`
- `5: pub mod webhook;`
- `16: use tokio::sync::{RwLock, mpsc};`
- `30: outbound: Arc<RwLock<Option<mpsc::Sender<OutboundItem>>>>,`
- `31: reply_requests: Arc<RwLock<HashMap<String, String>>>,`
- `41: outbound: Arc::new(RwLock::new(None)),`
- `42: reply_requests: Arc::new(RwLock::new(HashMap::new())),`
- `276: #[path = "tests.rs"]`
- `277: mod tests;`
- `280: #[path = "outbound_tests.rs"]`
- `281: mod outbound_tests;`
- `284: #[path = "runtime_tests.rs"]`
- `285: mod runtime_tests;`

### crates/vw-agent/src/channels/wecom/runtime.rs

- `8: sync::{Mutex, OnceLock},`
- `58: pub(super) fn registry() -> &'static Mutex<HashMap<WeComRuntimeScope, Entry>> {`
- `59: static REGISTRY: OnceLock<Mutex<HashMap<WeComRuntimeScope, Entry>>> = OnceLock::new();`
- `60: REGISTRY.get_or_init(Mutex::default)`

### crates/vw-agent/src/channels/wecom/webhook.rs

- `159: #[path = "webhook_tests.rs"]`
- `160: mod tests;`

### crates/vw-agent/src/channels/wecom/webhook_tests.rs

- `79: let responses = Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::from(responses)));`

### crates/vw-agent/src/channels/whatsapp/mod.rs

- `271: #[path = "mod_tests.rs"]`
- `272: mod mod_tests;`

### crates/vw-agent/src/channels/whatsapp/mod_tests.rs

- `5: mod tests {`

### crates/vw-agent/src/channels/whatsapp_storage/mod.rs

- `28: mod app_sync_store;`
- `30: mod device_store;`
- `32: mod protocol_store;`
- `34: mod schema;`
- `36: mod signal_store;`
- `39: #[path = "app_sync_store_tests.rs"]`
- `40: mod app_sync_store_tests;`
- `42: #[path = "device_store_tests.rs"]`
- `43: mod device_store_tests;`
- `45: #[path = "mod_tests.rs"]`
- `46: mod mod_tests;`
- `48: #[path = "protocol_store_tests.rs"]`
- `49: mod protocol_store_tests;`
- `51: #[path = "schema_tests.rs"]`
- `52: mod schema_tests;`
- `54: #[path = "signal_store_tests.rs"]`
- `55: mod signal_store_tests;`
- `58: use parking_lot::Mutex;`
- `76: /// SQLite connection (thread-safe via Mutex)`
- `77: conn: Arc<Mutex<Connection>>,`
- `105: let store = Self { db_path, conn: Arc::new(Mutex::new(conn)), device_id: 1 };`
- `114: #[path = "tests.rs"]`
- `115: mod tests;`

### crates/vw-agent/src/channels/whatsapp_storage/tests.rs

- `10: mod tests {`

### crates/vw-agent/src/channels/whatsapp_web/mod.rs

- `37: use parking_lot::Mutex;`
- `178: bot_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,`
- `180: client: Arc<Mutex<Option<Arc<wa_rs::Client>>>>,`
- `182: tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<ChannelMessage>>>>,`
- `206: bot_handle: Arc::new(Mutex::new(None)),`
- `207: client: Arc::new(Mutex::new(None)),`
- `208: tx: Arc::new(Mutex::new(None)),`
- `759: #[path = "mod_tests.rs"]`
- `760: mod mod_tests;`

### crates/vw-agent/src/command/command.rs

- `55: use std::sync::LazyLock;`
- `60: pub mod event {`
- `247: pub mod r#default {`
- `276: static PROMPT_INITIALIZE: &str = include_str!("template/initialize.txt");`
- `290: static PROMPT_REVIEW: &str = include_str!("template/review.txt");`
- `333: static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\d+").unwrap());`

### crates/vw-agent/src/command/command_tests.rs

- `7: use std::sync::{Arc, Mutex};`
- `226: let seen = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/command/mod.rs

- `51: pub mod command;`
- `53: #[path = "command_tests.rs"]`
- `54: mod command_tests;`

### crates/vw-agent/src/computer_control/chrome/bridge.rs

- `6: use parking_lot::Mutex;`
- `52: binding: Mutex<DocumentBinding>,`
- `186: binding: Mutex::new(DocumentBinding::default()),`

### crates/vw-agent/src/computer_control/chrome/bridge_tests.rs

- `90: use parking_lot::RwLock;`
- `105: let policy = Arc::new(RwLock::new(policy));`

### crates/vw-agent/src/computer_control/chrome/mod.rs

- `2: pub mod bridge;`
- `4: mod bridge_tests;`
- `5: pub mod origin_policy;`
- `7: mod origin_policy_tests;`

### crates/vw-agent/src/computer_control/chrome/origin_policy_tests.rs

- `119: std::sync::Arc<parking_lot::RwLock<ComputerControlPolicy>>,`
- `133: let policy = Arc::new(parking_lot::RwLock::new(p));`

### crates/vw-agent/src/computer_control/host.rs

- `9: use parking_lot::{Mutex, RwLock};`
- `69: policy: Arc<RwLock<ComputerControlPolicy>>,`
- `72: sessions: Mutex<HashMap<ControlSessionId, Session>>,`
- `76: pub fn new(policy: Arc<RwLock<ComputerControlPolicy>>, locked: Arc<AtomicBool>) -> Self {`
- `84: sessions: Mutex::new(HashMap::new()),`
- `575: #[path = "lifecycle_tests.rs"]`
- `576: mod lifecycle_tests;`

### crates/vw-agent/src/computer_control/host_tests.rs

- `3: use parking_lot::RwLock;`
- `26: Arc::new(ControlHost::new(Arc::new(RwLock::new(policy)), Arc::new(AtomicBool::new(false))))`
- `264: let policy = Arc::new(RwLock::new(policy));`

### crates/vw-agent/src/computer_control/lifecycle_tests.rs

- `9: let host = ControlHost::new(Arc::new(RwLock::new(policy)), Arc::new(AtomicBool::new(false)));`

### crates/vw-agent/src/computer_control/mod.rs

- `3: pub mod policy;`
- `6: pub mod host;`
- `9: mod policy_tests;`
- `12: mod host_tests;`
- `14: pub mod native;`
- `17: pub mod chrome;`
- `19: pub mod office;`

### crates/vw-agent/src/computer_control/native/input_tests.rs

- `16: revoke_after: Option<Arc<RwLock<ComputerControlPolicy>>>,`
- `19: struct Os(Arc<Mutex<State>>);`
- `103: let os = Os(Arc::new(Mutex::new(State { focus: 100, ..Default::default() })));`
- `113: Arc::new(RwLock::new(p)),`

### crates/vw-agent/src/computer_control/native/locked_session.rs

- `132: #[path = "locked_session_tests.rs"]`
- `133: mod tests;`

### crates/vw-agent/src/computer_control/native/macos_input.rs

- `226: mod os {`

### crates/vw-agent/src/computer_control/native/mod.rs

- `6: use parking_lot::{Mutex, RwLock};`
- `16: mod input_tests;`
- `18: mod locked_ax;`
- `19: mod macos_input;`
- `20: pub mod locked_session;`
- `22: mod macos_observation;`
- `24: mod observation_tests;`
- `221: policy: &RwLock<ComputerControlPolicy>,`
- `226: policy: &RwLock<ComputerControlPolicy>,`
- `247: policy: Arc<RwLock<ComputerControlPolicy>>,`
- `254: media: Mutex<Option<StoredScreenshot>>,`
- `255: observe_gate: tokio::sync::Mutex<()>,`
- `259: policy: Arc<RwLock<ComputerControlPolicy>>,`
- `269: policy: Arc<RwLock<ComputerControlPolicy>>,`
- `296: media: Mutex::new(None),`
- `297: observe_gate: tokio::sync::Mutex::new(()),`

### crates/vw-agent/src/computer_control/native/observation_tests.rs

- `87: fn policy() -> Arc<RwLock<ComputerControlPolicy>> {`
- `96: Arc::new(RwLock::new(policy))`
- `253: let p = Arc::new(RwLock::new(ComputerControlPolicy::new(ComputerControlConfig::default())));`

### crates/vw-agent/src/computer_control/office/mod.rs

- `5: pub mod transport;`

### crates/vw-agent/src/computer_control/office/transport.rs

- `18: use parking_lot::Mutex;`
- `73: state: Arc<Mutex<State>>,`
- `112: let state = Arc::new(Mutex::new(State {`
- `339: state: Arc<Mutex<State>>,`
- `373: state: Arc<Mutex<State>>,`
- `390: state: Arc<Mutex<State>>,`
- `406: state: Arc<Mutex<State>>,`
- `542: #[path = "transport_tests.rs"]`
- `543: mod tests;`

### crates/vw-agent/src/computer_control/office/transport_tests.rs

- `8: static CERTS: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();`

### crates/vw-agent/src/config/mod.rs

- `40: use std::sync::LazyLock;`
- `44: pub mod migration;`
- `46: #[path = "migration_tests.rs"]`
- `47: mod migration_tests;`
- `49: #[path = "mod_tests.rs"]`
- `50: mod mod_tests;`
- `51: pub mod schema;`
- `53: pub mod store;`
- `55: #[path = "store_tests.rs"]`
- `56: mod store_tests;`
- `58: #[path = "tests.rs"]`
- `59: mod tests;`
- `60: pub mod traits;`
- `62: #[path = "traits_tests.rs"]`
- `63: mod traits_tests;`
- `134: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `824: mod runtime_home;`
- `826: mod runtime_home_tests;`

### crates/vw-agent/src/config/schema/channel_reload.rs

- `122: #[path = "channel_reload_tests.rs"]`
- `123: mod tests;`

### crates/vw-agent/src/config/schema/channels/mod.rs

- `37: pub mod chat;`
- `38: pub mod email;`
- `39: pub mod helpers;`
- `40: pub mod lark;`
- `41: pub mod other;`
- `42: pub mod secrets;`
- `43: mod secrets_isolation;`
- `44: pub mod telegram_env;`
- `45: pub mod types;`
- `48: #[path = "chat_tests.rs"]`
- `49: mod chat_tests;`
- `51: #[path = "email_tests.rs"]`
- `52: mod email_tests;`
- `54: #[path = "helpers_tests.rs"]`
- `55: mod helpers_tests;`
- `57: #[path = "lark_tests.rs"]`
- `58: mod lark_tests;`
- `60: #[path = "other_tests.rs"]`
- `61: mod other_tests;`
- `63: #[path = "secrets_tests.rs"]`
- `64: mod secrets_tests;`
- `66: #[path = "telegram_env_tests.rs"]`
- `67: mod telegram_env_tests;`
- `69: #[path = "types_tests.rs"]`
- `70: mod types_tests;`

### crates/vw-agent/src/config/schema/channels/telegram_env_tests.rs

- `5: use std::sync::{Mutex, MutexGuard};`
- `7: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/config/schema/config_helpers_tests.rs

- `7: use std::sync::{Mutex, OnceLock};`
- `10: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `11: LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()`

### crates/vw-agent/src/config/schema/config_tests.rs

- `6: use std::sync::{Mutex, OnceLock};`
- `9: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `10: LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()`

### crates/vw-agent/src/config/schema/config_validate_tests.rs

- `3: use std::sync::{Mutex, MutexGuard};`
- `11: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/config/schema/mod.rs

- `2: mod channel_reload;`
- `6: pub mod automation;`
- `7: pub mod channels;`
- `8: pub mod provider;`
- `9: pub mod proxy;`
- `10: pub mod security;`
- `11: pub mod skills;`
- `15: mod config;`
- `16: mod config_env;`
- `17: mod config_helpers;`
- `18: mod config_io;`
- `19: pub(crate) mod config_load;`
- `20: mod config_save;`
- `21: mod config_secrets;`
- `22: mod config_validate;`
- `23: pub(crate) mod workspace;`
- `26: #[path = "config_env_tests.rs"]`
- `27: mod config_env_tests;`
- `29: #[path = "config_helpers_tests.rs"]`
- `30: mod config_helpers_tests;`
- `32: #[path = "config_io_tests.rs"]`
- `33: mod config_io_tests;`
- `35: #[path = "config_load_tests.rs"]`
- `36: mod config_load_tests;`
- `38: #[path = "config_save_tests.rs"]`
- `39: mod config_save_tests;`
- `41: #[path = "config_secrets_tests.rs"]`
- `42: mod config_secrets_tests;`
- `44: #[path = "config_tests.rs"]`
- `45: mod config_tests;`
- `47: #[path = "config_validate_tests.rs"]`
- `48: mod config_validate_tests;`
- `50: #[path = "workspace_tests.rs"]`
- `51: mod workspace_tests;`

### crates/vw-agent/src/config/schema/proxy.rs

- `9: use std::sync::{OnceLock, RwLock};`
- `13: static RUNTIME_PROXY_CONFIG: OnceLock<RwLock<ProxyConfig>> = OnceLock::new();`
- `14: static RUNTIME_PROXY_CLIENT_CACHE: OnceLock<RwLock<HashMap<String, reqwest::Client>>> =`
- `15: OnceLock::new();`
- `215: fn runtime_proxy_state() -> &'static RwLock<ProxyConfig> {`
- `216: RUNTIME_PROXY_CONFIG.get_or_init(|| RwLock::new(ProxyConfig::default()))`
- `219: fn runtime_proxy_client_cache() -> &'static RwLock<HashMap<String, reqwest::Client>> {`
- `220: RUNTIME_PROXY_CLIENT_CACHE.get_or_init(|| RwLock::new(HashMap::new()))`
- `377: #[path = "proxy_tests.rs"]`
- `378: mod proxy_tests;`

### crates/vw-agent/src/config/store/postgres.rs

- `184: #[path = "postgres_tests.rs"]`
- `185: mod postgres_tests;`

### crates/vw-agent/src/config/store.rs

- `15: mod postgres;`

### crates/vw-agent/src/cron/active_runs.rs

- `9: use std::sync::{Mutex, MutexGuard, OnceLock};`
- `21: static ACTIVE_RUNS: OnceLock<Mutex<BTreeMap<String, ActiveRun>>> = OnceLock::new();`
- `25: .get_or_init(|| Mutex::new(BTreeMap::new()))`

### crates/vw-agent/src/cron/consolidation/mod.rs

- `85: #[path = "tests.rs"]`
- `86: mod tests;`

### crates/vw-agent/src/cron/consolidation/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/cron/mod.rs

- `9: pub mod active_runs;`
- `10: pub mod consolidation;`
- `11: pub(crate) mod delivery;`
- `14: mod delivery_tests;`
- `17: pub(crate) mod active_runs_tests;`
- `18: mod schedule;`
- `19: mod store;`
- `20: mod types;`
- `23: #[path = "mod_tests.rs"]`
- `24: mod mod_tests;`
- `26: #[path = "store_tests.rs"]`
- `27: mod store_tests;`
- `29: pub mod scheduler;`
- `276: #[path = "tests.rs"]`
- `277: mod tests;`
- `280: #[path = "tests/schedule.rs"]`
- `281: mod schedule_acceptance_tests;`
- `284: mod wecom_acceptance_tests;`

### crates/vw-agent/src/cron/schedule/mod.rs

- `18: mod biweekly;`
- `261: #[path = "tests.rs"]`
- `262: mod tests;`
- `265: mod biweekly_tests;`

### crates/vw-agent/src/cron/schedule/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/cron/scheduler.rs

- `60: use std::sync::{Arc, OnceLock};`
- `81: static SCHEDULER_REFRESH: OnceLock<tokio::sync::Notify> = OnceLock::new();`
- `1354: #[path = "tests/scheduler.rs"]`
- `1355: mod tests;`
- `1358: #[path = "scheduler_tests.rs"]`
- `1359: mod scheduler_tests;`

### crates/vw-agent/src/cron/store_tests.rs

- `12: #[path = "tests/store.rs"]`
- `13: mod legacy_store_tests;`

### crates/vw-agent/src/cron/tests/mod.rs

- `6: mod commands;`
- `7: mod consolidation;`
- `8: mod schedule;`
- `9: mod scheduler;`
- `10: mod store;`
- `11: mod types;`

### crates/vw-agent/src/cron/tests/scheduler.rs

- `28: use std::sync::OnceLock;`
- `49: static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();`
- `50: LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await`

### crates/vw-agent/src/cron/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/cron/types/mod.rs

- `513: #[path = "tests.rs"]`
- `514: mod tests;`

### crates/vw-agent/src/cron/types/tests.rs

- `9: mod tests {`

### crates/vw-agent/src/cron/wecom_acceptance_tests.rs

- `33: fn state_with_config(config: Arc<parking_lot::Mutex<Config>>, require_pairing: bool) -> AppState {`
- `93: let shared = Arc::new(parking_lot::Mutex::new(config.clone()));`

### crates/vw-agent/src/daemon/mod.rs

- `796: #[path = "tests.rs"]`
- `797: mod tests;`

### crates/vw-agent/src/daemon/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/device_sync/change_events.rs

- `5: sync::{Mutex, OnceLock},`
- `46: fn channels() -> &'static Mutex<BTreeMap<TenantKey, watch::Sender<u64>>> {`
- `47: static CHANNELS: OnceLock<Mutex<BTreeMap<TenantKey, watch::Sender<u64>>>> = OnceLock::new();`

### crates/vw-agent/src/device_sync/lifecycle.rs

- `4: use tokio::sync::Mutex;`
- `22: pub struct SyncLifecycle { state: Arc<Mutex<State>> }`

### crates/vw-agent/src/device_sync/mod.rs

- `3: pub mod operation_lock;`
- `5: pub mod webdav_settings;`
- `7: pub mod s3_settings;`
- `10: pub mod export;`
- `13: pub mod import;`
- `16: pub mod apply;`
- `19: pub mod webdav_upload;`
- `22: pub mod projection;`
- `25: pub mod s3_upload;`
- `28: pub mod s3_download;`
- `31: pub mod webdav_download;`
- `33: pub mod change_events;`
- `35: pub mod tasks;`
- `37: pub mod cancel;`
- `39: pub mod status;`
- `41: pub mod auto_sync;`
- `43: pub mod lifecycle;`
- `46: pub(crate) mod skill_receive;`

### crates/vw-agent/src/device_sync/operation_lock.rs

- `5: use std::sync::{Arc, Mutex, OnceLock, Weak};`
- `8: use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};`
- `15: static LOCKS: OnceLock<Mutex<HashMap<Scope, Weak<AsyncMutex<()>>>>> = OnceLock::new();`
- `42: .get_or_init(|| Mutex::new(HashMap::new()))`

### crates/vw-agent/src/device_sync/status.rs

- `3: use std::sync::{Mutex, OnceLock};`
- `46: fn bindings() -> &'static Mutex<Vec<Binding>> {`
- `47: static BINDINGS: OnceLock<Mutex<Vec<Binding>>> = OnceLock::new();`

### crates/vw-agent/src/device_sync/tasks.rs

- `2: use std::{future::Future, sync::{Mutex, OnceLock}};`
- `31: fn records() -> &'static Mutex<Vec<Record>> {`
- `32: static RECORDS: OnceLock<Mutex<Vec<Record>>> = OnceLock::new();`

### crates/vw-agent/src/doctor/environment_checks_tests.rs

- `3: use std::sync::{Mutex, MutexGuard};`
- `8: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/doctor/mod.rs

- `18: mod config_checks;`
- `19: mod daemon_checks;`
- `20: mod environment_checks;`
- `21: mod model_probe;`
- `22: mod traces;`
- `23: mod utils;`
- `24: mod workspace_checks;`
- `27: #[path = "config_checks_tests.rs"]`
- `28: mod config_checks_tests;`
- `30: #[path = "daemon_checks_tests.rs"]`
- `31: mod daemon_checks_tests;`
- `33: #[path = "environment_checks_tests.rs"]`
- `34: mod environment_checks_tests;`
- `36: #[path = "mod_tests.rs"]`
- `37: mod mod_tests;`
- `39: #[path = "model_probe_tests.rs"]`
- `40: mod model_probe_tests;`
- `42: #[path = "traces_tests.rs"]`
- `43: mod traces_tests;`
- `45: #[path = "utils_tests.rs"]`
- `46: mod utils_tests;`
- `48: #[path = "workspace_checks_tests.rs"]`
- `49: mod workspace_checks_tests;`
- `185: #[path = "tests.rs"]`
- `186: mod tests;`

### crates/vw-agent/src/doctor/tests.rs

- `20: mod tests {`

### crates/vw-agent/src/env/mod.rs

- `9: use std::sync::LazyLock;`
- `10: use std::sync::Mutex;`
- `26: static ENV_CONTEXT: LazyLock<context::Context<Mutex<Vars>>> =`
- `27: LazyLock::new(|| context::create("env"));`
- `28: static DEFAULT: LazyLock<Mutex<Vars>> = LazyLock::new(|| Mutex::new(snapshot()));`
- `37: fn with_env<R>(f: impl FnOnce(&Mutex<Vars>) -> R) -> R {`
- `52: ENV_CONTEXT.provide(Mutex::new(snapshot()), f)`
- `64: ENV_CONTEXT.provide(Mutex::new(vars), f)`
- `121: mod tests;`

### crates/vw-agent/src/file/event.rs

- `10: #[path = "event_tests.rs"]`
- `11: mod event_tests;`

### crates/vw-agent/src/file/ignore.rs

- `9: use std::sync::LazyLock;`
- `12: #[path = "ignore_tests.rs"]`
- `13: mod ignore_tests;`
- `15: static FOLDERS: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {`
- `50: static FILES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {`
- `66: static FILE_GLOBS: LazyLock<Vec<Pattern>> =`
- `67: LazyLock::new(|| FILES.iter().filter_map(|p| Pattern::new(p).ok()).collect());`
- `69: pub static PATTERNS: LazyLock<Vec<String>> = LazyLock::new(|| {`

### crates/vw-agent/src/file/mod.rs

- `13: mod fs_ops;`
- `14: mod git_status;`
- `15: mod pathing;`
- `16: mod search;`
- `17: mod text;`
- `18: mod types;`
- `21: mod text_tests;`
- `24: #[path = "fs_ops_tests.rs"]`
- `25: mod fs_ops_tests;`
- `27: #[path = "git_status_tests.rs"]`
- `28: mod git_status_tests;`
- `30: #[path = "pathing_tests.rs"]`
- `31: mod pathing_tests;`
- `33: #[path = "search_tests.rs"]`
- `34: mod search_tests;`
- `36: #[path = "types_tests.rs"]`
- `37: mod types_tests;`
- `39: pub mod event;`
- `40: pub mod ignore;`
- `41: pub mod ripgrep;`
- `42: pub mod time;`
- `43: pub mod watcher;`
- `46: mod tests;`

### crates/vw-agent/src/file/ripgrep.rs

- `14: #[path = "ripgrep_tests.rs"]`
- `15: mod ripgrep_tests;`

### crates/vw-agent/src/file/search.rs

- `5: use std::sync::LazyLock;`
- `9: static LOG: LazyLock<log::Logger> = LazyLock::new(|| {`

### crates/vw-agent/src/file/time.rs

- `9: use std::sync::LazyLock;`
- `10: use std::sync::{Arc, Mutex};`
- `14: #[path = "time_tests.rs"]`
- `15: mod time_tests;`
- `17: static READ: LazyLock<Mutex<HashMap<String, HashMap<String, SystemTime>>>> =`
- `18: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `20: static LOCKS: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =`
- `21: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `23: static LOCKS_SYNC: LazyLock<Mutex<HashMap<String, Arc<std::sync::Mutex<()>>>>> =`
- `24: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `69: .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))`
- `93: .or_insert_with(|| Arc::new(std::sync::Mutex::new(())))`

### crates/vw-agent/src/file/time_tests.rs

- `11: use std::sync::{Arc, Mutex};`
- `243: let events = Arc::new(tokio::sync::Mutex::new(Vec::new()));`
- `305: let events = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/file/watcher.rs

- `42: use std::sync::LazyLock;`
- `43: use std::sync::Mutex;`
- `47: #[path = "watcher_tests.rs"]`
- `48: mod watcher_tests;`
- `53: pub mod event {`
- `106: /// 使用 Mutex 包装确保多线程环境下的安全访问。`
- `108: static TASKS: LazyLock<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>> =`
- `109: LazyLock::new(|| Mutex::new(HashMap::new()));`

### crates/vw-agent/src/file/watcher_tests.rs

- `9: use std::sync::{Arc, Mutex};`
- `63: let seen = Arc::new(Mutex::new(Vec::<Value>::new()));`
- `218: let seen = Arc::new(Mutex::new(Vec::<Value>::new()));`
- `258: async fn wait_for_watcher_event(seen: &Arc<Mutex<Vec<Value>>>, file: &str, event: &str) {`

### crates/vw-agent/src/format/format/detect_tests.rs

- `12: use std::sync::{Arc, Mutex};`
- `123: enabled: Mutex::new(HashMap::from([("cached".to_string(), true)])),`

### crates/vw-agent/src/format/format/runtime.rs

- `14: use std::sync::LazyLock;`
- `15: use std::sync::OnceLock;`
- `19: static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();`
- `45: static INIT: LazyLock<()> = LazyLock::new(|| {`
- `62: LazyLock::force(&INIT);`

### crates/vw-agent/src/format/format/state.rs

- `12: use std::sync::LazyLock;`
- `13: use std::sync::{Arc, Mutex};`
- `15: pub(super) static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `62: pub(super) enabled: Mutex<HashMap<String, bool>>,`
- `79: State { enabled: Mutex::new(HashMap::new()), formatters }`

### crates/vw-agent/src/format/format/state_tests.rs

- `5: use std::sync::Mutex;`
- `86: let state = State { enabled: Mutex::new(HashMap::new()), formatters: HashMap::new() };`

### crates/vw-agent/src/format/format.rs

- `6: mod builtins;`
- `7: mod detect;`
- `8: mod runtime;`
- `9: mod state;`
- `12: mod builtins_tests;`
- `14: mod detect_tests;`
- `16: mod runtime_tests;`
- `18: mod state_tests;`

### crates/vw-agent/src/format/mod.rs

- `18: pub mod format;`

### crates/vw-agent/src/gateway/agent.rs

- `254: #[path = "agent_tests.rs"]`
- `255: mod agent_tests;`

### crates/vw-agent/src/gateway/agent_apps/mod.rs

- `2: pub(crate) mod profile_store;`
- `4: pub(crate) mod profile_operations;`
- `6: pub(crate) mod profile_skills;`

### crates/vw-agent/src/gateway/agent_apps/profile_operations.rs

- `10: sync::{Condvar, Mutex, OnceLock},`
- `25: type Locks = (Mutex<HashSet<Resource>>, Condvar);`
- `27: static LOCKS: OnceLock<Locks> = OnceLock::new();`
- `28: LOCKS.get_or_init(|| (Mutex::new(HashSet::new()), Condvar::new()))`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/mod.rs

- `1: pub mod model_check;`
- `2: pub mod reachability;`
- `3: pub mod speedtest;`
- `4: pub mod stream_check;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check.rs

- `36: pub(super) mod response;`
- `37: pub(super) mod target;`
- `224: #[path = "model_check_tests.rs"]`
- `225: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/reachability.rs

- `309: #[path = "reachability_tests.rs"]`
- `310: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/speedtest.rs

- `143: #[path = "speedtest_tests.rs"]`
- `144: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/stream_check.rs

- `238: #[path = "stream_check_tests.rs"]`
- `239: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_jobs/endpoints.rs

- `69: #[path = "endpoints_tests.rs"]`
- `70: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_diagnostic_jobs.rs

- `15: use parking_lot::Mutex;`
- `36: mod endpoints;`
- `76: entries: Mutex<HashMap<TaskId, Job>>,`
- `77: endpoints: Mutex<endpoints::Candidates>,`
- `182: entries: Mutex::new(entries),`
- `183: endpoints: Mutex::default(),`
- `553: #[path = "agent_apps_diagnostic_jobs_tests.rs"]`
- `554: mod tests;`

### crates/vw-agent/src/gateway/agent_apps_failover_runtime.rs

- `13: pub(crate) events: parking_lot::RwLock<Option<tokio::sync::broadcast::Sender<serde_json::Value>>>,`
- `19: events: parking_lot::RwLock::new(None),`

### crates/vw-agent/src/gateway/agent_apps_skills_store.rs

- `730: static SKILL_EXECUTION_EPOCH: std::sync::LazyLock<String> =`
- `731: std::sync::LazyLock::new(|| uuid::Uuid::new_v4().to_string());`
- `1329: #[path = "agent_apps_skill_migration.rs"]`
- `1330: mod migration;`

### crates/vw-agent/src/gateway/agent_tests.rs

- `14: use parking_lot::Mutex;`
- `54: entries: Mutex<Vec<(String, String, MemoryCategory)>>,`
- `115: events: Mutex<Vec<ObserverEvent>>,`
- `149: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/aggregation_resync.rs

- `55: pub(crate) failure: Arc<std::sync::Mutex<Option<StatusCode>>>,`
- `579: mod codex;`
- `580: mod route;`
- `582: mod publish;`
- `584: mod exit;`

### crates/vw-agent/src/gateway/api/auth.rs

- `52: #[path = "auth_tests.rs"]`
- `53: mod auth_tests;`

### crates/vw-agent/src/gateway/api/auth_tests.rs

- `10: use parking_lot::Mutex;`
- `32: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/api/handlers/agent_app_prompt_sync.rs

- `4: use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};`
- `15: pub(super) struct Jobs(Arc<Mutex<HashMap<String, Arc<Job>>>>);`
- `22: results: Mutex<Vec<TargetResult>>,`
- `97: owner: contexts[0].2.clone(), apps: q.apps, results: Mutex::new(Vec::new()) });`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs

- `1892: #[path = "aggregation_lifecycle_access.rs"]`
- `1893: mod aggregation_lifecycle;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs

- `18: use parking_lot::Mutex;`
- `204: progress: Mutex::default(),`
- `217: pub(super) struct Operations(Mutex<HashMap<Uuid, Arc<RefreshOperation>>>);`
- `257: progress: Mutex<Progress>,`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh_tests.rs

- `482: #[path = "../apps_usage_acceptance_tests.rs"]`
- `483: mod acceptance;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs

- `217: #[path = "launch_tests.rs"]`
- `218: mod tests;`
- `221: #[path = "../app_providers/agy_tests.rs"]`
- `222: mod agy_tests;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/launch_sessions.rs

- `5: sync::{Arc, Mutex, mpsc},`
- `20: pub(super) struct Sessions(Arc<Mutex<HashMap<String, (Scope, LaunchSessionPhase)>>>);`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs

- `14: sync::{Arc, Mutex},`
- `34: store: Arc<Mutex<Store>>,`
- `83: .layer(Extension(State { policy, store: Arc::new(Mutex::new(Store::default())) }))`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle_tests.rs

- `26: static WRITES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs

- `193: mod read;`
- `195: mod read_tests;`
- `197: mod access;`
- `366: pub(super) mod lifecycle;`
- `368: mod lifecycle_tests;`
- `370: mod preferences;`
- `372: mod preferences_tests;`
- `374: mod launch;`
- `376: mod launch_identities;`
- `377: mod launch_sessions;`
- `380: mod remote_tests_tests;`
- `383: mod local_access_tests;`
- `452: mod recovery_tests;`
- `543: mod import_tests;`
- `546: mod usage_refresh;`
- `554: mod usage_read;`
- `557: mod usage_refresh_tests;`
- `560: mod usage_read_tests;`
- `563: mod apps_usage_refresh;`
- `565: mod apps_usage_refresh_tests;`
- `567: mod usage_codex;`
- `569: mod usage_native;`
- `572: mod apps_usage_script;`
- `574: mod apps_usage_script_tests;`
- `664: #[path = "../hermes_dashboard.rs"]`
- `665: mod hermes_dashboard;`
- `671: #[path = "../hermes_memory.rs"]`
- `672: mod hermes_memory;`
- `681: #[path = "../workspace_memory.rs"]`
- `682: mod workspace_memory;`
- `685: #[path = "../openclaw_settings.rs"]`
- `686: mod openclaw_settings;`
- `689: #[path = "../workspace_directory.rs"]`
- `690: mod workspace_directory;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs

- `16: use parking_lot::Mutex;`
- `117: state: Mutex<State>,`
- `134: state: Mutex::default(),`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_integration_tests.rs

- `16: sync::{Arc, Mutex},`
- `99: struct OAuth(Arc<Mutex<Vec<&'static str>>>);`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_redaction_tests.rs

- `12: Arc, Mutex,`
- `57: records: Arc<Mutex<Vec<String>>>,`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_start/native.rs

- `9: sync::Mutex,`
- `50: pub(super) struct ClaudeAttempts(Mutex<HashMap<String, ClaudeAttempt>>);`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_start.rs

- `24: sync::{Arc, OnceLock},`
- `49: mod native;`
- `56: kimi: OnceLock<Result<KimiDeviceAuth, AuthErrorCode>>,`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_data/data_aux_api.rs

- `5: use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}, time::{Instant, Duration}};`
- `14: struct State { policy: Option<PathBuf>, downloads: Arc<Mutex<HashMap<String, Download>>> }`
- `36: .layer(Extension(State { policy, downloads: Arc::new(Mutex::new(HashMap::new())) }))`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_data/migration_read_api.rs

- `13: sync::{Arc, Mutex},`
- `34: pub(super) cache: Arc<Mutex<Cache>>,`
- `72: router.layer(Extension(State { policy, cache: Arc::new(Mutex::new(Cache::default())) })).layer(`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_data/mod.rs

- `1: pub(crate) mod backup_crud_api;`
- `3: mod backup_capture;`
- `5: mod backup_crud_api_tests;`
- `8: pub(crate) mod periodic;`
- `10: pub(crate) mod migration_read_api;`
- `13: pub(crate) mod migration_write_api;`
- `16: mod migration_write_api_tests;`
- `19: pub(crate) mod data_aux_api;`
- `22: mod data_aux_api_tests;`
- `25: pub(crate) mod backup_restore_api;`
- `27: mod backup_restore_api_tests;`
- `30: mod migration_acceptance_tests;`
- `33: mod backup_acceptance_tests;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics.rs

- `473: #[path = "agent_apps_diagnostics_endpoint_drafts_tests.rs"]`
- `474: mod endpoint_drafts_tests;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_error_tests.rs

- `24: struct Capture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);`
- `63: let error = std::sync::Arc::new(std::sync::Mutex::new(Some(error)));`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability.rs

- `2: #[path = "agent_apps_high_availability_health.rs"]`
- `3: mod health;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/discovery_api.rs

- `4: use tokio::sync::Mutex;`
- `17: type Caches = Arc<Mutex<BTreeMap<String, Arc<Mutex<SkillDiscoveryCache>>>>>;`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/import_api.rs

- `56: mod native {`
- `58: use std::{collections::BTreeMap, path::PathBuf, sync::{LazyLock, Mutex}, time::{Duration, Instant}};`
- `74: static SELECTIONS: LazyLock<Mutex<BTreeMap<String, Selection>>> =`
- `75: LazyLock::new(|| Mutex::new(BTreeMap::new()));`

### crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/mod.rs

- `1: pub(crate) mod repos_api;`
- `3: pub(crate) mod search_api;`
- `6: mod search_api_tests;`
- `8: pub(crate) mod claude_api;`
- `10: mod claude_api_tests;`
- `12: pub(crate) mod discovery_api;`
- `14: mod discovery_api_tests;`
- `16: pub(crate) mod inventory_api;`
- `18: pub(crate) mod install_api;`
- `20: mod install_api_tests;`
- `22: pub(crate) mod import_api;`
- `24: pub(crate) mod updates_api;`
- `26: pub(crate) mod jobs_api;`
- `29: mod jobs_api_tests;`
- `32: mod updates_api_tests;`
- `34: pub(crate) mod bindings_api;`
- `36: pub(crate) mod uninstall_api;`
- `39: mod uninstall_api_tests;`
- `41: pub(crate) mod storage_api;`
- `44: mod macos_api_tests;`
- `46: pub(crate) mod backups_api;`
- `49: mod backups_api_tests;`
- `52: mod macos_files_tests;`

### crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs

- `6: use std::sync::{Arc, Mutex};`
- `20: use tokio::sync::Mutex as AsyncMutex;`
- `683: locks: Arc<Mutex<HashMap<PathBuf, ProjectDispatchLock>>>,`
- `2777: state: std::sync::Arc<std::sync::Mutex<FakeAgentSchedulerState>>,`
- `2979: #[path = "agent_scheduler_tests.rs"]`
- `2980: mod agent_scheduler_tests;`

### crates/vw-agent/src/gateway/api/handlers/agent_scheduler_tests.rs

- `10: use std::sync::{Arc, Mutex, mpsc};`
- `1756: started: Mutex::new(Some(started_tx)),`
- `1757: release: Mutex::new(release_rx),`
- `2444: started: Mutex<Option<mpsc::Sender<()>>>,`
- `2445: release: Mutex<mpsc::Receiver<()>>,`

### crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings_tests.rs

- `1: #[path = "aggregation_codex_settings_fixture.rs"]`
- `2: mod fixture;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_desktop.rs

- `98: mod native {`
- `321: #[path = "aggregation_desktop_write.rs"]`
- `322: mod write;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_tests.rs

- `128: #[path = "aggregation_desktop_routing_tests.rs"]`
- `129: mod routing_tests;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_members_client_tests.rs

- `239: #[path = "aggregation_members_ordinary_tests.rs"]`
- `240: mod ordinary_tests;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_members_tests.rs

- `514: #[path = "aggregation_members_client_tests.rs"]`
- `515: mod client_tests;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_omo.rs

- `153: mod native {`

### crates/vw-agent/src/gateway/api/handlers/aggregation_omp.rs

- `263: use std::sync::LazyLock;`
- `264: static KEY: LazyLock<Option<ring::hmac::Key>> = LazyLock::new(|| {`

### crates/vw-agent/src/gateway/api/handlers/aggregation_publish_tests.rs

- `437: #[path = "aggregation_publish_client_tests.rs"]`
- `438: mod clients;`
- `440: #[path = "aggregation_publish_operation_tests.rs"]`
- `441: mod operation_contract;`
- `443: #[path = "aggregation_publish_failure_tests.rs"]`
- `444: mod failure_contract;`
- `446: #[path = "../../aggregation_lifecycle_tests.rs"]`
- `447: mod lifecycle_tests;`
- `449: #[path = "aggregation_exit_tests.rs"]`
- `450: mod exit_contract;`

### crates/vw-agent/src/gateway/api/handlers/aggregation_read_snapshot.rs

- `6: use std::{path::Path, sync::LazyLock};`
- `319: static KEY: LazyLock<Result<ring::hmac::Key, StatusCode>> = LazyLock::new(|| {`

### crates/vw-agent/src/gateway/api/handlers/aggregation_restart_tests.rs

- `30: router: std::sync::OnceLock<Router>,`
- `79: let f = Self { root, db, tx, router: std::sync::OnceLock::new() };`

### crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs

- `10: sync::{Arc, Mutex},`
- `26: struct Logs(Arc<Mutex<Vec<u8>>>);`

### crates/vw-agent/src/gateway/api/handlers/app_providers/catalog_tests.rs

- `44: mod host_reads {`

### crates/vw-agent/src/gateway/api/handlers/app_providers/claude.rs

- `7: #[path = "claude_form.rs"]`
- `8: mod form;`
- `571: #[path = "claude_login.rs"]`
- `572: mod login;`

### crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login_tests.rs

- `5: sync::{Arc, Mutex},`
- `14: struct Keychain(Mutex<BTreeMap<Option<PathBuf>, Value>>);`
- `137: #[path = "claude_managed_binding_tests.rs"]`
- `138: mod managed_binding_tests;`

### crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs

- `509: #[path = "claude_login_tests.rs"]`
- `510: mod claude_login_tests;`

### crates/vw-agent/src/gateway/api/handlers/app_providers/editor_merge.rs

- `13: sync::{Arc, Mutex},`
- `29: contexts: Arc<Mutex<HashMap<String, Context>>>,`

### crates/vw-agent/src/gateway/api/handlers/app_providers/env_clean_tests.rs

- `35: mod execution {`

### crates/vw-agent/src/gateway/api/handlers/app_providers/env_restore_tests.rs

- `35: mod execution {`

### crates/vw-agent/src/gateway/api/handlers/app_providers/mode_apply_tests.rs

- `8: Arc, Mutex,`
- `26: active: Mutex<Option<String>>,`
- `27: published: Mutex<Vec<Option<String>>>,`
- `173: active: Mutex::new(None),`
- `174: published: Mutex::new(vec![]),`

### crates/vw-agent/src/gateway/api/handlers/app_providers/operations/aggregation.rs

- `18: outcome: Arc<Mutex<Option<AggregationOutcome>>>,`
- `116: let outcome = Arc::new(Mutex::new(None));`
- `221: #[path = "../../aggregation_restart_operation_tests.rs"]`
- `222: mod restart_tests;`
- `225: #[path = "../../aggregation_exit_operation_tests.rs"]`
- `226: mod exit_tests;`

### crates/vw-agent/src/gateway/api/handlers/app_providers/operations.rs

- `16: sync::{Arc, Mutex},`
- `28: operations: Arc<Mutex<HashMap<String, Running>>>,`
- `36: editor_outcome: Arc<Mutex<Option<ProviderEditorOutcome>>>,`
- `63: operations: Arc::new(Mutex::new(HashMap::new())),`
- `178: let editor_outcome = Arc::new(Mutex::new(None));`
- `340: pub(in crate::gateway::api::handlers) mod aggregation;`

### crates/vw-agent/src/gateway/api/handlers/app_providers.rs

- `164: #[path = "app_providers/catalog_tests.rs"]`
- `165: mod catalog_tests;`
- `167: mod catalog;`
- `169: mod create;`
- `171: #[path = "app_providers/create_tests.rs"]`
- `172: mod create_tests;`
- `174: mod update;`
- `176: #[path = "app_providers/update_tests.rs"]`
- `177: mod update_tests;`
- `179: mod delete;`
- `181: #[path = "app_providers/delete_tests.rs"]`
- `182: mod delete_tests;`
- `184: #[path = "app_providers/duplicate.rs"]`
- `185: mod duplicate;`
- `187: #[path = "app_providers/duplicate_tests.rs"]`
- `188: mod duplicate_tests;`
- `190: #[path = "app_providers/presentation.rs"]`
- `191: mod presentation;`
- `193: #[path = "app_providers/presentation_tests.rs"]`
- `194: mod presentation_tests;`
- `196: #[path = "app_providers/endpoints.rs"]`
- `197: mod endpoints;`
- `199: #[path = "app_providers/endpoints_tests.rs"]`
- `200: mod endpoints_tests;`
- `202: #[path = "app_providers/icons.rs"]`
- `203: mod icons;`
- `205: #[path = "app_providers/icons_tests.rs"]`
- `206: mod icons_tests;`
- `208: mod universal;`
- `210: #[path = "app_providers/universal_tests.rs"]`
- `211: mod universal_tests;`
- `213: mod universal_sync;`
- `215: #[path = "app_providers/universal_sync_tests.rs"]`
- `216: mod universal_sync_tests;`
- `218: #[path = "app_providers/paths.rs"]`
- `219: mod paths;`
- `221: #[path = "app_providers/paths_tests.rs"]`
- `222: mod paths_tests;`
- `224: mod preview;`
- `226: #[path = "app_providers/preview_tests.rs"]`
- `227: mod preview_tests;`
- `230: #[path = "app_providers/validation_tests.rs"]`
- `231: mod validation_tests;`
- `233: mod editor_merge;`
- `235: mod editor_merge_tests;`
- `237: #[path = "app_providers/snippets.rs"]`
- `238: mod snippets;`
- `240: #[path = "app_providers/snippets_tests.rs"]`
- `241: mod snippets_tests;`
- `243: pub(super) mod claude;`
- `245: mod claude_tests;`
- `248: mod codex_tests;`
- `250: pub(super) mod codex_catalog;`
- `252: mod codex_catalog_tests;`
- `254: mod gemini;`
- `256: mod gemini_tests;`
- `258: mod claude_desktop;`
- `259: mod grok;`
- `261: mod grok_tests;`
- `264: mod claude_desktop_tests;`
- `266: pub(super) mod desktop_mapping;`
- `268: mod desktop_mapping_tests;`
- `270: mod hermes;`
- `272: mod hermes_tests;`
- `273: mod openclaw;`
- `275: mod openclaw_tests;`
- `276: mod opencode;`
- `278: mod opencode_tests;`
- `281: #[path = "aggregation_pi_tests.rs"]`
- `282: mod aggregation_pi_tests;`
- `283: mod pi;`
- `285: mod pi_tests;`
- `287: pub(super) mod omp;`
- `289: mod omp_tests;`
- `291: mod mcode;`
- `293: mod mcode_tests;`
- `295: mod kimi;`
- `297: mod kimi_tests;`
- `299: mod dsh;`
- `301: mod dsh_tests;`
- `303: pub(super) mod agy;`
- `305: mod import;`
- `307: mod import_tests;`
- `309: mod mode_state;`
- `311: mod mode_state_tests;`
- `313: pub(crate) mod mode_apply;`
- `315: mod mode_apply_tests;`
- `317: mod env_diagnose;`
- `319: mod env_diagnose_tests;`
- `321: mod env_clean;`
- `323: mod env_clean_tests;`
- `325: mod env_restore;`
- `327: mod env_restore_tests;`
- `329: pub(super) mod operations;`
- `331: mod operations_tests;`
- `334: mod api_security_tests;`
- `336: mod basic;`
- `338: mod basic_tests;`
- `340: mod request_options;`
- `342: mod request_options_tests;`

### crates/vw-agent/src/gateway/api/handlers/app_routing/mod.rs

- `1: mod config_api;`
- `2: mod lifecycle_api;`
- `3: mod outbound_api;`
- `4: mod outbound_probe;`
- `5: mod outbound_scan;`
- `7: #[path = "../agent_apps_high_availability.rs"]`
- `8: pub(crate) mod agent_apps_high_availability;`
- `22: mod config_api_tests;`
- `24: pub(super) mod lifecycle_api_tests;`
- `27: mod outbound_api_tests;`
- `30: mod outbound_probe_tests;`
- `33: mod outbound_scan_tests;`
- `35: mod request_status;`
- `38: mod mode_attach;`
- `41: mod client_host;`

### crates/vw-agent/src/gateway/api/handlers/auth.rs

- `40: #[path = "auth_tests.rs"]`
- `41: mod auth_tests;`

### crates/vw-agent/src/gateway/api/handlers/auth_tests.rs

- `9: use std::sync::OnceLock;`
- `13: static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();`
- `14: LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await`
- `18: static HOME: OnceLock<PathBuf> = OnceLock::new();`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/dingtalk.rs

- `137: #[path = "dingtalk_tests.rs"]`
- `138: mod dingtalk_tests;`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/feishu.rs

- `126: #[path = "feishu_tests.rs"]`
- `127: mod feishu_tests;`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/feishu_tests.rs

- `8: use std::sync::{Arc, Mutex};`
- `12: requests: Arc<Mutex<Vec<String>>>,`
- `13: responses: Arc<Mutex<Vec<Value>>>,`
- `25: async fn mock_server(responses: Vec<Value>) -> (String, Arc<Mutex<Vec<String>>>) {`
- `26: let requests = Arc::new(Mutex::new(Vec::new()));`
- `28: MockState { requests: Arc::clone(&requests), responses: Arc::new(Mutex::new(responses)) };`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/qq.rs

- `160: #[path = "qq_tests.rs"]`
- `161: mod qq_tests;`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/wechat.rs

- `149: #[path = "wechat_tests.rs"]`
- `150: mod wechat_tests;`

### crates/vw-agent/src/gateway/api/handlers/channel_qr/wecom.rs

- `153: #[path = "wecom_tests.rs"]`
- `154: mod wecom_tests;`

### crates/vw-agent/src/gateway/api/handlers/channel_qr.rs

- `6: use std::sync::{Mutex, OnceLock};`
- `20: mod dingtalk;`
- `21: mod feishu;`
- `22: mod qq;`
- `23: mod wechat;`
- `24: mod wecom;`
- `123: static HANDLES: OnceLock<Mutex<HashMap<String, PendingHandle>>> = OnceLock::new();`
- `125: fn handles() -> &'static Mutex<HashMap<String, PendingHandle>> {`
- `126: HANDLES.get_or_init(|| Mutex::new(HashMap::new()))`
- `243: #[path = "channel_qr_tests.rs"]`
- `244: mod channel_qr_tests;`

### crates/vw-agent/src/gateway/api/handlers/computer_control.rs

- `12: static EDIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/config.rs

- `457: #[path = "config_tests.rs"]`
- `458: mod config_tests;`

### crates/vw-agent/src/gateway/api/handlers/config_tests.rs

- `11: use parking_lot::Mutex;`
- `73: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/cron.rs

- `27: #[path = "cron_schedule.rs"]`
- `28: mod cron_schedule;`
- `782: #[path = "cron_tests.rs"]`
- `783: mod cron_tests;`

### crates/vw-agent/src/gateway/api/handlers/cron_tests.rs

- `35: fn state_with_config(config: Arc<parking_lot::Mutex<Config>>, require_pairing: bool) -> AppState {`
- `60: fn temp_config(workspace: &std::path::Path) -> Arc<parking_lot::Mutex<Config>> {`
- `63: Arc::new(parking_lot::Mutex::new(config))`
- `794: #[path = "cron_schedule_tests.rs"]`
- `795: mod cron_schedule_tests;`

### crates/vw-agent/src/gateway/api/handlers/desktop.rs

- `390: #[path = "desktop_tests.rs"]`
- `391: mod desktop_tests;`
- `928: -> &'static std::sync::Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>`
- `930: static PROBES: std::sync::OnceLock<`
- `931: std::sync::Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,`
- `932: > = std::sync::OnceLock::new();`

### crates/vw-agent/src/gateway/api/handlers/desktop_acp_install.rs

- `28: #[path = "desktop_acp_install_tests.rs"]`
- `29: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/desktop_browser_install.rs

- `29: #[path = "desktop_browser_install_tests.rs"]`
- `30: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/desktop_skills.rs

- `25: static BUILT_IN_SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/skills");`
- `1032: #[path = "desktop_skills_tests.rs"]`
- `1033: mod desktop_skills_tests;`

### crates/vw-agent/src/gateway/api/handlers/desktop_tests.rs

- `7: use tokio::sync::Mutex;`
- `15: static DESKTOP_STORAGE_LOCK: Mutex<()> = Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/external_sessions/catalog.rs

- `2: use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}, time::{Duration, Instant}};`
- `21: pub(super) struct Snapshots(Arc<Mutex<HashMap<String, Snapshot>>>);`

### crates/vw-agent/src/gateway/api/handlers/external_sessions/maintenance.rs

- `10: sync::{Arc, Mutex},`
- `19: pub(super) mod host;`
- `23: pub(super) struct Jobs(Arc<Mutex<HashMap<String, Job>>>);`
- `211: pub(super) mod history_settings;`

### crates/vw-agent/src/gateway/api/handlers/external_sessions/mod.rs

- `2: mod catalog;`
- `5: mod scan;`
- `7: mod messages;`
- `9: mod stream;`
- `11: mod content;`
- `13: mod images;`
- `15: mod delete;`
- `17: mod resume;`
- `19: mod maintenance;`
- `21: mod compression;`
- `23: mod markdown;`
- `25: pub(crate) mod usage_scan;`

### crates/vw-agent/src/gateway/api/handlers/external_sessions/stream.rs

- `14: sync::{Arc, Mutex},`
- `24: pub(super) struct Reads(Arc<Mutex<HashMap<String, Entry>>>);`

### crates/vw-agent/src/gateway/api/handlers/external_sessions/usage_scan.rs

- `2: use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::Instant};`
- `9: static SCHEDULES: OnceLock<Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>>> = OnceLock::new();`
- `10: pub(in super::super) fn schedules() -> &'static Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>> {`

### crates/vw-agent/src/gateway/api/handlers/file.rs

- `18: #[path = "file_duplicate.rs"]`
- `19: mod file_duplicate;`
- `20: #[path = "file_import.rs"]`
- `21: mod file_import;`
- `953: #[path = "file_tests.rs"]`
- `954: mod file_tests;`
- `957: #[path = "file_import_tests.rs"]`
- `958: mod file_import_tests;`
- `961: #[path = "file_drag_integration_tests.rs"]`
- `962: mod file_drag_integration_tests;`

### crates/vw-agent/src/gateway/api/handlers/file_drag_integration_tests.rs

- `8: use parking_lot::Mutex;`
- `36: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/api/handlers/file_duplicate.rs

- `78: #[path = "file_duplicate_tests.rs"]`
- `79: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/git/repository.rs

- `9: #[path = "history.rs"]`
- `10: mod history;`
- `11: #[path = "remote.rs"]`
- `12: mod remote;`
- `63: #[path = "repository_tests.rs"]`
- `64: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/git.rs

- `28: mod repository;`
- `835: #[path = "git_tests.rs"]`
- `836: mod git_tests;`

### crates/vw-agent/src/gateway/api/handlers/git_command.rs

- `78: #[path = "git_command_tests.rs"]`
- `79: mod git_command_tests;`

### crates/vw-agent/src/gateway/api/handlers/global.rs

- `181: static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`
- `202: #[path = "global_tests.rs"]`
- `203: mod global_tests;`

### crates/vw-agent/src/gateway/api/handlers/global_tests.rs

- `14: use parking_lot::Mutex;`
- `23: const BOOTSTRAP_SOURCE: &str = include_str!("../../bootstrap.rs");`
- `78: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/hermes_dashboard.rs

- `3: use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};`
- `14: jobs: Arc<Mutex<HashMap<String, (ProjectScope, String, String, Operation)>>>,`
- `98: #[path = "hermes_dashboard_tests.rs"]`
- `99: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/hermes_memory.rs

- `94: #[path = "hermes_memory_tests.rs"]`
- `95: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/hooks.rs

- `237: #[path = "hooks_tests.rs"]`
- `238: mod hooks_tests;`

### crates/vw-agent/src/gateway/api/handlers/hooks_tests.rs

- `12: use parking_lot::Mutex;`
- `83: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/instance.rs

- `167: #[path = "instance_tests.rs"]`
- `168: mod instance_tests;`

### crates/vw-agent/src/gateway/api/handlers/integrations.rs

- `255: #[path = "integrations_tests.rs"]`
- `256: mod integrations_tests;`

### crates/vw-agent/src/gateway/api/handlers/integrations_tests.rs

- `80: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/knowledge.rs

- `554: #[path = "knowledge_tests.rs"]`
- `555: mod knowledge_tests;`

### crates/vw-agent/src/gateway/api/handlers/knowledge_tests.rs

- `13: use tokio::sync::Mutex;`
- `25: static KNOWLEDGE_TEST_LOCK: Mutex<()> = Mutex::const_new(());`
- `980: config: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/api/handlers/mcp.rs

- `40: #[path = "mcp_tests.rs"]`
- `41: mod mcp_tests;`

### crates/vw-agent/src/gateway/api/handlers/mcp_management_resync.rs

- `15: sync::{Arc, Mutex},`
- `29: pub(super) struct Jobs(Arc<Mutex<HashMap<String, Arc<Job>>>>);`
- `36: reports: Mutex<Vec<TargetResult>>,`
- `37: outcome: Mutex<Option<McpBatchOutcome>>,`
- `151: reports: Mutex::new(Vec::new()),`
- `152: outcome: Mutex::new(None),`
- `450: reports: Mutex::new(Vec::new()), outcome: Mutex::new(None) });`

### crates/vw-agent/src/gateway/api/handlers/mcp_tests.rs

- `12: use parking_lot::Mutex;`
- `37: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/memory.rs

- `107: #[path = "memory_tests.rs"]`
- `108: mod memory_tests;`

### crates/vw-agent/src/gateway/api/handlers/memory_tests.rs

- `14: use parking_lot::Mutex;`
- `33: recalled: Mutex<Vec<(String, usize)>>,`
- `34: listed: Mutex<Vec<Option<MemoryCategory>>>,`
- `35: stored: Mutex<Vec<(String, String, MemoryCategory)>>,`
- `36: forgotten: Mutex<Vec<String>>,`
- `137: config: Arc::new(parking_lot::Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/api/handlers/misc.rs

- `147: #[path = "misc_tests.rs"]`
- `148: mod misc_tests;`

### crates/vw-agent/src/gateway/api/handlers/mod.rs

- `44: pub mod config;`
- `47: pub mod auth;`
- `50: pub(crate) mod channel_qr;`
- `53: pub mod agent_scheduler;`
- `56: pub mod desktop;`
- `59: pub(crate) mod desktop_skills;`
- `61: mod desktop_acp_install;`
- `62: mod desktop_browser_install;`
- `64: pub(crate) mod plugins;`
- `67: pub(crate) mod hooks;`
- `70: pub(crate) mod skill_market;`
- `73: pub(crate) mod shiju_account;`
- `74: pub(crate) mod shiju_apps;`
- `75: mod shiju_connectors;`
- `76: mod shiju_knowledge;`
- `78: mod shiju_knowledge_gateway_tests;`
- `79: pub(crate) mod shiju_skills;`
- `84: pub mod cron;`
- `89: pub mod integrations;`
- `92: pub mod file;`
- `95: pub mod git;`
- `98: pub(crate) mod git_command;`
- `101: pub mod global;`
- `104: pub mod knowledge;`
- `107: pub mod instance;`
- `112: pub mod memory;`
- `115: pub mod mcp;`
- `118: pub mod misc;`
- `121: pub mod permission;`
- `124: pub mod project;`
- `125: pub mod worktree_management;`
- `128: pub mod provider;`
- `131: pub mod pty;`
- `134: pub mod question;`
- `137: pub(super) mod resumable_stream;`
- `140: pub mod session;`
- `145: pub mod status;`
- `148: pub mod task_pool;`
- `151: pub mod task_lifecycle;`
- `154: pub mod task_plan_run;`
- `159: pub mod tools;`
- `162: pub mod workflow;`
- `229: mod tests;`
- `232: mod dynamic_tools_tests;`
- `235: pub(crate) mod dynamic_tools;`
- `237: pub mod agent_apps_error;`
- `239: mod agent_apps_error_tests;`
- `242: mod agent_apps_read_tests;`
- `245: pub(crate) mod app_routing;`
- `247: pub(crate) mod agent_apps;`
- `250: mod agent_apps_high_availability_tests;`
- `253: mod agent_apps_ha_acceptance_tests;`
- `255: pub(crate) mod agent_apps_auth_read;`
- `257: mod agent_apps_auth_read_tests;`
- `259: mod agent_apps_auth_binding;`
- `261: mod agent_apps_auth_binding_tests;`
- `263: mod agent_apps_auth_cancel;`
- `265: mod agent_apps_auth_cancel_tests;`
- `267: mod agent_apps_auth_default;`
- `269: mod agent_apps_auth_default_tests;`
- `272: mod agent_apps_auth_integration_tests;`
- `274: mod agent_apps_auth_logout;`
- `276: mod agent_apps_auth_logout_tests;`
- `278: mod agent_apps_auth_poll;`
- `280: mod agent_apps_auth_poll_tests;`
- `283: mod agent_apps_auth_redaction_tests;`
- `285: mod agent_apps_auth_remove;`
- `287: mod agent_apps_auth_remove_tests;`
- `290: mod agent_apps_auth_security_tests;`
- `292: mod agent_apps_auth_start;`
- `294: mod agent_apps_auth_start_tests;`
- `297: mod agent_apps_add_tests;`
- `300: mod agent_apps_rename_tests;`
- `303: mod agent_apps_delete_tests;`
- `306: mod agent_apps_acceptance_tests;`
- `308: pub(crate) mod app_providers;`
- `311: mod app_providers_tests;`
- `313: pub(crate) mod model_catalog;`
- `315: mod model_catalog_acceptance_tests;`
- `316: mod model_catalog_edits;`
- `318: mod model_catalog_tests;`
- `321: mod model_discovery;`
- `323: mod model_discovery_tests;`
- `325: mod mcp_management_create;`
- `327: mod mcp_management_create_tests;`
- `328: mod mcp_management_delete;`
- `330: mod mcp_management_delete_tests;`
- `331: mod mcp_management_read;`
- `333: mod mcp_management_read_tests;`
- `335: mod mcp_management_update;`
- `337: mod mcp_management_update_tests;`
- `339: mod aggregation_members;`
- `341: mod aggregation_members_tests;`
- `342: mod aggregation_omo;`
- `344: mod aggregation_omo_tests;`
- `345: mod aggregation_omp;`
- `346: mod aggregation_pi;`
- `347: mod aggregation_read;`
- `349: mod aggregation_read_tests;`
- `352: pub(crate) mod aggregation_read_snapshot;`
- `355: pub(crate) mod voice;`
- `356: pub(crate) mod voice_handoff;`
- `358: mod voice_handoff_session_tests;`
- `359: pub(crate) mod voice_realtime;`
- `362: mod aggregation_codex_settings_tests;`
- `365: mod aggregation_omp_tests;`
- `368: mod aggregation_desktop_tests;`
- `371: pub(crate) mod aggregation_resync;`
- `373: mod aggregation_codex_settings;`
- `376: mod mcp_management_bindings_tests;`
- `378: mod mcp_management_bindings;`
- `381: mod aggregation_resync_tests;`
- `383: mod aggregation_desktop;`
- `386: mod aggregation_resync_codex_tests;`
- `389: pub(crate) mod agent_apps_diagnostics;`
- `391: mod agent_apps_diagnostics_acceptance_tests;`
- `393: mod agent_apps_diagnostics_tests;`
- `395: mod mcp_management_import;`
- `397: mod mcp_management_import_tests;`
- `399: mod voice_settings;`
- `402: mod aggregation_publish_tests;`
- `404: mod aggregation_publish;`
- `407: mod mcp_management_resync_tests;`
- `409: mod mcp_management_resync;`
- `412: mod mcp_management_probe_tests;`
- `415: mod aggregation_clients_tests;`
- `418: mod mcp_management_probe;`
- `420: mod aggregation_clients;`
- `423: mod aggregation_restart_tests;`
- `425: mod aggregation_restart;`
- `427: mod aggregation_exit;`
- `430: mod mcp_management_tools;`
- `433: mod mcp_management_tools_tests;`
- `435: pub(crate) mod agent_apps_skills;`
- `437: pub(crate) mod agent_app_prompt_files;`
- `439: pub(crate) mod agent_app_prompts_read;`
- `441: pub(crate) mod agent_app_prompts_write;`
- `443: pub(crate) mod agent_app_prompts_activation;`
- `445: pub(crate) mod agent_app_prompt_templates;`
- `448: pub(crate) mod agent_app_prompt_sync;`
- `450: pub(crate) mod agent_app_prompt_shared_import;`
- `452: pub(crate) mod computer_control;`
- `454: mod computer_control_tests;`
- `456: pub(crate) mod project_profiles;`
- `458: pub(crate) mod external_sessions;`
- `459: mod backup_inventory;`
- `461: pub(crate) mod agent_apps_data;`
- `463: pub(crate) mod device_sync_jobs;`
- `464: mod device_sync_download;`
- `465: mod device_sync_projection;`
- `467: mod device_sync_config;`
- `469: mod device_sync_probe;`
- `471: mod device_sync_tasks;`
- `473: pub(crate) mod usage;`

### crates/vw-agent/src/gateway/api/handlers/model_catalog.rs

- `34: #[path = "model_catalog/presets.rs"]`
- `35: mod presets;`

### crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs

- `9: Arc, Mutex,`
- `113: requests: Arc<Mutex<Vec<Value>>>,`
- `158: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/gateway/api/handlers/model_catalog_tests.rs

- `303: #[path = "model_catalog_edits_tests.rs"]`
- `304: mod edits;`

### crates/vw-agent/src/gateway/api/handlers/model_discovery.rs

- `19: sync::{Arc, Mutex, MutexGuard},`
- `37: #[path = "model_discovery/source.rs"]`
- `38: mod source;`
- `46: tasks: Arc<Mutex<HashMap<String, Arc<Task>>>>,`
- `70: state: Mutex<ModelDiscoveryState>,`
- `216: state: Mutex::new(ModelDiscoveryState::Queued),`

### crates/vw-agent/src/gateway/api/handlers/openclaw_settings.rs

- `129: #[path = "openclaw_settings_tests.rs"]`
- `130: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/permission.rs

- `217: #[path = "permission_tests.rs"]`
- `218: mod permission_tests;`

### crates/vw-agent/src/gateway/api/handlers/plugins.rs

- `270: #[path = "plugins_tests.rs"]`
- `271: mod plugins_tests;`
- `273: #[path = "plugins_operations.rs"]`
- `274: mod operations;`

### crates/vw-agent/src/gateway/api/handlers/plugins_operations.rs

- `7: sync::{Mutex, OnceLock},`
- `52: fn pending() -> &'static Mutex<HashMap<String, Pending>> {`
- `53: static PENDING: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();`
- `54: PENDING.get_or_init(|| Mutex::new(HashMap::new()))`

### crates/vw-agent/src/gateway/api/handlers/project.rs

- `515: #[path = "project_tests.rs"]`
- `516: mod project_tests;`

### crates/vw-agent/src/gateway/api/handlers/project_profiles.rs

- `13: #[path = "project_profiles_maintenance.rs"]`
- `14: mod maintenance;`
- `15: #[path = "project_profiles_execution.rs"]`
- `16: mod execution;`

### crates/vw-agent/src/gateway/api/handlers/provider.rs

- `109: #[path = "provider_tests.rs"]`
- `110: mod provider_tests;`

### crates/vw-agent/src/gateway/api/handlers/pty.rs

- `168: #[path = "pty_tests.rs"]`
- `169: mod pty_tests;`

### crates/vw-agent/src/gateway/api/handlers/question.rs

- `45: #[path = "question_tests.rs"]`
- `46: mod question_tests;`

### crates/vw-agent/src/gateway/api/handlers/resumable_stream.rs

- `9: use std::sync::{Arc, Mutex, OnceLock};`
- `83: state: Mutex<RunState>,`
- `95: state: Arc<Mutex<StoreState>>,`
- `122: Self { state: Arc::new(Mutex::new(StoreState::default())), terminal_capacity, terminal_ttl }`
- `191: state: Mutex::new(RunState {`
- `418: static STORE: OnceLock<ResumableStreamStore> = OnceLock::new();`
- `434: fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {`
- `439: #[path = "resumable_stream_tests.rs"]`
- `440: mod resumable_stream_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs

- `5: use std::sync::{Arc, Mutex};`
- `96: let requests = Arc::new(Mutex::new(Vec::<Value>::new()));`
- `277: let contracts: Value = serde_json::from_str(include_str!(concat!(`

### crates/vw-agent/src/gateway/api/handlers/session/archived_delete_tests.rs

- `80: mod persistence {`

### crates/vw-agent/src/gateway/api/handlers/session/fork_acceptance_tests.rs

- `3: use std::sync::{Arc, Mutex};`
- `128: async fn serve_model(requests: Arc<Mutex<Vec<Value>>>) -> String {`
- `335: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/gateway/api/handlers/session/message_ops.rs

- `163: #[path = "message_ops_tests.rs"]`
- `164: mod message_ops_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/message_ops_tests.rs

- `16: static MESSAGE_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/session/raw_commit.rs

- `6: use std::sync::{LazyLock, Mutex};`
- `158: static PENDING_RAW_COMMITS: LazyLock<Mutex<PendingRawCommitRegistry>> =`
- `159: LazyLock::new(|| Mutex::new(PendingRawCommitRegistry::default()));`
- `191: #[path = "raw_commit_tests.rs"]`
- `192: mod raw_commit_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/session_ops.rs

- `739: #[path = "session_ops_tests.rs"]`
- `740: mod session_ops_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs

- `41: static SESSION_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`
- `69: config: Arc::new(parking_lot::Mutex::new(Config::default())),`
- `764: #[path = "turn_fork_tests.rs"]`
- `765: mod turn_fork_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/shared.rs

- `129: #[path = "shared_tests.rs"]`
- `130: mod shared_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/shared_tests.rs

- `14: static SHARED_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/session/stream.rs

- `9: use std::sync::{Arc, Mutex};`
- `1321: type SharedPromptCancel = Arc<Mutex<Option<agent_session::runtime::PromptCancelHandle>>>;`
- `1488: let sink = Arc::new(Mutex::new(Sink { send, sent_terminal: false }));`
- `1497: let patch_recorder = Arc::new(Mutex::new(StreamPatchRecorder::default()));`
- `2201: let prompt_cancel: SharedPromptCancel = Arc::new(Mutex::new(None));`
- `2786: #[path = "stream_tests.rs"]`
- `2787: mod stream_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/stream_tests.rs

- `2317: include_str!(concat!(`

### crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs

- `403: let requests = Arc::new(parking_lot::Mutex::new(Vec::<Value>::new()));`
- `703: let failed_directory = std::sync::Arc::new(std::sync::Mutex::new(None));`
- `816: #[path = "fork_acp_tests.rs"]`
- `817: mod fork_acp_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/ui_ops.rs

- `575: #[path = "ui_ops_tests.rs"]`
- `576: mod ui_ops_tests;`

### crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs

- `20: static UI_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/api/handlers/session.rs

- `10: mod acp_options;`
- `11: mod archived_delete;`
- `12: mod message_ops;`
- `13: mod raw_commit;`
- `14: mod session_ops;`
- `15: mod shared;`
- `16: pub(crate) mod stream;`
- `17: mod ui_ops;`
- `117: #[path = "session_tests.rs"]`
- `118: mod tests;`
- `121: #[path = "session/agent_profile_acceptance_tests.rs"]`
- `122: mod agent_profile_acceptance_tests;`
- `126: #[path = "session/fork_acceptance_tests.rs"]`
- `127: mod fork_acceptance_tests;`
- `130: #[path = "session/acp_source_tests.rs"]`
- `131: mod acp_source_tests;`
- `134: #[path = "session/archived_delete_tests.rs"]`
- `135: mod archived_delete_tests;`

### crates/vw-agent/src/gateway/api/handlers/session_tests.rs

- `48: config: Arc::new(parking_lot::Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/api/handlers/shiju_account.rs

- `178: #[path = "shiju_account_tests.rs"]`
- `179: mod shiju_account_tests;`

### crates/vw-agent/src/gateway/api/handlers/shiju_apps.rs

- `123: #[path = "shiju_apps_tests.rs"]`
- `124: mod shiju_apps_tests;`

### crates/vw-agent/src/gateway/api/handlers/shiju_skills.rs

- `276: #[path = "shiju_skills_tests.rs"]`
- `277: mod shiju_skills_tests;`

### crates/vw-agent/src/gateway/api/handlers/skill_market.rs

- `598: #[path = "skill_market_tests.rs"]`
- `599: mod skill_market_tests;`

### crates/vw-agent/src/gateway/api/handlers/skill_market_tests.rs

- `4: use std::sync::{Arc, Mutex};`
- `34: calls: Arc<Mutex<Vec<String>>>,`
- `40: fn new(source: &str, calls: Arc<Mutex<Vec<String>>>) -> Self {`
- `58: calls: Arc::new(Mutex::new(Vec::new())),`
- `288: let calls = Arc::new(Mutex::new(Vec::new()));`
- `344: let calls = Arc::new(Mutex::new(Vec::new()));`
- `431: let calls = Arc::new(Mutex::new(Vec::new()));`
- `457: let calls = Arc::new(Mutex::new(Vec::new()));`
- `552: let calls = Arc::new(Mutex::new(Vec::new()));`
- `605: let registry = registry(RecordingProvider::new("fixture", Arc::new(Mutex::new(Vec::new()))));`
- `631: let calls = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/gateway/api/handlers/status.rs

- `140: #[path = "status_tests.rs"]`
- `141: mod status_tests;`

### crates/vw-agent/src/gateway/api/handlers/status_tests.rs

- `41: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/api/handlers/task_lifecycle.rs

- `119: #[path = "task_lifecycle_tests.rs"]`
- `120: mod task_lifecycle_tests;`

### crates/vw-agent/src/gateway/api/handlers/task_plan_run.rs

- `218: #[path = "task_plan_run_tests.rs"]`
- `219: mod task_plan_run_tests;`

### crates/vw-agent/src/gateway/api/handlers/task_pool.rs

- `6: use std::sync::LazyLock;`
- `23: static TASK_WORKTREE_POOL_SERVICE: LazyLock<TaskWorktreePoolService> =`
- `24: LazyLock::new(TaskWorktreePoolService::new);`
- `166: #[path = "task_pool_tests.rs"]`
- `167: mod task_pool_tests;`

### crates/vw-agent/src/gateway/api/handlers/tools.rs

- `56: #[path = "tools_tests.rs"]`
- `57: mod tools_tests;`

### crates/vw-agent/src/gateway/api/handlers/tools_tests.rs

- `38: config: Arc::new(parking_lot::Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/api/handlers/usage/mod.rs

- `2: mod read;`
- `3: mod scan;`
- `4: mod events;`
- `5: mod prices;`

### crates/vw-agent/src/gateway/api/handlers/usage/prices.rs

- `2: use std::{collections::BTreeMap, path::PathBuf, sync::{Arc, Mutex}};`
- `142: u::models_sync::ModelsSync::new(Arc::new(Mutex::new(connection()?)),o.scope,o.target_host).map_err(unavailable)`
- `153: let result_slot=Arc::new(Mutex::new(None));`

### crates/vw-agent/src/gateway/api/handlers/usage/scan.rs

- `2: use std::{collections::HashMap, path::PathBuf, sync::{Mutex, OnceLock}};`
- `16: pub(super) struct Job { pub(super) owner: ScanOwner, pub(super) operation: Operation, pub(super) result: std::sync::Arc<Mutex<Option<Value>>> }`
- `17: static JOBS: OnceLock<Mutex<HashMap<String, Job>>> = OnceLock::new();`
- `18: pub(super) fn jobs() -> &'static Mutex<HashMap<String, Job>> { JOBS.get_or_init(Default::default) }`

### crates/vw-agent/src/gateway/api/handlers/voice.rs

- `22: #[path = "voice_requests.rs"]`
- `23: mod requests;`
- `24: #[path = "voice_upload.rs"]`
- `25: mod upload;`
- `211: #[path = "voice_tests.rs"]`
- `212: mod tests;`
- `215: #[path = "voice_realtime_http_tests.rs"]`
- `216: mod realtime_http_tests;`

### crates/vw-agent/src/gateway/api/handlers/voice_handoff.rs

- `142: use std::sync::{Arc, Mutex};`
- `212: let outcome = Arc::new(Mutex::new(Outcome::default()));`
- `246: #[path = "voice_handoff_tests.rs"]`
- `247: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/voice_realtime.rs

- `18: sync::{Arc, Mutex},`
- `64: pending: Mutex<HashMap<String, Reservation>>,`
- `69: Self { pending: Mutex::new(HashMap::new()), capacity: Arc::new(Semaphore::new(4)) }`
- `530: #[path = "voice_realtime_tests.rs"]`
- `531: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/voice_realtime_tests.rs

- `68: let upstream = Arc::new(tokio::sync::Mutex::new(Some(upstream)));`
- `118: let tx = Arc::new(Mutex::new(Some(tx)));`
- `280: let done = Arc::new(Mutex::new(Some(done)));`

### crates/vw-agent/src/gateway/api/handlers/voice_requests.rs

- `4: sync::{Arc, Mutex},`
- `27: Mutex<HashMap<Owner, HashMap<String, Record>>>,`
- `33: Self(Mutex::new(HashMap::new()), Arc::new(tokio::sync::Semaphore::new(4)))`
- `132: #[path = "voice_requests_tests.rs"]`
- `133: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/voice_settings.rs

- `163: #[path = "voice_settings_tests.rs"]`
- `164: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/voice_upload.rs

- `52: #[path = "voice_upload_tests.rs"]`
- `53: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/workflow.rs

- `1151: #[path = "workflow_tests.rs"]`
- `1152: mod workflow_tests;`

### crates/vw-agent/src/gateway/api/handlers/workflow_tests.rs

- `39: use parking_lot::Mutex;`
- `1159: config: Arc::new(Mutex::new(Config::default())),`
- `1184: calls: Mutex<Vec<(String, String)>>,`
- `1248: calls: Mutex::new(vec![]),`
- `1264: let yaml = include_str!("../../../../testdata/workflow/talking_head.yaml");`

### crates/vw-agent/src/gateway/api/handlers/workspace_memory.rs

- `4: use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};`
- `17: jobs: Arc<Mutex<HashMap<String, Job>>>,`
- `24: result: Arc<Mutex<Option<Vec<DailyMemorySearchResult>>>>,`
- `106: let result = Arc::new(Mutex::new(None));`
- `126: fn owned(state: &State, tenant: TenantScope, headers: &HeaderMap, scope: &Scope, id: &str) -> Result<(Operation, Arc<Mutex<Option<Vec<DailyMemorySearchResult>>>>), StatusCode> {`
- `147: #[path = "workspace_memory_tests.rs"]`
- `148: mod tests;`

### crates/vw-agent/src/gateway/api/handlers/worktree_management.rs

- `124: #[path = "worktree_management_tests.rs"]`
- `125: mod tests;`

### crates/vw-agent/src/gateway/api/integrations.rs

- `663: #[path = "integrations_tests.rs"]`
- `664: mod integrations_tests;`

### crates/vw-agent/src/gateway/api/mod.rs

- `59: pub mod auth;`
- `64: pub mod handlers;`
- `69: pub mod integrations;`
- `74: pub mod secrets;`
- `79: pub mod types;`
- `110: #[path = "tests.rs"]`
- `111: mod tests;`

### crates/vw-agent/src/gateway/api/secrets.rs

- `611: #[path = "secrets_tests.rs"]`
- `612: mod secrets_tests;`

### crates/vw-agent/src/gateway/api/tests.rs

- `7: mod tests {`

### crates/vw-agent/src/gateway/api/types.rs

- `516: #[path = "types_tests.rs"]`
- `517: mod types_tests;`

### crates/vw-agent/src/gateway/approval_state.rs

- `7: use std::sync::{Arc, LazyLock, Mutex};`
- `18: static APPROVAL_MANAGERS: LazyLock<Mutex<HashMap<ApprovalKey, Arc<ApprovalManager>>>> =`
- `19: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `71: #[path = "approval_state_tests.rs"]`
- `72: mod approval_state_tests;`

### crates/vw-agent/src/gateway/approval_state_tests.rs

- `9: static APPROVAL_STATE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/gateway/bootstrap.rs

- `21: use parking_lot::Mutex;`
- `126: let config_state = Arc::new(Mutex::new(config.clone()));`
- `271: channels: Arc::new(Mutex::new(channels)),`
- `975: #[path = "bootstrap_tests.rs"]`
- `976: mod bootstrap_tests;`

### crates/vw-agent/src/gateway/bootstrap_tests.rs

- `34: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/channel_reload.rs

- `175: #[path = "channel_reload_tests.rs"]`
- `176: mod tests;`

### crates/vw-agent/src/gateway/channel_runtime.rs

- `5: use std::sync::{Arc, Mutex};`
- `29: let project_state = Arc::new(Mutex::new(ChannelProjectState::default()));`
- `52: provider_cache: Arc::new(Mutex::new(HashMap::from([(`
- `66: non_cli_excluded_tools: Arc::new(Mutex::new(`

### crates/vw-agent/src/gateway/chat.rs

- `90: let created = Arc::new(tokio::sync::Mutex::new(engine));`
- `147: Arc::new(tokio::sync::Mutex::new(forked_engine)),`
- `404: #[path = "chat_tests.rs"]`
- `405: mod chat_tests;`

### crates/vw-agent/src/gateway/chat_tests.rs

- `41: responses: std::sync::Mutex<VecDeque<ChatResponse>>,`
- `46: Self { responses: std::sync::Mutex::new(responses.into()) }`
- `109: messages: std::sync::Mutex<Vec<ChatMessage>>,`
- `203: config: Arc::new(parking_lot::Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/deepagents_window_tests.rs

- `57: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/error.rs

- `90: #[path = "error_tests.rs"]`
- `91: mod error_tests;`

### crates/vw-agent/src/gateway/handlers.rs

- `566: #[path = "handlers_tests.rs"]`
- `567: mod handlers_tests;`

### crates/vw-agent/src/gateway/handlers_tests.rs

- `20: use parking_lot::Mutex;`
- `27: sent: Mutex<Vec<SendMessage>>,`
- `107: config: Arc::new(Mutex::new(Config::default())),`
- `259: let channel = RecordingChannel { sent: Mutex::new(Vec::new()), fail_send: true };`
- `1107: let replies = Arc::new(tokio::sync::Mutex::new(Vec::<SendMessage>::new()));`

### crates/vw-agent/src/gateway/health.rs

- `180: #[path = "health_tests.rs"]`
- `181: mod health_tests;`

### crates/vw-agent/src/gateway/health_tests.rs

- `14: use parking_lot::Mutex;`
- `37: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/instance.rs

- `139: #[path = "instance_tests.rs"]`
- `140: mod instance_tests;`

### crates/vw-agent/src/gateway/limits.rs

- `13: //! - 线程安全：使用 Mutex 保护共享状态，支持多线程并发访问`
- `23: use parking_lot::Mutex;`
- `65: /// 内部使用 Mutex 保护请求记录，支持多线程并发访问。`
- `91: pub(crate) requests: Mutex<(HashMap<String, Vec<Instant>>, Instant)>,`
- `123: requests: Mutex::new((HashMap::new(), Instant::now())),`
- `354: /// 内部使用 Mutex 保护键存储，支持多线程并发访问。`
- `377: pub(crate) keys: Mutex<HashMap<String, Instant>>,`
- `403: Self { ttl, max_keys: max_keys.max(1), keys: Mutex::new(HashMap::new()) }`
- `471: #[path = "limits_tests.rs"]`
- `472: mod limits_tests;`

### crates/vw-agent/src/gateway/middleware.rs

- `267: #[path = "middleware_tests.rs"]`
- `268: mod middleware_tests;`

### crates/vw-agent/src/gateway/mod.rs

- `14: mod agent;`
- `15: pub(crate) mod agent_apps;`
- `16: pub mod agent_apps_diagnostic_executors;`
- `17: pub mod agent_apps_diagnostic_jobs;`
- `19: pub(crate) mod agent_apps_failover_runtime;`
- `21: mod agent_apps_failover_runtime_tests;`
- `23: mod agent_apps_failover_timeouts_tests;`
- `25: pub mod agent_apps_high_availability_service;`
- `27: mod agent_apps_high_availability_service_tests;`
- `28: pub mod agent_apps_high_availability_store;`
- `30: mod agent_apps_high_availability_store_tests;`
- `31: pub mod agent_apps_skills_store;`
- `33: mod agent_apps_skills_store_tests;`
- `34: pub mod api;`
- `35: pub(crate) mod approval_state;`
- `36: mod bootstrap;`
- `37: mod channel_reload;`
- `38: mod channel_runtime;`
- `39: mod chat;`
- `40: mod error;`
- `41: mod handlers;`
- `42: mod health;`
- `43: mod instance;`
- `44: mod limits;`
- `45: mod middleware;`
- `47: mod mod_tests;`
- `48: mod node_control;`
- `49: mod openai_compat;`
- `50: mod options;`
- `52: mod pairing;`
- `53: mod router;`
- `56: mod runtime;`
- `57: pub(crate) mod settings_store;`
- `59: mod settings_store_tests;`
- `60: pub mod sse;`
- `61: mod state;`
- `63: mod tests;`
- `64: mod types;`
- `65: mod util;`
- `66: mod webhook;`
- `67: mod webhook_ingress;`
- `68: pub mod ws;`
- `167: mod recovery_window_tests;`
- `170: pub(crate) mod aggregation_resync;`
- `173: pub(crate) mod aggregation_desktop_record;`
- `176: pub(crate) mod aggregation_lifecycle;`
- `189: mod deepagents_window_tests;`

### crates/vw-agent/src/gateway/mod_tests.rs

- `18: use parking_lot::Mutex;`
- `46: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/node_control.rs

- `189: #[path = "node_control_tests.rs"]`
- `190: mod node_control_tests;`

### crates/vw-agent/src/gateway/node_control_tests.rs

- `482: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/openai_compat/mod.rs

- `569: #[path = "tests.rs"]`
- `570: mod tests;`
- `573: #[path = "mod_tests.rs"]`
- `574: mod mod_tests;`

### crates/vw-agent/src/gateway/openai_compat/mod_tests.rs

- `16: use parking_lot::Mutex;`
- `42: chat_calls: Mutex<Vec<RecordedChatCall>>,`
- `43: stream_calls: Mutex<Vec<RecordedChatCall>>,`
- `52: chat_calls: Mutex::new(Vec::new()),`
- `53: stream_calls: Mutex::new(Vec::new()),`
- `62: chat_calls: Mutex::new(Vec::new()),`
- `63: stream_calls: Mutex::new(Vec::new()),`
- `72: chat_calls: Mutex::new(Vec::new()),`
- `73: stream_calls: Mutex::new(Vec::new()),`
- `145: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/openai_compat/tests.rs

- `16: mod tests {`

### crates/vw-agent/src/gateway/options.rs

- `75: #[path = "options_tests.rs"]`
- `76: mod options_tests;`

### crates/vw-agent/src/gateway/pairing.rs

- `31: use parking_lot::Mutex;`
- `230: /// - config: 共享的应用配置对象（使用 Arc<Mutex> 包装以保证线程安全）`
- `252: /// 由于 parking_lot::Mutex 的 guard 不是 Send，我们需要克隆配置对象`
- `264: /// let config = Arc::new(Mutex::new(Config::load().await?));`
- `271: config: Arc<Mutex<Config>>,`
- `299: #[path = "pairing_tests.rs"]`
- `300: mod pairing_tests;`

### crates/vw-agent/src/gateway/pairing_tests.rs

- `76: config: Arc::new(Mutex::new(config)),`
- `247: let shared = Arc::new(Mutex::new(config));`
- `269: let error = persist_pairing_tokens(Arc::new(Mutex::new(bad_config)), &guard).await.unwrap_err();`

### crates/vw-agent/src/gateway/recovery_window_tests.rs

- `12: sync::{Arc, Mutex},`
- `18: calls: Mutex<HashMap<String, usize>>,`
- `110: config: Arc::new(parking_lot::Mutex::new(config)),`

### crates/vw-agent/src/gateway/router.rs

- `251: #[path = "router_tests.rs"]`
- `252: mod router_tests;`

### crates/vw-agent/src/gateway/runtime.rs

- `123: #[path = "runtime_tests.rs"]`
- `124: mod runtime_tests;`

### crates/vw-agent/src/gateway/settings_store/postgres.rs

- `357: #[path = "postgres_tests.rs"]`
- `358: mod postgres_tests;`

### crates/vw-agent/src/gateway/settings_store.rs

- `14: mod postgres;`

### crates/vw-agent/src/gateway/sse.rs

- `329: #[path = "sse_tests.rs"]`
- `330: mod sse_tests;`

### crates/vw-agent/src/gateway/sse_tests.rs

- `21: state: Arc<parking_lot::Mutex<TestObserverState>>,`
- `76: config: Arc::new(parking_lot::Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/state.rs

- `29: use parking_lot::Mutex;`
- `34: pub(crate) type SharedQueryEngine = Arc<tokio::sync::Mutex<QueryEngine>>;`
- `36: Arc<tokio::sync::Mutex<HashMap<(TenantScope, String), SharedQueryEngine>>>;`
- `47: /// - 可变状态（如 config）使用 Mutex 保护`
- `54: ///     config: Arc::new(Mutex::new(config)),`
- `68: pub config: Arc<Mutex<Config>>,`
- `98: pub(crate) channels: Arc<Mutex<GatewayChannels>>,`
- `126: #[path = "state_tests.rs"]`
- `127: mod state_tests;`
- `136: Mutex<HashMap<vw_api_transport::TenantScope, Arc<crate::channels::ChannelRuntimeContext>>>,`

### crates/vw-agent/src/gateway/tests/agent.rs

- `50: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/tests/auth.rs

- `59: config: Arc::new(Mutex::new(Config::default())),`
- `114: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/tests/metrics.rs

- `17: config: Arc::new(Mutex::new(Config::default())),`
- `70: config: Arc::new(Mutex::new(Config::default())),`
- `109: config: Arc::new(Mutex::new(Config::default())),`
- `149: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/tests/mod.rs

- `51: use parking_lot::Mutex;`
- `277: /// 已存储键的列表，使用 Mutex 保护以支持并发访问`
- `278: keys: Mutex<Vec<String>>,`
- `425: mod agent;`
- `426: mod allowlist;`
- `427: mod auth;`
- `428: mod basics;`
- `429: mod client_key;`
- `430: mod idempotency;`
- `431: mod memory_keys;`
- `432: mod metrics;`
- `433: mod nextcloud;`
- `434: mod node_control;`
- `435: mod pairing;`
- `436: mod qq;`
- `437: mod rate_limiter;`
- `438: mod sanitize;`
- `439: mod state;`
- `440: mod webhook;`
- `441: mod whatsapp_signature;`

### crates/vw-agent/src/gateway/tests/nextcloud.rs

- `79: config: Arc::new(Mutex::new(Config::default())),`
- `97: channels: std::sync::Arc::new(parking_lot::Mutex::new(`
- `170: config: Arc::new(Mutex::new(Config::default())),`
- `188: channels: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/tests/node_control.rs

- `52: config: Arc::new(Mutex::new(Config::default())),`
- `129: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/gateway/tests/pairing.rs

- `27: let shared_config = Arc::new(Mutex::new(config));`
- `51: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/tests/qq.rs

- `57: config: Arc::new(Mutex::new(Config::default())),`
- `75: channels: std::sync::Arc::new(parking_lot::Mutex::new(`
- `153: config: Arc::new(Mutex::new(Config::default())),`
- `171: channels: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/tests/rate_limiter.rs

- `287: /// - 验证 Mutex 保护的共享状态在并发下的正确性`

### crates/vw-agent/src/gateway/tests/webhook.rs

- `40: config: Arc::new(Mutex::new(Config::default())),`
- `120: config: Arc::new(Mutex::new(Config::default())),`
- `183: config: Arc::new(Mutex::new(Config::default())),`
- `301: config: Arc::new(Mutex::new(Config::default())),`
- `319: channels: std::sync::Arc::new(parking_lot::Mutex::new(`
- `380: config: Arc::new(Mutex::new(Config::default())),`
- `398: channels: std::sync::Arc::new(parking_lot::Mutex::new(`
- `462: config: Arc::new(Mutex::new(Config::default())),`
- `480: channels: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/types.rs

- `112: #[path = "types_tests.rs"]`
- `113: mod types_tests;`

### crates/vw-agent/src/gateway/util.rs

- `358: #[path = "util_tests.rs"]`
- `359: mod util_tests;`

### crates/vw-agent/src/gateway/webhook.rs

- `80: #[path = "webhook_tests.rs"]`
- `81: mod webhook_tests;`

### crates/vw-agent/src/gateway/webhook_ingress/mod.rs

- `516: #[path = "tests.rs"]`
- `517: mod tests;`

### crates/vw-agent/src/gateway/webhook_ingress/tests.rs

- `17: use parking_lot::Mutex;`
- `118: entries: Mutex<Vec<(String, String, MemoryCategory)>>,`
- `123: events: Mutex<Vec<ObserverEvent>>,`
- `124: metrics: Mutex<Vec<ObserverMetric>>,`
- `211: config: Arc::new(parking_lot::Mutex::new(Config::default())),`
- `229: channels: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/webhook_tests.rs

- `73: config: Arc::new(parking_lot::Mutex::new(Config::default())),`
- `91: channels: std::sync::Arc::new(parking_lot::Mutex::new(`

### crates/vw-agent/src/gateway/ws/mod.rs

- `689: #[path = "mod_tests.rs"]`
- `690: mod mod_tests;`
- `693: #[path = "tests.rs"]`
- `694: mod tests;`

### crates/vw-agent/src/gateway/ws/mod_tests.rs

- `16: use parking_lot::Mutex;`
- `26: responses: Mutex<VecDeque<ChatResponse>>,`
- `31: Self { responses: Mutex::new(responses.into()) }`
- `128: config: Arc::new(Mutex::new(Config::default())),`

### crates/vw-agent/src/gateway/ws/tests.rs

- `14: mod tests {`

### crates/vw-agent/src/global_database.rs

- `5: #[path = "global_database_tests.rs"]`
- `6: mod global_database_tests;`

### crates/vw-agent/src/health/mod.rs

- `29: use std::sync::Mutex;`
- `30: use std::sync::OnceLock;`
- `95: /// 使用 Mutex 保证线程安全。`
- `102: /// 使用 Mutex<BTreeMap> 保证并发安全，`
- `104: components: Mutex<BTreeMap<String, ComponentHealth>>,`
- `109: /// 使用 OnceLock 实现线程安全的延迟初始化，`
- `111: static REGISTRY: OnceLock<HealthRegistry> = OnceLock::new();`
- `112: static RUNTIME_BUILD_ID: OnceLock<String> = OnceLock::new();`
- `145: components: Mutex::new(BTreeMap::new()),`
- `345: mod tests;`
- `348: #[path = "mod_tests.rs"]`
- `349: mod mod_tests;`

### crates/vw-agent/src/heartbeat/engine/mod.rs

- `318: #[path = "tests.rs"]`
- `319: mod tests;`
- `322: #[path = "mod_tests.rs"]`
- `323: mod mod_tests;`

### crates/vw-agent/src/heartbeat/engine/tests.rs

- `7: mod tests {`

### crates/vw-agent/src/heartbeat/mod.rs

- `38: pub mod engine;`
- `51: #[path = "tests.rs"]`
- `52: mod tests;`

### crates/vw-agent/src/heartbeat/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/hooks/builtin/command_logger/mod.rs

- `24: use std::sync::{Arc, Mutex};`
- `45: entries: Arc<Mutex<VecDeque<CommandAuditEntry>>>,`
- `52: entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.max(1)))),`
- `220: #[path = "tests.rs"]`
- `221: mod tests;`
- `224: #[path = "mod_tests.rs"]`
- `225: mod mod_tests;`

### crates/vw-agent/src/hooks/builtin/command_logger/tests.rs

- `16: mod tests {`

### crates/vw-agent/src/hooks/builtin/mod.rs

- `35: pub mod command_logger;`

### crates/vw-agent/src/hooks/mod.rs

- `47: pub mod builtin;`
- `55: mod runner;`
- `58: mod runtime;`
- `65: mod traits;`
- `92: mod tests;`
- `95: #[path = "runtime_tests.rs"]`
- `96: mod runtime_tests;`

### crates/vw-agent/src/hooks/runner/mod.rs

- `602: #[path = "tests.rs"]`
- `603: mod tests;`

### crates/vw-agent/src/hooks/runner/tests.rs

- `12: mod tests {`
- `18: use std::sync::Mutex;`
- `153: events: Arc<Mutex<Vec<String>>>,`
- `161: events: Arc<Mutex<Vec<String>>>,`
- `180: fn shared_events() -> Arc<Mutex<Vec<String>>> {`
- `181: Arc::new(Mutex::new(Vec::new()))`
- `184: fn events_snapshot(events: &Arc<Mutex<Vec<String>>>) -> Vec<String> {`

### crates/vw-agent/src/hooks/runtime.rs

- `5: use std::sync::{Arc, RwLock};`
- `22: runner: Arc<RwLock<Option<Arc<HookRunner>>>>,`
- `31: Ok(Self { runner: Arc::new(RwLock::new(candidate.runner)), audit_store })`

### crates/vw-agent/src/hooks/traits/mod.rs

- `491: #[path = "tests.rs"]`
- `492: mod tests;`
- `495: #[path = "mod_tests.rs"]`
- `496: mod mod_tests;`

### crates/vw-agent/src/hooks/traits/tests.rs

- `20: mod tests {`

### crates/vw-agent/src/identity/load.rs

- `77: #[path = "load_tests.rs"]`
- `78: mod load_tests;`

### crates/vw-agent/src/identity/mod.rs

- `31: mod load;`
- `45: #[path = "tests.rs"]`
- `46: mod tests;`

### crates/vw-agent/src/identity/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/installation/mod.rs

- `16: use std::sync::LazyLock;`
- `18: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `24: pub mod event {`
- `567: #[path = "mod_tests.rs"]`
- `568: mod mod_tests;`
- `570: #[path = "tests.rs"]`
- `571: mod tests;`

### crates/vw-agent/src/integrations/mod.rs

- `5: pub mod registry;`
- `340: #[path = "mod_tests.rs"]`
- `341: mod mod_tests;`
- `343: #[path = "tests.rs"]`
- `344: mod tests;`

### crates/vw-agent/src/integrations/registry.rs

- `766: #[path = "registry_tests.rs"]`
- `767: mod tests;`

### crates/vw-agent/src/knowledge/chunker/patterns.rs

- `39: #[path = "patterns_tests.rs"]`
- `40: mod tests;`

### crates/vw-agent/src/knowledge/chunker/profile.rs

- `71: #[path = "profile_tests.rs"]`
- `72: mod profile_tests;`

### crates/vw-agent/src/knowledge/chunker.rs

- `7: mod patterns;`
- `8: mod profile;`
- `415: #[path = "chunker_tests.rs"]`
- `416: mod chunker_tests;`

### crates/vw-agent/src/knowledge/fusion.rs

- `209: #[path = "fusion_tests.rs"]`
- `210: mod fusion_tests;`

### crates/vw-agent/src/knowledge/mod.rs

- `6: mod chunker;`
- `7: mod fusion;`
- `9: pub(crate) mod import;`
- `10: mod store;`
- `16: #[path = "tests.rs"]`
- `17: mod tests;`
- `20: #[path = "import_tests.rs"]`
- `21: mod import_tests;`

### crates/vw-agent/src/knowledge/store/postgres/cache.rs

- `94: #[path = "cache_tests.rs"]`
- `95: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/chunks.rs

- `426: #[path = "chunks_tests.rs"]`
- `427: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/datasets.rs

- `196: #[path = "datasets_tests.rs"]`
- `197: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/documents.rs

- `277: #[path = "documents_tests.rs"]`
- `278: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/indexing.rs

- `518: #[path = "indexing_tests.rs"]`
- `519: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/mod.rs

- `3: mod cache;`
- `4: mod chunks;`
- `5: mod datasets;`
- `6: mod documents;`
- `7: mod indexing;`
- `8: mod retrieval;`
- `9: mod schema;`
- `30: #[path = "mod_tests.rs"]`
- `31: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/retrieval.rs

- `517: #[path = "retrieval_tests.rs"]`
- `518: mod tests;`

### crates/vw-agent/src/knowledge/store/postgres/schema.rs

- `218: #[path = "schema_tests.rs"]`
- `219: mod tests;`

### crates/vw-agent/src/knowledge/store.rs

- `26: use tokio::sync::Mutex;`
- `39: mod postgres;`
- `198: last_embedding_request_at: Arc<Mutex<Option<Instant>>>,`
- `293: last_embedding_request_at: Arc::new(Mutex::new(None)),`
- `3923: #[path = "store_tests.rs"]`
- `3924: mod store_tests;`

### crates/vw-agent/src/lib.rs

- `157: mod cli_commands;`
- `159: #[path = "cli_commands_tests.rs"]`
- `160: mod cli_commands_tests;`
- `163: pub mod acp;`
- `164: pub mod agent;`
- `165: pub mod agent_apps;`
- `168: pub(crate) mod app;`
- `171: pub mod approval;`
- `172: pub mod auth;`
- `175: pub mod bus;`
- `178: pub mod channels;`
- `179: pub mod computer_control;`
- `182: pub mod config;`
- `185: pub mod command;`
- `188: pub mod cron;`
- `191: pub mod daemon;`
- `194: pub mod device_sync;`
- `197: pub mod doctor;`
- `200: pub mod env;`
- `203: pub mod file;`
- `206: pub mod flag;`
- `210: pub mod format;`
- `214: pub mod gateway;`
- `224: pub mod global_database;`
- `227: pub(crate) mod health;`
- `230: pub(crate) mod heartbeat;`
- `233: pub mod hooks;`
- `236: pub mod identity;`
- `239: pub mod id;`
- `242: pub mod installation;`
- `245: pub mod integrations;`
- `248: pub mod memory;`
- `252: pub mod knowledge;`
- `256: pub mod mcp;`
- `259: pub(crate) mod migration;`
- `262: pub(crate) mod multimodal;`
- `265: pub mod observability;`
- `268: pub mod patch;`
- `271: pub mod permission;`
- `274: pub mod plugins;`
- `278: pub(crate) mod git_repository;`
- `281: mod package_boundaries;`
- `284: pub mod project;`
- `288: pub mod model_relay;`
- `289: pub mod provider;`
- `292: pub mod providers;`
- `296: pub mod pty;`
- `299: pub mod question;`
- `302: pub mod runtime;`
- `305: pub mod scheduler;`
- `309: pub mod agent_scheduler;`
- `312: pub mod security;`
- `315: pub mod service;`
- `319: pub(crate) mod shiju;`
- `322: pub mod session;`
- `325: pub mod shell;`
- `328: pub mod skill;`
- `331: pub mod skillforge;`
- `334: pub mod skills;`
- `337: pub mod snapshot;`
- `340: pub mod storage;`
- `343: pub mod task_worktree_pool;`
- `346: pub mod task_lifecycle;`
- `349: pub mod task_plan_run;`
- `352: pub mod test_capabilities;`
- `355: pub(crate) mod test_support {`
- `356: use std::sync::{Mutex, OnceLock};`
- `358: pub(crate) fn global_env_lock() -> &'static Mutex<()> {`
- `359: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `360: LOCK.get_or_init(|| Mutex::new(()))`
- `363: pub(crate) fn global_runtime_proxy_lock() -> &'static Mutex<()> {`
- `364: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `365: LOCK.get_or_init(|| Mutex::new(()))`
- `368: pub(crate) fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {`
- `369: static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`
- `375: pub mod tools;`
- `379: pub(crate) mod tunnel;`
- `382: pub mod update;`
- `385: pub mod util;`
- `388: pub mod worktree;`
- `392: pub mod workflow;`
- `407: #[path = "tests.rs"]`
- `408: mod tests;`
- `413: pub mod app_providers;`
- `416: pub mod app_routing;`
- `418: mod model_catalog;`
- `420: pub(crate) mod usage_bridge;`

### crates/vw-agent/src/mcp/mod.rs

- `285: #[path = "tests.rs"]`
- `286: mod tests;`

### crates/vw-agent/src/memory/chunker/mod.rs

- `6: mod reexport_tests;`

### crates/vw-agent/src/memory/cli/mod.rs

- `570: #[path = "tests.rs"]`
- `571: mod tests;`

### crates/vw-agent/src/memory/cli/tests.rs

- `9: use std::sync::Mutex;`
- `341: entries: Mutex<Vec<MemoryEntry>>,`
- `342: forgotten: Mutex<Vec<String>>,`
- `351: Self { entries: Mutex::new(entries), forgotten: Mutex::new(Vec::new()), ..Self::default() }`

### crates/vw-agent/src/memory/embeddings/alibaba.rs

- `103: #[path = "alibaba_tests.rs"]`
- `104: mod alibaba_tests;`

### crates/vw-agent/src/memory/embeddings/mod.rs

- `23: mod alibaba;`
- `410: #[path = "tests.rs"]`
- `411: mod tests;`

### crates/vw-agent/src/memory/hygiene/mod.rs

- `14: mod imp {`
- `724: mod tests {`

### crates/vw-agent/src/memory/hygiene/mod_tests.rs

- `4: mod additional_hygiene_tests {`

### crates/vw-agent/src/memory/mod.rs

- `41: pub mod backend;`
- `42: pub mod chunker;`
- `43: pub mod cli;`
- `44: pub mod embeddings;`
- `45: pub mod hygiene;`
- `47: pub mod lucid;`
- `49: pub mod mariadb;`
- `50: pub mod markdown;`
- `51: pub mod none;`
- `52: pub(crate) mod paths;`
- `54: pub mod postgres;`
- `55: pub mod qdrant;`
- `56: pub mod response_cache;`
- `58: pub mod snapshot;`
- `60: pub mod sqlite;`
- `61: pub mod traits;`
- `62: pub mod vector;`
- `914: #[path = "tests.rs"]`
- `915: mod tests;`
- `918: #[path = "mod_tests.rs"]`
- `919: mod mod_tests;`

### crates/vw-agent/src/memory/mod_tests.rs

- `6: use std::sync::{Mutex, MutexGuard};`
- `9: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/memory/qdrant/mod.rs

- `930: #[path = "tests.rs"]`
- `931: mod tests;`

### crates/vw-agent/src/memory/qdrant/tests.rs

- `28: use tokio::sync::Mutex;`
- `111: requests: Arc<Mutex<Vec<RecordedRequest>>>,`
- `123: let responses = Arc::new(Mutex::new(VecDeque::from(responses)));`
- `124: let requests = Arc::new(Mutex::new(Vec::new()));`
- `145: responses: Arc<Mutex<VecDeque<MockResponse>>>,`
- `146: requests: Arc<Mutex<Vec<RecordedRequest>>>,`

### crates/vw-agent/src/memory/tests.rs

- `9: mod tests {`

### crates/vw-agent/src/memory/vector/mod.rs

- `6: mod reexport_tests;`

### crates/vw-agent/src/migration/mod.rs

- `914: mod tests;`
- `917: #[path = "mod_tests.rs"]`
- `918: mod mod_tests;`

### crates/vw-agent/src/migration/tests/mod.rs

- `15: mod backup;`
- `16: mod migration;`
- `17: mod parse;`
- `18: mod sqlite;`
- `19: mod workspace;`
- `166: /// 使用 OnceLock 实现懒初始化的静态互斥锁，`
- `169: use std::sync::{Mutex, OnceLock};`
- `170: // 使用 OnceLock 确保互斥锁只初始化一次`
- `171: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `173: LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()`

### crates/vw-agent/src/mod.rs

- `41: pub mod agent;`
- `46: pub mod approval;`
- `51: pub mod auth;`
- `56: pub mod bus;`
- `66: pub mod channels;`
- `71: pub mod command;`
- `81: pub mod config;`
- `86: pub mod cron;`
- `93: pub mod daemon;`
- `100: pub mod doctor;`
- `105: pub mod env;`
- `114: pub mod file;`
- `119: pub mod flag;`
- `126: pub mod format;`
- `138: pub mod gateway;`
- `143: pub mod global;`
- `147: pub mod global_database;`
- `156: pub mod goals;`
- `161: pub mod health;`
- `166: pub mod heartbeat;`
- `171: pub mod hooks;`
- `180: pub mod id;`
- `185: pub mod identity;`
- `190: pub mod installation;`
- `195: pub mod integrations;`
- `206: pub mod mcp;`
- `212: pub mod memory;`
- `218: pub mod knowledge;`
- `223: pub mod multimodal;`
- `233: pub mod observability;`
- `242: pub mod patch;`
- `247: pub mod permission;`
- `252: pub mod plugins;`
- `257: pub mod project;`
- `267: pub mod provider;`
- `273: pub mod providers;`
- `284: pub mod pty;`
- `289: pub mod question;`
- `300: pub mod runtime;`
- `305: pub mod scheduler;`
- `311: pub mod security;`
- `318: pub mod service;`
- `327: pub mod session;`
- `332: pub mod shell;`
- `341: pub mod skill;`
- `346: pub mod skillforge;`
- `352: pub mod skills;`
- `361: pub mod snapshot;`
- `366: pub mod storage;`
- `377: pub mod tools;`
- `388: pub mod tunnel;`
- `394: pub mod update;`
- `403: pub mod util;`
- `408: pub mod worktree;`
- `444: #[path = "tests.rs"]`
- `445: mod tests;`

### crates/vw-agent/src/model_relay/chat_tests.rs

- `45: let received = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/gemini_shadow.rs

- `19: pub(crate) struct Store(parking_lot::Mutex<Vec<Receipt>>);`

### crates/vw-agent/src/model_relay/gemini_tests.rs

- `75: let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/messages_request_http_tests.rs

- `44: let received = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/messages_request_tests.rs

- `3: #[path = "messages_request_core_tests.rs"]`
- `4: mod request_core_tests;`

### crates/vw-agent/src/model_relay/messages_stream.rs

- `15: #[path = "messages_stream_tests.rs"]`
- `16: mod tests;`

### crates/vw-agent/src/model_relay/messages_tests.rs

- `187: mod http {`
- `236: let received = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/messages_thinking_http_tests.rs

- `44: let received = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/mod.rs

- `2: mod binding;`
- `3: mod server;`
- `4: pub(crate) mod upstream;`
- `9: mod binding_tests;`
- `11: mod server_tests;`
- `13: mod chat;`
- `14: pub(crate) mod generation;`
- `15: mod protocol_bridge;`
- `16: mod protocol_bridge_stream;`
- `17: pub(crate) mod routing;`
- `18: mod transport;`
- `22: mod chat_tests;`
- `24: mod transport_tests;`
- `26: pub(crate) mod messages;`
- `28: mod messages_request_http_tests;`
- `30: mod messages_request_tests;`
- `31: pub(crate) mod messages_stream;`
- `33: mod messages_tests;`
- `34: pub(crate) mod messages_thinking;`
- `35: mod messages_thinking_config;`
- `37: mod messages_thinking_core_tests;`
- `39: mod messages_thinking_http_tests;`
- `41: mod messages_thinking_tests;`
- `44: mod responses_tests;`
- `46: mod responses;`
- `47: mod responses_chat_request;`
- `49: mod responses_chat_request_tests;`
- `50: pub(crate) mod responses_compaction;`
- `52: mod responses_compaction_tests;`
- `53: mod responses_stream;`
- `56: mod responses_http_tests;`
- `59: mod gemini_tests;`
- `61: mod gemini;`
- `62: mod gemini_stream;`
- `64: mod kiro;`
- `66: pub(crate) mod stream_observer;`
- `69: mod sse_framing_tests;`
- `71: pub(crate) mod sse_framing;`
- `73: pub(crate) mod native_chat;`
- `75: mod native_chat_tests;`
- `78: mod native_messages_tests;`
- `80: pub(crate) mod native_messages;`
- `82: pub(crate) mod native_responses;`
- `84: mod native_responses_tests;`
- `86: pub(crate) mod native_gemini;`
- `88: mod native_gemini_tests;`
- `90: pub(crate) mod messages_chat_request;`
- `92: mod messages_chat_request_tests;`
- `94: pub(crate) mod messages_chat_result;`
- `96: mod messages_chat_result_tests;`
- `99: mod messages_chat_stream_tests;`
- `101: pub(crate) mod messages_gemini_request;`
- `103: mod messages_gemini_request_tests;`
- `105: pub(crate) mod messages_responses_request;`
- `107: mod messages_responses_request_tests;`
- `109: pub(crate) mod messages_responses_result;`
- `111: mod messages_responses_result_tests;`
- `113: pub(crate) mod messages_gemini_result;`
- `115: mod messages_gemini_result_tests;`
- `117: pub(crate) mod messages_gemini_stream;`
- `119: mod messages_gemini_stream_tests;`
- `121: pub(crate) mod messages_responses_stream;`
- `123: mod messages_responses_stream_tests;`
- `125: pub(crate) mod gemini_shadow;`
- `127: mod gemini_shadow_tests;`
- `129: pub(crate) mod hosted_search;`
- `131: mod hosted_search_tests;`
- `133: pub(crate) mod search_citations;`
- `135: mod search_citations_tests;`
- `137: mod responses_chat_result;`
- `139: mod responses_chat_result_tests;`
- `141: mod responses_chat_stream;`
- `143: pub(crate) mod responses_history;`
- `145: mod responses_history_tests;`
- `147: mod tool_namespace;`
- `149: mod responses_anthropic_request;`
- `151: mod responses_anthropic_request_tests;`
- `153: mod responses_anthropic_result;`
- `155: mod responses_anthropic_result_tests;`
- `157: mod responses_anthropic_stream;`
- `159: mod responses_anthropic_stream_tests;`
- `161: pub(crate) mod reasoning_bridge;`
- `164: mod reasoning_bridge_tests;`
- `166: mod inline_think;`
- `168: pub(crate) mod late_arguments;`
- `170: mod late_arguments_tests;`
- `172: pub(crate) mod xai_responses;`
- `174: mod xai_responses_tests;`
- `176: pub(crate) mod signature_rectify;`
- `178: mod signature_rectify_tests;`
- `180: mod tool_media;`
- `181: mod tool_media_request;`
- `184: pub(crate) mod copilot_transport;`
- `186: mod copilot_transport_tests;`
- `188: pub(crate) mod codex_oauth_transport;`
- `190: pub(crate) mod quota_gate;`
- `192: pub(crate) mod xai_oauth_transport;`
- `194: pub(crate) mod gemini_oauth_transport;`
- `196: pub(crate) mod compaction;`
- `198: pub(crate) mod compaction_v2;`
- `200: mod compaction_v2_tests;`
- `202: pub(crate) mod models;`
- `204: pub(crate) mod gemini_metadata;`
- `206: mod gemini_metadata_tests;`
- `208: pub(crate) mod alpha_search;`
- `210: pub(crate) mod image_generation;`
- `212: pub(crate) mod image_edit;`
- `214: mod image_edit_tests;`
- `216: pub(crate) mod forced_sse;`

### crates/vw-agent/src/model_relay/protocol_bridge.rs

- `7: #[path = "protocol_bridge_response.rs"]`
- `8: mod decode;`
- `9: #[path = "protocol_bridge_request.rs"]`
- `10: mod encode;`
- `231: #[path = "protocol_bridge_tests.rs"]`
- `232: mod tests;`

### crates/vw-agent/src/model_relay/protocol_bridge_stream.rs

- `284: #[path = "protocol_bridge_stream_tests.rs"]`
- `285: mod tests;`

### crates/vw-agent/src/model_relay/quota_gate.rs

- `2: use std::{collections::HashMap, sync::{Mutex, OnceLock}, time::{Duration, Instant}};`
- `11: fn readings() -> &'static Mutex<HashMap<Account, Reading>> {`
- `12: static CACHE: OnceLock<Mutex<HashMap<Account, Reading>>> = OnceLock::new();`
- `13: CACHE.get_or_init(Mutex::default)`

### crates/vw-agent/src/model_relay/responses.rs

- `8: #[path = "responses_chat_tools.rs"]`
- `9: mod chat_tools;`
- `11: #[path = "responses_codex_chat.rs"]`
- `12: mod codex_chat;`

### crates/vw-agent/src/model_relay/responses_anthropic_stream.rs

- `23: signed: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,`
- `204: let signed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/responses_history.rs

- `24: pub(crate) struct Store(parking_lot::Mutex<VecDeque<Entry>>);`

### crates/vw-agent/src/model_relay/responses_http_tests.rs

- `165: let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/responses_stream.rs

- `21: signed_blocks: Option<Arc<std::sync::Mutex<Vec<Value>>>>,`
- `657: blocks: Arc<std::sync::Mutex<Vec<Value>>>,`

### crates/vw-agent/src/model_relay/search_citations.rs

- `4: #[path = "search_citations_markdown.rs"]`
- `5: mod markdown;`

### crates/vw-agent/src/model_relay/server.rs

- `49: rate: parking_lot::Mutex<(Instant, u32)>,`
- `81: rate: parking_lot::Mutex::new((Instant::now(), 0)),`
- `437: static STATUS: std::sync::LazyLock<parking_lot::Mutex<Vec<serde_json::Value>>> =`
- `438: std::sync::LazyLock::new(|| parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/model_relay/server_tests.rs

- `42: cancellation: parking_lot::Mutex::new(None),`
- `110: cancellation: parking_lot::Mutex<Option<CancellationToken>>,`
- `190: cancellation: parking_lot::Mutex::new(None),`
- `422: cancellation: parking_lot::Mutex::new(None),`

### crates/vw-agent/src/model_relay/stream_observer.rs

- `461: #[path = "stream_observer_tests.rs"]`
- `462: pub(crate) mod tests;`

### crates/vw-agent/src/multimodal/analysis.rs

- `15: use parking_lot::Mutex;`
- `23: use std::sync::LazyLock;`
- `42: static ANALYSIS_INFLIGHT: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =`
- `43: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `107: .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))`

### crates/vw-agent/src/multimodal/mod.rs

- `13: pub mod analysis;`
- `14: pub mod assets;`
- `15: pub mod context;`
- `16: mod image_compression;`
- `17: pub mod ocr;`
- `796: #[path = "analysis_tests.rs"]`
- `797: mod analysis_tests;`
- `800: #[path = "assets_tests.rs"]`
- `801: mod assets_tests;`
- `804: #[path = "context_tests.rs"]`
- `805: mod context_tests;`
- `808: #[path = "image_compression_tests.rs"]`
- `809: mod image_compression_tests;`
- `812: #[path = "ocr_tests.rs"]`
- `813: mod ocr_tests;`
- `816: #[path = "tests.rs"]`
- `817: mod tests;`

### crates/vw-agent/src/multimodal/ocr_tests.rs

- `13: use std::sync::Mutex;`
- `26: requests: Mutex<Vec<(String, String)>>,`
- `35: requests: Mutex::new(Vec::new()),`
- `158: requests: Mutex::new(Vec::new()),`
- `198: requests: Mutex::new(Vec::new()),`
- `216: requests: Mutex::new(Vec::new()),`

### crates/vw-agent/src/multimodal/tests.rs

- `7: mod tests {`

### crates/vw-agent/src/observability/mod.rs

- `53: pub mod runtime_trace;`
- `54: pub mod traits;`
- `243: mod tests;`

### crates/vw-agent/src/observability/runtime_trace/mod.rs

- `14: use std::sync::{Arc, LazyLock, RwLock};`
- `81: write_lock: std::sync::Mutex<()>,`
- `86: Self { mode, max_entries: max_entries.max(1), path, write_lock: std::sync::Mutex::new(()) }`
- `162: static TRACE_LOGGER: LazyLock<RwLock<Option<Arc<RuntimeTraceLogger>>>> =`
- `163: LazyLock::new(|| RwLock::new(None));`
- `399: mod tests;`

### crates/vw-agent/src/observability/runtime_trace/tests.rs

- `7: use std::sync::{LazyLock, Mutex};`
- `9: static TRACE_LOGGER_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));`

### crates/vw-agent/src/package_boundaries/mod.rs

- `1: mod git_discovery_tests;`
- `2: mod project_storage_paths_tests;`
- `3: mod secret_store_tests;`
- `4: mod gateway_pairing_tests;`
- `5: mod process_sandbox_tests;`
- `6: mod memory_types_tests;`
- `7: mod memory_markdown_tests;`
- `8: mod memory_sqlite_tests;`
- `10: mod memory_postgres_tests;`
- `12: mod memory_mariadb_tests;`
- `14: mod memory_lucid_tests;`
- `16: mod memory_response_cache_tests;`
- `18: mod memory_snapshot_tests;`

### crates/vw-agent/src/permission/mod.rs

- `19: pub mod next;`

### crates/vw-agent/src/permission/next.rs

- `53: use std::sync::LazyLock;`
- `54: use std::sync::Mutex;`
- `232: static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));`
- `492: #[path = "next_tests.rs"]`
- `493: mod next_tests;`

### crates/vw-agent/src/permission/next_tests.rs

- `6: use std::sync::{LazyLock, Mutex};`
- `8: static STATE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));`

### crates/vw-agent/src/plugins/audit.rs

- `133: #[path = "audit_tests.rs"]`
- `134: mod audit_tests;`

### crates/vw-agent/src/plugins/compat/hook_async.rs

- `2: use std::sync::Mutex;`
- `6: tasks: Mutex<JoinSet<()>>,`
- `13: tasks: Mutex::default(),`

### crates/vw-agent/src/plugins/compat/hook_session.rs

- `8: sync::{Arc, OnceLock},`
- `12: fn sessions() -> &'static tokio::sync::Mutex<Sessions> {`
- `13: static SESSIONS: OnceLock<tokio::sync::Mutex<Sessions>> = OnceLock::new();`

### crates/vw-agent/src/plugins/compat/mcp_app_execution.rs

- `8: sync::{Mutex, OnceLock},`
- `19: Mutex<`
- `26: fn bindings() -> &'static Mutex<HashMap<String, Binding>> {`
- `27: static BINDINGS: OnceLock<Mutex<HashMap<String, Binding>>> = OnceLock::new();`
- `65: prompts: std::sync::Arc::new(Mutex::new(prompt_rx)),`

### crates/vw-agent/src/plugins/compat/mcp_connection_sessions.rs

- `7: sync::{Arc, Mutex, OnceLock},`
- `10: fn sessions() -> &'static Mutex<HashMap<String, Arc<McpSessionConnections>>> {`
- `11: static SESSIONS: OnceLock<Mutex<HashMap<String, Arc<McpSessionConnections>>>> = OnceLock::new();`

### crates/vw-agent/src/plugins/compat/mcp_connections.rs

- `13: sync::{Arc, Mutex, OnceLock},`
- `16: use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};`
- `27: catalogs: Mutex<std::sync::Weak<super::mcp_notifications::McpCatalogs>>,`
- `57: pub(super) state: Mutex<State>,`
- `66: static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();`
- `138: state: Mutex::new(State::default()),`
- `342: catalogs: Mutex::new(std::sync::Weak::new()),`

### crates/vw-agent/src/plugins/compat/mcp_notifications.rs

- `7: Arc, Mutex,`
- `19: listed: Mutex<Option<Listed<T>>>,`
- `20: refresh: tokio::sync::Mutex<()>,`
- `26: listed: Mutex::new(None),`
- `27: refresh: tokio::sync::Mutex::new(()),`

### crates/vw-agent/src/plugins/compat/mcp_oauth_tokens.rs

- `9: sync::{Arc, Mutex, OnceLock},`
- `79: fn locks() -> &'static Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> {`
- `80: static LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();`
- `81: LOCKS.get_or_init(Mutex::default)`
- `296: fn cancellations() -> &'static Mutex<HashMap<String, tokio_util::sync::CancellationToken>> {`
- `297: static TOKENS: OnceLock<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>> =`
- `298: OnceLock::new();`
- `299: TOKENS.get_or_init(Mutex::default)`

### crates/vw-agent/src/plugins/compat/mcp_resources.rs

- `15: use std::sync::Mutex;`
- `21: Mutex<Option<(Value, Value)>>,`
- `22: Mutex<std::collections::HashMap<String, Value>>,`
- `23: Mutex<std::collections::HashMap<String, Value>>,`

### crates/vw-agent/src/plugins/compat/mod.rs

- `2: pub(crate) mod hook_mcp;`
- `3: pub(crate) mod hook_subagent;`
- `5: mod t43_tests;`
- `7: pub(super) mod agents;`
- `8: pub(super) mod commands;`
- `9: pub(super) mod discovery;`
- `10: pub(super) mod identity;`
- `11: pub(super) mod legacy_manifest;`
- `12: pub(super) mod mcp_auth;`
- `13: pub(super) mod mcp_capabilities;`
- `14: pub(crate) mod mcp_connection_sessions;`
- `15: pub(crate) mod mcp_connection_settings;`
- `16: pub(crate) mod mcp_connections;`
- `17: pub(super) mod mcp_content;`
- `18: pub(crate) mod mcp_elicitation;`
- `19: pub(super) mod mcp_environment;`
- `20: pub(super) mod mcp_legacy;`
- `21: pub(super) mod mcp_portable;`
- `22: pub(super) mod mcp_portable_overlay;`
- `23: pub(super) mod mcp_public_auth;`
- `24: pub(super) mod onboarding;`
- `25: pub(super) mod path_audit;`
- `26: pub(super) mod portable_manifest;`
- `27: pub(super) mod skill_frontmatter;`
- `28: pub(super) mod skill_invocation;`
- `29: pub(super) mod skill_roots;`
- `30: pub(super) mod transaction_install;`
- `33: #[path = "t02_tests.rs"]`
- `34: mod t02_tests;`
- `37: #[path = "t03_tests.rs"]`
- `38: mod t03_tests;`
- `41: #[path = "t04_tests.rs"]`
- `42: mod t04_tests;`
- `45: #[path = "t05_tests.rs"]`
- `46: mod t05_tests;`
- `49: #[path = "t06_tests.rs"]`
- `50: mod t06_tests;`
- `53: #[path = "t07_tests.rs"]`
- `54: mod t07_tests;`
- `57: #[path = "t08_tests.rs"]`
- `58: mod t08_tests;`
- `61: #[path = "t09_tests.rs"]`
- `62: mod t09_tests;`
- `65: #[path = "t10_tests.rs"]`
- `66: mod t10_tests;`
- `69: #[path = "t11_tests.rs"]`
- `70: mod t11_tests;`
- `73: #[path = "t12_tests.rs"]`
- `74: mod t12_tests;`
- `77: #[path = "t13_tests.rs"]`
- `78: mod t13_tests;`
- `81: #[path = "t14_tests.rs"]`
- `82: mod t14_tests;`
- `85: #[path = "t15_tests.rs"]`
- `86: mod t15_tests;`
- `89: #[path = "t16_tests.rs"]`
- `90: mod t16_tests;`
- `93: #[path = "t17_tests.rs"]`
- `94: mod t17_tests;`
- `97: #[path = "t17_public_tests.rs"]`
- `98: mod t17_public_tests;`
- `101: #[path = "t18_tests.rs"]`
- `102: mod t18_tests;`
- `105: mod t19_tests;`
- `107: pub(crate) mod mcp_catalog;`
- `108: mod mcp_catalog_refresh;`
- `109: pub(crate) mod mcp_notifications;`
- `110: pub(crate) mod mcp_resources;`
- `113: mod t20_tests;`
- `116: mod t21_tests;`
- `119: mod t22_tests;`
- `122: mod t23_tests;`
- `125: mod t24_tests;`
- `127: pub mod mcp_oauth_discovery;`
- `129: mod t26_tests;`
- `131: pub mod mcp_oauth_flow;`
- `133: mod t27_flow_tests;`
- `135: mod t27_tests;`
- `137: pub mod mcp_oauth_tokens;`
- `139: mod t28_tests;`
- `141: pub(crate) mod mcp_apps;`
- `143: mod t29_tests;`
- `145: pub(crate) mod mcp_app_execution;`
- `147: #[path = "t30_tests.rs"]`
- `148: mod t30_tests;`
- `150: pub(crate) mod hook_manifest;`
- `152: mod t32_tests;`
- `154: pub(crate) mod hook_matcher;`
- `156: mod t33_tests;`
- `159: mod t34_tests;`
- `161: pub(crate) mod hook_command;`
- `163: pub(crate) mod hook_async;`
- `165: mod t35_tests;`
- `167: pub(crate) mod hook_pre_tool;`
- `169: mod t36_tests;`
- `171: pub(crate) mod hook_permission;`
- `173: mod t37_tests;`
- `175: pub(crate) mod hook_post_tool;`
- `177: mod t38_tests;`
- `179: pub(crate) mod hook_prompt;`
- `181: mod t40_tests;`
- `183: pub(crate) mod hook_session;`
- `186: mod t39_tests;`
- `188: pub(crate) mod hook_model;`
- `190: pub(crate) mod t41_tests;`
- `192: pub(crate) mod hook_compact;`
- `194: pub(crate) mod t42_tests;`
- `196: pub(crate) mod hook_stop;`
- `198: pub(crate) mod t44_tests;`
- `201: mod t45_tests;`
- `203: mod t46_tests;`
- `205: pub(crate) mod hook_trust_content;`
- `207: pub(crate) mod marketplace;`
- `209: mod t47_tests;`
- `211: pub(crate) mod git_source;`
- `213: mod t48_tests;`
- `215: pub(crate) mod npm_source;`
- `217: mod t49_tests;`
- `219: pub(crate) mod npm_archive;`
- `221: mod t50_tests;`
- `224: mod t51_tests;`
- `227: mod t52_tests;`
- `228: pub(crate) mod upgrade;`
- `230: pub(crate) mod installed_state;`
- `233: mod t53_tests;`
- `236: mod t54_tests;`
- `238: pub(crate) mod dependencies;`
- `240: mod t58_tests;`
- `243: mod plugin_window_fixture;`
- `245: pub(crate) mod session_snapshot;`
- `247: mod t57_tests;`
- `250: mod t59_tests;`
- `253: mod t60_tests;`
- `255: pub(crate) mod host_files;`
- `258: mod t61_tests;`
- `261: mod t62_tests;`
- `264: mod t64_tests;`
- `267: mod platform_tests;`
- `270: mod closure_tests;`
- `272: mod t66_tests;`

### crates/vw-agent/src/plugins/compat/plugin_window_fixture.rs

- `7: use parking_lot::Mutex;`
- `28: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/plugins/compat/skill_invocation.rs

- `6: use std::sync::OnceLock;`
- `49: static LINK: OnceLock<Regex> = OnceLock::new();`

### crates/vw-agent/src/plugins/compat/t03_tests.rs

- `14: serde_json::from_str(include_str!("../../../testdata/plugin_compat/t03/legacy.json"))`

### crates/vw-agent/src/plugins/compat/t04_tests.rs

- `17: serde_json::from_str(include_str!("../../../testdata/plugin_compat/t04/portable.json")).unwrap()`

### crates/vw-agent/src/plugins/compat/t05_tests.rs

- `49: serde_json::from_str(include_str!("../../../testdata/plugin_compat/t05/root_aliases.json"))`

### crates/vw-agent/src/plugins/compat/t06_tests.rs

- `13: const PACKAGE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t06/plugin.json");`

### crates/vw-agent/src/plugins/compat/t07_tests.rs

- `8: const NATIVE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/native.json");`
- `9: const LEGACY: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/legacy.json");`
- `10: const PORTABLE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/portable.json");`

### crates/vw-agent/src/plugins/compat/t08_tests.rs

- `15: const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t08/plugin.json");`
- `16: const PAYLOAD: &[u8] = include_bytes!("../../../testdata/plugin_compat/t08/payload.txt");`

### crates/vw-agent/src/plugins/compat/t09_tests.rs

- `8: const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t09/plugin.json");`
- `9: const ALPHA: &str = include_str!("../../../testdata/plugin_compat/t09/alpha/SKILL.md");`
- `10: const BETA: &str = include_str!("../../../testdata/plugin_compat/t09/beta/SKILL.md");`

### crates/vw-agent/src/plugins/compat/t10_tests.rs

- `13: const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t10/plugin.json");`
- `14: const SKILL: &[u8] = include_bytes!("../../../testdata/plugin_compat/t10/SKILL.md");`

### crates/vw-agent/src/plugins/compat/t11_tests.rs

- `15: const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t11/plugin.json");`
- `16: const COMMAND: &[u8] = include_bytes!("../../../testdata/plugin_compat/t11/review.md");`
- `17: const EXPECTED: &str = include_str!("../../../testdata/plugin_compat/t11/expected-skill.md");`
- `113: include_bytes!("../../../testdata/plugin_compat/t11/missing.md"),`

### crates/vw-agent/src/plugins/compat/t12_tests.rs

- `45: include_bytes!("../../../testdata/plugin_compat/t12/SKILL.md"),`
- `49: include_bytes!("../../../testdata/plugin_compat/t12/openai.yaml"),`
- `295: include_bytes!("../../../testdata/plugin_compat/t12/openai.yaml"),`

### crates/vw-agent/src/plugins/compat/t13_tests.rs

- `49: include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),`
- `258: include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),`
- `290: String::from_utf8(include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md").to_vec())`
- `346: include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),`

### crates/vw-agent/src/plugins/compat/t14_tests.rs

- `49: include_bytes!("../../../testdata/plugin_compat/t14/wrapped.json"),`

### crates/vw-agent/src/plugins/compat/t15_tests.rs

- `43: fs::write(&path, include_bytes!("../../../testdata/plugin_compat/t15/mcp.json")).unwrap();`

### crates/vw-agent/src/plugins/compat/t16_tests.rs

- `24: include_bytes!("../../../testdata/plugin_compat/t16/server.py"),`
- `141: include_bytes!("../../../testdata/plugin_compat/t16/server.py"),`
- `199: include_str!("../../../testdata/plugin_compat/t16/server.py")`

### crates/vw-agent/src/plugins/compat/t18_tests.rs

- `11: fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/server.py")).unwrap();`
- `248: fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/http_server.py"))`

### crates/vw-agent/src/plugins/compat/t20_tests.rs

- `159: peer: Arc<tokio::sync::Mutex<Option<Peer<RoleServer>>>>,`
- `169: let peer = Arc::new(tokio::sync::Mutex::new(None));`

### crates/vw-agent/src/plugins/compat/t21_tests.rs

- `9: serde_json::from_slice(include_bytes!("../../../testdata/plugin_compat/t21/result.json"))`
- `20: fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t21/server.py")).unwrap();`
- `23: include_bytes!("../../../testdata/plugin_compat/t21/result.json"),`
- `239: fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t21/http_server.py"))`

### crates/vw-agent/src/plugins/compat/t23_tests.rs

- `9: std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t23/server.py"))`
- `127: include_bytes!("../../../testdata/plugin_compat/t23/http_server.py"),`

### crates/vw-agent/src/plugins/compat/t24_tests.rs

- `71: let events = Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/plugins/compat/t29_tests.rs

- `73: std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t29/server.py"))`
- `77: include_bytes!("../../../testdata/plugin_compat/t29/resource.json"),`

### crates/vw-agent/src/plugins/compat/t30_tests.rs

- `12: include_bytes!("../../../testdata/plugin_compat/t30/server.py"),`
- `17: include_bytes!("../../../testdata/plugin_compat/t29/resource.json"),`

### crates/vw-agent/src/plugins/compat/t32_tests.rs

- `15: serde_json::from_str(include_str!("../../../testdata/plugin_compat/t32/hooks.json")).unwrap()`

### crates/vw-agent/src/plugins/compat/t33_tests.rs

- `10: serde_json::from_str(include_str!("../../../testdata/plugin_compat/t33/hooks.json")).unwrap()`

### crates/vw-agent/src/plugins/compat/t34_tests.rs

- `154: "hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":include_str!("../../../testdata/plugin_compat/t34/capture.sh")}]}]}`

### crates/vw-agent/src/plugins/compat/t54_tests.rs

- `10: use parking_lot::Mutex;`
- `32: config: Arc::new(Mutex::new(config)),`

### crates/vw-agent/src/plugins/compat/t57_tests.rs

- `274: include_bytes!("../../../testdata/plugin_compat/t18/server.py"),`

### crates/vw-agent/src/plugins/hook_handler.rs

- `26: std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Weak<super::AsyncHooks>>>>,`

### crates/vw-agent/src/plugins/hook_handler_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `103: events: Arc<Mutex<Vec<tracing::Level>>>,`

### crates/vw-agent/src/plugins/loader.rs

- `257: #[path = "loader_tests.rs"]`
- `258: mod loader_tests;`

### crates/vw-agent/src/plugins/mod.rs

- `6: mod audit;`
- `7: mod cli;`
- `8: mod hook_handler;`
- `9: mod hook_trust;`
- `10: mod hooks;`
- `11: mod install;`
- `12: mod loader;`
- `13: mod manifest;`
- `14: mod mcp;`
- `15: mod skills;`
- `16: mod types;`
- `18: mod compat;`
- `69: #[path = "manifest_tests.rs"]`
- `70: mod manifest_tests;`
- `73: #[path = "mcp_tests.rs"]`
- `74: mod mcp_tests;`
- `77: #[path = "hooks_tests.rs"]`
- `78: mod hooks_tests;`
- `81: #[path = "hook_trust_tests.rs"]`
- `82: mod hook_trust_tests;`
- `85: #[path = "hook_handler_tests.rs"]`
- `86: mod hook_handler_tests;`
- `89: #[path = "install_tests.rs"]`
- `90: mod install_tests;`
- `93: #[path = "cli_tests.rs"]`
- `94: mod cli_tests;`
- `97: #[path = "skills_tests.rs"]`
- `98: mod skills_tests;`

### crates/vw-agent/src/plugins/skills_tests.rs

- `10: use std::sync::{Mutex, MutexGuard, OnceLock};`
- `12: fn env_lock() -> &'static Mutex<()> {`
- `13: static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `14: ENV_LOCK.get_or_init(|| Mutex::new(()))`

### crates/vw-agent/src/project/core.rs

- `418: #[path = "core_tests.rs"]`
- `419: mod core_tests;`

### crates/vw-agent/src/project/icon_discovery.rs

- `108: #[path = "icon_discovery_tests.rs"]`
- `109: mod icon_discovery_tests;`

### crates/vw-agent/src/project/instance.rs

- `16: use std::sync::LazyLock;`
- `17: use tokio::sync::Mutex;`
- `36: static CACHE: LazyLock<Mutex<HashMap<String, Arc<Context>>>> =`
- `37: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `193: #[path = "instance_tests.rs"]`
- `194: mod instance_tests;`

### crates/vw-agent/src/project/migration.rs

- `104: #[path = "migration_tests.rs"]`
- `105: mod migration_tests;`

### crates/vw-agent/src/project/mod.rs

- `19: use std::sync::{LazyLock, Mutex};`
- `30: mod core;`
- `31: mod icon_discovery;`
- `32: pub mod instance;`
- `34: mod migration;`
- `35: pub mod state;`
- `37: pub mod store;`
- `38: pub mod vcs;`
- `41: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `47: static BOOTSTRAPPED_WORKTREES: LazyLock<Mutex<HashSet<String>>> =`
- `48: LazyLock::new(|| Mutex::new(HashSet::new()));`
- `51: static RUNTIME_STORE: LazyLock<Mutex<Option<store::ProjectStore>>> =`
- `52: LazyLock::new(|| Mutex::new(None));`
- `58: pub mod event {`
- `215: #[path = "tests.rs"]`
- `216: mod tests;`

### crates/vw-agent/src/project/state.rs

- `10: use std::sync::LazyLock;`
- `11: use std::sync::{Arc, Mutex};`
- `27: static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));`
- `120: #[path = "state_tests.rs"]`
- `121: mod state_tests;`

### crates/vw-agent/src/project/store/postgres.rs

- `374: #[path = "postgres_tests.rs"]`
- `375: mod postgres_tests;`

### crates/vw-agent/src/project/store.rs

- `21: mod postgres;`
- `520: #[path = "store_tests.rs"]`
- `521: mod store_tests;`

### crates/vw-agent/src/project/vcs.rs

- `16: use std::sync::LazyLock;`
- `17: use std::sync::{Arc, Mutex};`
- `19: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `25: pub mod event {`
- `53: pub async fn init(worktree: PathBuf) -> Arc<Mutex<Info>> {`
- `57: let info = Arc::new(Mutex::new(Info { branch: current_branch(&wt) }));`
- `101: #[path = "vcs_tests.rs"]`
- `102: mod vcs_tests;`

### crates/vw-agent/src/project/vcs_tests.rs

- `9: use std::sync::{Arc, Mutex};`
- `168: let seen = Arc::new(Mutex::new(Vec::new()));`
- `208: let seen = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/provider/auth.rs

- `307: #[path = "auth_tests.rs"]`
- `308: mod auth_tests;`

### crates/vw-agent/src/provider/error.rs

- `212: #[path = "error_tests.rs"]`
- `213: mod error_tests;`

### crates/vw-agent/src/provider/mod.rs

- `20: pub mod auth;`
- `22: pub mod error {`
- `26: pub mod models {`
- `30: pub(crate) mod model_catalog;`
- `31: pub mod provider;`
- `33: pub mod transform;`
- `36: pub mod voice;`
- `39: #[path = "tests.rs"]`
- `40: mod tests;`
- `43: mod model_catalog_tests;`

### crates/vw-agent/src/provider/models.rs

- `23: #[path = "models_tests.rs"]`
- `24: mod models_tests;`

### crates/vw-agent/src/provider/provider.rs

- `39: use std::sync::{Arc, LazyLock};`
- `58: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `85: type CatalogCell = tokio::sync::Mutex<Option<Arc<CatalogSnapshot>>>;`
- `794: #[path = "provider_tests.rs"]`
- `795: mod provider_tests;`

### crates/vw-agent/src/provider/provider_tests.rs

- `260: let fixture: Value = serde_json::from_str(include_str!(`
- `294: let fixture: Value = serde_json::from_str(include_str!(`

### crates/vw-agent/src/provider/transform.rs

- `411: #[path = "transform_tests.rs"]`
- `412: mod transform_tests;`

### crates/vw-agent/src/provider/voice/mod.rs

- `2: pub mod realtime;`
- `3: pub(crate) mod realtime_handoff;`
- `5: mod realtime_tests;`
- `6: pub mod transcription;`
- `30: mod transcription_tests;`
- `117: mod realtime_preview_tests;`

### crates/vw-agent/src/provider/voice/realtime.rs

- `516: #[path = "realtime_events.rs"]`
- `517: mod events;`

### crates/vw-agent/src/provider/voice/realtime_handoff.rs

- `218: #[path = "realtime_handoff_tests.rs"]`
- `219: mod tests;`

### crates/vw-agent/src/providers/mod.rs

- `34: pub mod session_llm;`
- `35: pub mod traits;`
- `682: #[path = "tests.rs"]`
- `683: mod tests;`
- `686: mod native_search_capability_tests;`

### crates/vw-agent/src/providers/session_llm.rs

- `16: #[path = "session_llm_output.rs"]`
- `17: mod output;`
- `491: #[path = "session_llm_tests.rs"]`
- `492: mod session_llm_tests;`

### crates/vw-agent/src/providers/traits/mod.rs

- `456: #[path = "tests.rs"]`
- `457: mod tests;`

### crates/vw-agent/src/providers/traits/tests.rs

- `20: mod tests {`

### crates/vw-agent/src/pty/mod.rs

- `7: pub mod pty;`

### crates/vw-agent/src/pty/pty.rs

- `35: use std::sync::LazyLock;`
- `36: use std::sync::{Arc, Mutex};`
- `41: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `57: pub mod event {`
- `290: info: Mutex<Info>,`
- `292: master: Mutex<Box<dyn MasterPty + Send>>,`
- `294: writer: Mutex<Box<dyn Write + Send>>,`
- `296: child: Mutex<Box<dyn portable_pty::Child + Send>>,`
- `298: buffer: Mutex<BufferState>,`
- `302: type Sessions = Arc<Mutex<HashMap<String, Arc<Session>>>>;`
- `317: || async { Arc::new(Mutex::new(HashMap::new())) },`
- `514: info: Mutex::new(info.clone()),`
- `515: master: Mutex::new(master),`
- `516: writer: Mutex::new(writer),`
- `517: child: Mutex::new(child),`
- `518: buffer: Mutex::new(BufferState::new()),`
- `822: #[path = "pty_tests.rs"]`
- `823: mod pty_tests;`

### crates/vw-agent/src/question/mod.rs

- `16: use std::sync::LazyLock;`
- `17: use std::sync::Mutex;`
- `20: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `30: pub mod event {`
- `79: static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));`
- `274: #[path = "mod_tests.rs"]`
- `275: mod mod_tests;`

### crates/vw-agent/src/question/mod_tests.rs

- `5: use std::sync::{Arc, Mutex};`
- `93: let asked_events = Arc::new(Mutex::new(Vec::new()));`
- `98: let replied_events = Arc::new(Mutex::new(Vec::new()));`
- `155: let rejected_events = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/runtime/docker/mod.rs

- `310: #[path = "tests.rs"]`
- `311: mod tests;`

### crates/vw-agent/src/runtime/docker/tests.rs

- `16: mod tests {`

### crates/vw-agent/src/runtime/mod.rs

- `26: pub mod docker;`
- `27: pub mod native;`
- `28: pub mod traits;`
- `99: #[path = "tests.rs"]`
- `100: mod tests;`

### crates/vw-agent/src/runtime/native/mod.rs

- `614: mod tests;`
- `617: mod shell_tool_tests;`
- `620: mod windows_commands_tests;`

### crates/vw-agent/src/runtime/native/tests.rs

- `21: mod tests {`

### crates/vw-agent/src/runtime/tests.rs

- `22: mod tests {`

### crates/vw-agent/src/runtime/traits/mod.rs

- `177: #[path = "tests.rs"]`
- `178: mod tests;`

### crates/vw-agent/src/runtime/traits/tests.rs

- `20: mod tests {`

### crates/vw-agent/src/scheduler/mod.rs

- `54: use std::sync::LazyLock;`
- `55: use std::sync::{Mutex, OnceLock};`
- `61: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `204: static GLOBAL: LazyLock<Mutex<Entry>> = LazyLock::new(|| Mutex::new(Entry::new()));`
- `209: static INSTANCES: LazyLock<Mutex<HashMap<String, Entry>>> =`
- `210: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `214: /// 使用 OnceLock 确保运行时只创建一次。`
- `222: static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();`
- `482: #[path = "tests.rs"]`
- `483: mod tests;`

### crates/vw-agent/src/security/detect/mod.rs

- `263: #[path = "tests.rs"]`
- `264: mod tests;`

### crates/vw-agent/src/security/detect/tests.rs

- `6: mod tests {`

### crates/vw-agent/src/security/domain_matcher/mod.rs

- `6: mod reexport_tests;`

### crates/vw-agent/src/security/estop/mod.rs

- `695: #[path = "tests.rs"]`
- `696: mod tests;`

### crates/vw-agent/src/security/estop/tests.rs

- `25: mod tests {`

### crates/vw-agent/src/security/gateway_skey_store/postgres.rs

- `127: #[path = "postgres_tests.rs"]`
- `128: mod postgres_tests;`

### crates/vw-agent/src/security/gateway_skey_store.rs

- `17: mod postgres;`
- `236: #[path = "gateway_skey_store_tests.rs"]`
- `237: mod gateway_skey_store_tests;`

### crates/vw-agent/src/security/leak_detector.rs

- `4: #[path = "leak_detector_reexport_tests.rs"]`
- `5: mod reexport_tests;`

### crates/vw-agent/src/security/mod.rs

- `50: pub mod bubblewrap;`
- `53: pub mod canary_guard;`
- `56: pub mod detect;`
- `59: pub mod docker;`
- `63: pub mod domain_matcher;`
- `66: pub mod estop;`
- `69: pub mod gateway_skey_store;`
- `74: pub mod firejail;`
- `79: pub mod landlock;`
- `82: pub mod leak_detector;`
- `85: pub mod otp;`
- `88: pub mod pairing;`
- `91: pub mod policy;`
- `94: pub mod prompt_guard;`
- `97: pub mod secrets;`
- `100: pub mod semantic_guard;`
- `103: pub mod syscall_anomaly;`
- `106: pub mod traits;`
- `222: #[path = "tests.rs"]`
- `223: mod tests;`

### crates/vw-agent/src/security/otp/mod.rs

- `20: //! │  │ (OtpConfig) │  │  (Vec<u8>)  │  │ (Mutex<HashMap>)│  │`
- `44: use parking_lot::Mutex;`
- `76: /// 内部使用 Mutex 保护已缓存验证码的 HashMap，确保在多线程环境下安全使用。`
- `109: cached_codes: Mutex<HashMap<String, u64>>,`
- `168: cached_codes: Mutex::new(HashMap::new()),`
- `200: Self { config: config.clone(), secret, cached_codes: Mutex::new(HashMap::new()) };`
- `684: #[path = "tests.rs"]`
- `685: mod tests;`

### crates/vw-agent/src/security/otp/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/security/policy/action_tracker.rs

- `11: //! - **线程安全**：使用 Mutex 保证并发访问的安全性`
- `25: use parking_lot::Mutex;`
- `35: /// - actions：使用 Mutex 保护的 Vec<Instant> 集合`
- `42: /// 使用 parking_lot::Mutex 提供高性能的互斥锁实现。`
- `53: /// 使用 Mutex 包装以支持并发访问。`
- `54: actions: Mutex<Vec<Instant>>,`
- `75: Self { actions: Mutex::new(Vec::new()) }`
- `191: Self { actions: Mutex::new(actions.clone()) }`
- `209: #[path = "action_tracker_tests.rs"]`
- `210: mod action_tracker_tests;`

### crates/vw-agent/src/security/policy/allowlist.rs

- `187: #[path = "allowlist_tests.rs"]`
- `188: mod allowlist_tests;`

### crates/vw-agent/src/security/policy/mod.rs

- `7: mod action_tracker;`
- `8: pub(crate) mod allowlist;`
- `9: mod path_utils;`
- `10: mod risk;`
- `11: mod shell_lexer;`
- `12: mod shell_redirect;`
- `13: mod types;`
- `768: #[path = "tests.rs"]`
- `769: mod tests;`

### crates/vw-agent/src/security/policy/path_utils.rs

- `125: #[path = "path_utils_tests.rs"]`
- `126: mod path_utils_tests;`

### crates/vw-agent/src/security/policy/path_utils_tests.rs

- `5: use std::sync::{Mutex, OnceLock};`
- `8: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `9: LOCK.get_or_init(|| Mutex::new(())).lock().expect("HOME environment lock")`

### crates/vw-agent/src/security/policy/risk.rs

- `230: #[path = "risk_tests.rs"]`
- `231: mod risk_tests;`

### crates/vw-agent/src/security/policy/shell_lexer.rs

- `535: #[path = "shell_lexer_tests.rs"]`
- `536: mod shell_lexer_tests;`

### crates/vw-agent/src/security/policy/shell_redirect.rs

- `468: #[path = "shell_redirect_tests.rs"]`
- `469: mod shell_redirect_tests;`

### crates/vw-agent/src/security/policy/types.rs

- `58: #[path = "types_tests.rs"]`
- `59: mod types_tests;`

### crates/vw-agent/src/security/prompt_guard.rs

- `4: #[path = "prompt_guard_reexport_tests.rs"]`
- `5: mod reexport_tests;`

### crates/vw-agent/src/security/semantic_guard.rs

- `38: const BUILTIN_CORPUS_JSONL: &str = include_str!("../../assets/security/attack-corpus-v1.jsonl");`
- `837: #[path = "semantic_guard_tests.rs"]`
- `838: mod semantic_guard_tests;`

### crates/vw-agent/src/security/semantic_guard_tests.rs

- `72: point_bodies: Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,`
- `76: -> (String, Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>, tokio::task::JoinHandle<()>) {`
- `95: let point_bodies = Arc::new(tokio::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/security/tests/policy/mod.rs

- `14: mod autonomy;`
- `15: mod command_policy;`
- `16: mod config;`
- `17: mod path_policy;`
- `18: mod shell_safety;`
- `19: mod tracker;`

### crates/vw-agent/src/service/common_tests.rs

- `8: use std::sync::Mutex;`
- `20: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/service/mod.rs

- `24: mod common;`
- `25: mod linux;`
- `26: mod macos;`
- `27: mod openrc;`
- `28: mod windows;`
- `226: #[path = "mod_tests.rs"]`
- `227: mod mod_tests;`
- `229: #[path = "tests.rs"]`
- `230: mod tests;`
- `249: #[path = "common_tests.rs"]`
- `250: mod common_tests;`
- `252: #[path = "linux_tests.rs"]`
- `253: mod linux_tests;`
- `255: #[path = "macos_tests.rs"]`
- `256: mod macos_tests;`
- `258: #[path = "openrc_tests.rs"]`
- `259: mod openrc_tests;`
- `261: #[path = "windows_tests.rs"]`
- `262: mod windows_tests;`

### crates/vw-agent/src/service/openrc.rs

- `428: pub(super) mod test_support {`

### crates/vw-agent/src/service/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/session/agent_binding.rs

- `225: static FULL_ACCESS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`
- `293: #[path = "agent_binding_tests.rs"]`
- `294: mod tests;`
- `296: static INITIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/session/capability_policy_tests.rs

- `68: std::fs::write(&script, include_str!("native_defaults_mcp_fixture.py")).unwrap();`

### crates/vw-agent/src/session/child_agent/coordinator.rs

- `3: #[path = "mailbox.rs"]`
- `4: pub(crate) mod mailbox;`
- `6: #[path = "continuation.rs"]`
- `7: mod continuation;`
- `8: #[path = "delivery.rs"]`
- `9: mod delivery;`
- `14: use std::sync::{Arc, Condvar, Mutex, OnceLock};`
- `80: execution: HashMap<String, Arc<Mutex<Option<OwnedSemaphorePermit>>>>,`
- `85: state: Mutex<CoordinatorState>,`
- `97: static GLOBAL_COORDINATOR: OnceLock<ChildAgentCoordinator> = OnceLock::new();`
- `108: state: Mutex::new(CoordinatorState::default()),`
- `132: ) -> Arc<Mutex<Option<OwnedSemaphorePermit>>> {`
- `133: let permit = Arc::new(Mutex::new(Some(permit)));`
- `865: #[path = "continuation_tests.rs"]`
- `866: mod continuation_tests;`

### crates/vw-agent/src/session/child_agent/delivery.rs

- `54: #[path = "delivery_tests.rs"]`
- `55: mod tests;`

### crates/vw-agent/src/session/child_agent/entry_tests.rs

- `7: Arc, Mutex,`
- `119: let requests = Arc::new(Mutex::new(Vec::<Value>::new()));`
- `120: let root = Arc::new(Mutex::new(String::new()));`

### crates/vw-agent/src/session/child_agent/history.rs

- `60: #[path = "history_tests.rs"]`
- `61: mod tests;`

### crates/vw-agent/src/session/child_agent/mailbox.rs

- `213: #[path = "mailbox_tests.rs"]`
- `214: mod tests;`

### crates/vw-agent/src/session/child_agent.rs

- `24: pub(crate) mod coordinator;`
- `25: mod history;`
- `38: #[path = "child_agent/coordinator_tests.rs"]`
- `39: mod coordinator_tests;`
- `96: hook_continuations: std::sync::Arc<std::sync::Mutex<Vec<String>>>,`
- `510: let execute = std::sync::Arc::new(std::sync::Mutex::new(execute));`
- `762: #[path = "child_agent/entry_tests.rs"]`
- `763: mod entry_tests;`

### crates/vw-agent/src/session/child_agent_tests.rs

- `4: use std::sync::Mutex;`
- `253: let provider_request = Arc::new(Mutex::new(None));`
- `330: let observed = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/codex_alignment_tests.rs

- `124: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/compaction.rs

- `32: pub mod event {`
- `723: #[path = "compaction_tests.rs"]`
- `724: pub(crate) mod compaction_tests;`

### crates/vw-agent/src/session/compaction_checkpoint.rs

- `173: #[path = "compaction_checkpoint_tests.rs"]`
- `174: mod tests;`

### crates/vw-agent/src/session/compaction_summary.rs

- `10: #[path = "compaction_summary_input.rs"]`
- `11: mod request_input;`
- `157: #[path = "compaction_summary_tests.rs"]`
- `158: mod tests;`

### crates/vw-agent/src/session/compaction_tests.rs

- `9: static COMPACTION_STORAGE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/session/context_injection.rs

- `83: #[path = "context_injection_tests.rs"]`
- `84: mod context_injection_tests;`

### crates/vw-agent/src/session/fork_execution.rs

- `85: #[path = "fork_execution_tests.rs"]`
- `86: mod tests;`

### crates/vw-agent/src/session/instruction.rs

- `17: use std::sync::LazyLock;`
- `18: use tokio::sync::Mutex;`
- `20: #[path = "instruction_cache.rs"]`
- `21: mod instruction_cache;`
- `22: #[path = "instruction_include.rs"]`
- `23: mod instruction_include;`
- `24: #[path = "instruction_rules.rs"]`
- `25: mod instruction_rules;`
- `28: #[path = "instruction_cache_tests.rs"]`
- `29: mod instruction_cache_tests;`
- `32: #[path = "instruction_include_tests.rs"]`
- `33: mod instruction_include_tests;`
- `35: #[path = "instruction_rules_tests.rs"]`
- `36: mod instruction_rules_tests;`
- `39: #[path = "claude_protocol_parity_tests.rs"]`
- `40: mod claude_protocol_parity_tests;`
- `42: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `56: claims: Mutex<HashMap<String, HashSet<String>>>,`
- `57: cache: Mutex<instruction_cache::InstructionCache>,`
- `479: #[path = "instruction_tests.rs"]`
- `480: mod instruction_tests;`

### crates/vw-agent/src/session/instruction_cache_tests.rs

- `3: use std::sync::Mutex;`
- `78: events: Mutex<Vec<InstructionsLoaded>>,`

### crates/vw-agent/src/session/instruction_discovery.rs

- `181: #[path = "instruction_discovery_tests.rs"]`
- `182: mod instruction_discovery_tests;`

### crates/vw-agent/src/session/instruction_tests.rs

- `16: static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/session/llm/acp/approval.rs

- `5: use parking_lot::Mutex;`
- `15: slot: &'a Mutex<Option<PermissionConfirmation>>,`
- `20: pub(super) fn bind(slot: &'a Mutex<Option<PermissionConfirmation>>, session_id: &str) -> Self {`

### crates/vw-agent/src/session/llm/acp/config.rs

- `499: #[path = "config_tests.rs"]`
- `500: mod config_tests;`

### crates/vw-agent/src/session/llm/acp/content.rs

- `80: #[path = "content_tests.rs"]`
- `81: mod content_tests;`

### crates/vw-agent/src/session/llm/acp/controls.rs

- `544: #[path = "controls_tests.rs"]`
- `545: mod tests;`

### crates/vw-agent/src/session/llm/acp/fork_replay.rs

- `111: #[path = "fork_replay_tests.rs"]`
- `112: mod tests;`

### crates/vw-agent/src/session/llm/acp/lifecycle.rs

- `6: sync::{Arc, LazyLock, Weak},`
- `13: static LOCKS: LazyLock<parking_lot::Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> =`
- `14: LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));`
- `37: let mutex = Arc::new(tokio::sync::Mutex::new(()));`
- `99: #[path = "lifecycle_tests.rs"]`
- `100: mod tests;`

### crates/vw-agent/src/session/llm/acp/mod.rs

- `18: use std::sync::{Arc, LazyLock};`
- `25: use parking_lot::Mutex;`
- `40: mod approval;`
- `41: mod config;`
- `42: mod content;`
- `43: pub(crate) mod controls;`
- `44: mod fork_replay;`
- `45: pub(super) mod lifecycle;`
- `46: mod model_source;`
- `47: mod recovery;`
- `48: mod replay;`
- `49: pub(super) mod request;`
- `50: mod session;`
- `51: mod skill_mounts;`
- `52: mod updates;`
- `54: static ACP_CLIENT_CACHE: LazyLock<Mutex<HashMap<String, Arc<CachedAcpClient>>>> =`
- `55: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `62: permission_confirmation: Arc<parking_lot::Mutex<Option<acp_host_io::permissions::PermissionConfirmation>>>,`
- `66: prompt_lock: Arc<tokio::sync::Mutex<()>>,`
- `67: output_tx: Arc<Mutex<Option<mpsc::UnboundedSender<AcpJsonRpcMessage>>>>,`
- `70: worktree_use: Mutex<Option<Arc<std::fs::File>>>,`
- `173: mod tests;`
- `175: pub(crate) mod source_selection;`

### crates/vw-agent/src/session/llm/acp/model_source/acceptance_tests.rs

- `554: static STATS: std::sync::LazyLock<parking_lot::Mutex<Value>> =`
- `555: std::sync::LazyLock::new(|| parking_lot::Mutex::new(json!({})));`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/amp_acp.rs

- `83: #[path = "amp_acp_tests.rs"]`
- `84: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/auggie.rs

- `99: #[path = "auggie_tests.rs"]`
- `100: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/claude.rs

- `167: #[path = "claude_tests.rs"]`
- `168: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/codebuddy.rs

- `65: #[path = "codebuddy_tests.rs"]`
- `66: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs

- `124: #[path = "codex_tests.rs"]`
- `125: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot.rs

- `146: #[path = "copilot_tests.rs"]`
- `147: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/cursor.rs

- `95: #[path = "cursor_tests.rs"]`
- `96: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/dirac.rs

- `79: #[path = "dirac_tests.rs"]`
- `80: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/droid.rs

- `80: #[path = "droid_tests.rs"]`
- `81: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini.rs

- `179: #[path = "gemini_tests.rs"]`
- `180: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/glm_acp_agent.rs

- `78: #[path = "glm_acp_agent_tests.rs"]`
- `79: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/goose.rs

- `54: #[path = "goose_tests.rs"]`
- `55: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/grok.rs

- `117: #[path = "grok_tests.rs"]`
- `118: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/hermes.rs

- `121: #[path = "hermes_tests.rs"]`
- `122: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/hermes_tests.rs

- `204: use std::sync::{Arc, Mutex};`
- `208: let records = Arc::new(Mutex::new(Vec::<Value>::new()));`
- `211: .route("/v1/chat/completions", post(move |State(records): State<Arc<Mutex<Vec<Value>>>>, headers: HeaderMap, Json(body): Json<Value>| {`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/iflow.rs

- `98: #[path = "iflow_tests.rs"]`
- `99: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/kilocode.rs

- `176: #[path = "kilocode_tests.rs"]`
- `177: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/kimi.rs

- `140: #[path = "kimi_tests.rs"]`
- `141: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/kiro.rs

- `145: #[path = "kiro_tests.rs"]`
- `146: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs

- `3: mod amp_acp;`
- `4: mod auggie;`
- `5: mod claude;`
- `6: mod codebuddy;`
- `7: pub(in crate::session::llm::acp) mod codex;`
- `8: mod copilot;`
- `9: mod cursor;`
- `10: mod dirac;`
- `11: mod droid;`
- `12: mod gemini;`
- `13: mod glm_acp_agent;`
- `14: mod goose;`
- `15: mod grok;`
- `16: mod hermes;`
- `17: mod iflow;`
- `18: mod kilocode;`
- `19: mod kimi;`
- `20: mod kiro;`
- `21: mod openclaw;`
- `22: mod opencode;`
- `23: mod pi;`
- `24: mod qoder;`
- `25: mod qwen;`
- `26: mod sigit;`
- `27: mod snow;`
- `28: mod trae;`
- `29: mod vibe;`
- `220: #[path = "drivers_tests.rs"]`
- `221: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/openclaw.rs

- `145: #[path = "openclaw_tests.rs"]`
- `146: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode.rs

- `228: #[path = "opencode_tests.rs"]`
- `229: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/pi.rs

- `212: #[path = "pi_tests.rs"]`
- `213: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/qoder.rs

- `73: #[path = "qoder_tests.rs"]`
- `74: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/qwen.rs

- `162: #[path = "qwen_tests.rs"]`
- `163: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/sigit.rs

- `76: #[path = "sigit_tests.rs"]`
- `77: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/snow.rs

- `55: #[path = "snow_tests.rs"]`
- `56: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/trae.rs

- `62: #[path = "trae_tests.rs"]`
- `63: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/drivers/vibe.rs

- `92: #[path = "vibe_tests.rs"]`
- `93: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/gateway.rs

- `68: #[path = "gateway_tests.rs"]`
- `69: mod tests;`

### crates/vw-agent/src/session/llm/acp/model_source/mod.rs

- `2: pub(super) mod drivers;`
- `3: mod gateway;`
- `4: pub(crate) mod profile;`
- `15: static KEY: std::sync::LazyLock<ring::hmac::Key> = std::sync::LazyLock::new(|| {`
- `95: #[path = "profile_tests.rs"]`
- `96: mod tests;`
- `99: pub(super) mod acceptance_tests;`

### crates/vw-agent/src/session/llm/acp/recovery.rs

- `137: #[path = "recovery_tests.rs"]`
- `138: mod tests;`

### crates/vw-agent/src/session/llm/acp/replay.rs

- `402: #[path = "replay_tests.rs"]`
- `403: mod replay_tests;`

### crates/vw-agent/src/session/llm/acp/request.rs

- `1019: #[path = "request_tests.rs"]`
- `1020: mod request_tests;`

### crates/vw-agent/src/session/llm/acp/session.rs

- `169: Arc::new(parking_lot::Mutex::new(None::<acp_host_io::permissions::PermissionConfirmation>));`
- `172: Arc::new(parking_lot::Mutex::new(None::<mpsc::UnboundedSender<AcpJsonRpcMessage>>));`
- `243: prompt_lock: Arc::new(tokio::sync::Mutex::new(())),`
- `532: #[path = "session_tests.rs"]`
- `533: mod session_tests;`

### crates/vw-agent/src/session/llm/acp/session_tests.rs

- `7: use tokio::sync::{Mutex, watch};`
- `21: static CACHE_LOCK: Mutex<()> = Mutex::const_new(());`

### crates/vw-agent/src/session/llm/acp/skill_mounts/auggie.rs

- `6: #[path = "auggie_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/claude.rs

- `16: #[path = "claude_tests.rs"]`
- `17: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/codebuddy.rs

- `31: #[path = "codebuddy_tests.rs"]`
- `32: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/codex.rs

- `7: #[path = "codex_tests.rs"]`
- `8: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/copilot.rs

- `33: #[path = "copilot_tests.rs"]`
- `34: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/cursor.rs

- `6: #[path = "cursor_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/droid.rs

- `6: #[path = "droid_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/gemini.rs

- `29: #[path = "gemini_tests.rs"]`
- `30: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/goose.rs

- `46: #[path = "goose_tests.rs"]`
- `47: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/hermes.rs

- `29: #[path = "hermes_tests.rs"]`
- `30: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/iflow.rs

- `29: #[path = "iflow_tests.rs"]`
- `30: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/kilocode.rs

- `29: #[path = "kilocode_tests.rs"]`
- `30: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/kimi.rs

- `6: #[path = "kimi_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/kiro.rs

- `30: #[path = "kiro_tests.rs"]`
- `31: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs

- `3: mod auggie;`
- `4: mod claude;`
- `5: mod codebuddy;`
- `6: mod codex;`
- `7: mod copilot;`
- `8: mod cursor;`
- `9: mod droid;`
- `10: mod gemini;`
- `11: mod goose;`
- `12: mod hermes;`
- `13: mod iflow;`
- `14: mod kilocode;`
- `15: mod kimi;`
- `16: mod kiro;`
- `17: mod openclaw;`
- `18: mod opencode;`
- `19: mod pi;`
- `20: mod qoder;`
- `21: mod qwen;`
- `22: mod snow;`
- `23: mod trae;`
- `24: mod vibe;`
- `115: #[path = "outbound_tests.rs"]`
- `116: mod outbound_tests;`
- `119: mod acceptance_tests;`
- `169: mod lifecycle_tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/openclaw.rs

- `27: #[path = "openclaw_tests.rs"]`
- `28: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/opencode.rs

- `30: #[path = "opencode_tests.rs"]`
- `31: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/pi.rs

- `25: #[path = "pi_tests.rs"]`
- `26: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/qoder.rs

- `6: #[path = "qoder_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/qwen.rs

- `27: #[path = "qwen_tests.rs"]`
- `28: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/snow.rs

- `27: #[path = "snow_tests.rs"]`
- `28: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/trae.rs

- `6: #[path = "trae_tests.rs"]`
- `7: mod tests;`

### crates/vw-agent/src/session/llm/acp/skill_mounts/vibe.rs

- `27: #[path = "vibe_tests.rs"]`
- `28: mod tests;`

### crates/vw-agent/src/session/llm/acp/tests.rs

- `33: prompt_lock: Arc::new(tokio::sync::Mutex::new(())),`
- `34: output_tx: Arc::new(Mutex::new(output_tx)),`

### crates/vw-agent/src/session/llm/acp/updates.rs

- `472: #[path = "updates_tests.rs"]`
- `473: mod updates_tests;`

### crates/vw-agent/src/session/llm/aisdk/convert.rs

- `460: #[path = "convert_tests.rs"]`
- `461: mod convert_tests;`

### crates/vw-agent/src/session/llm/aisdk/driver.rs

- `244: #[path = "driver_tests.rs"]`
- `245: mod driver_tests;`

### crates/vw-agent/src/session/llm/aisdk/error.rs

- `215: #[path = "error_tests.rs"]`
- `216: mod error_tests;`

### crates/vw-agent/src/session/llm/aisdk/mod.rs

- `7: pub(super) mod convert;`
- `8: mod driver;`
- `9: mod error;`
- `11: mod reasoning;`
- `12: mod request;`
- `13: mod stream;`
- `14: mod util;`
- `23: mod native_search_acceptance_tests;`
- `26: mod model_catalog_tests;`

### crates/vw-agent/src/session/llm/aisdk/model_catalog_tests.rs

- `5: sync::{Arc, Mutex},`
- `65: let requests = Arc::new(Mutex::new(Vec::new()));`
- `132: State(requests): State<Arc<Mutex<Vec<Value>>>>,`

### crates/vw-agent/src/session/llm/aisdk/native_search_acceptance_tests.rs

- `4: use std::sync::{Arc, Mutex};`
- `50: requests: Arc<Mutex<Vec<Value>>>,`
- `58: let requests = Arc::new(Mutex::new(Vec::new()));`
- `420: let events = Arc::new(Mutex::new(Vec::new()));`
- `543: let card = Arc::new(Mutex::new(None));`

### crates/vw-agent/src/session/llm/aisdk/reasoning.rs

- `95: #[path = "reasoning_tests.rs"]`
- `96: mod tests;`

### crates/vw-agent/src/session/llm/aisdk/reasoning_tests.rs

- `9: let catalog: Value = serde_json::from_str(include_str!(concat!(`

### crates/vw-agent/src/session/llm/aisdk/request.rs

- `281: #[path = "request_tests.rs"]`
- `282: mod request_tests;`

### crates/vw-agent/src/session/llm/aisdk/stream.rs

- `628: #[path = "stream_tests.rs"]`
- `629: mod stream_tests;`
- `632: #[path = "truncation_tests.rs"]`
- `633: mod truncation_tests;`

### crates/vw-agent/src/session/llm/aisdk/stream_tests.rs

- `7: use std::sync::{Arc, Mutex};`
- `38: captured_options: Arc<Mutex<Option<LanguageModelOptions>>>,`
- `42: fn with_items(items: Vec<ChunkResult>) -> (Self, Arc<Mutex<Option<LanguageModelOptions>>>) {`
- `43: let captured_options = Arc::new(Mutex::new(None));`
- `60: captured_options: Arc::new(Mutex::new(None)),`
- `69: captured_options: Arc::new(Mutex::new(None)),`
- `152: ) -> (String, Arc<Mutex<Vec<Value>>>, std::thread::JoinHandle<()>) {`
- `155: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/llm/aisdk/truncation_tests.rs

- `5: use std::sync::{Arc, Mutex};`
- `21: let request = Arc::new(Mutex::new(None::<Value>));`

### crates/vw-agent/src/session/llm/aisdk/util.rs

- `539: #[path = "util_tests.rs"]`
- `540: mod util_tests;`

### crates/vw-agent/src/session/llm/logging.rs

- `8: use std::sync::LazyLock;`
- `16: pub static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `26: #[path = "logging_tests.rs"]`
- `27: mod logging_tests;`

### crates/vw-agent/src/session/llm/messages.rs

- `436: #[path = "messages_tests.rs"]`
- `437: mod messages_tests;`

### crates/vw-agent/src/session/llm/mod.rs

- `31: mod logging;`
- `32: mod messages;`
- `33: mod model_selector;`
- `35: mod model_selector_tests;`
- `36: mod options;`
- `37: mod prompt;`
- `38: mod stream;`
- `40: mod tests;`
- `41: mod types;`
- `44: mod acp;`
- `51: mod aisdk;`
- `75: pub(crate) mod recovery_test_support;`
- `83: mod native_tools_tests;`
- `86: pub mod native_tools;`

### crates/vw-agent/src/session/llm/options.rs

- `30: #[path = "options_tests.rs"]`
- `31: mod options_tests;`

### crates/vw-agent/src/session/llm/prompt.rs

- `536: let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));`
- `691: let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));`
- `876: let on_event = std::sync::Arc::new(std::sync::Mutex::new(on_event));`
- `946: /// 函数内部使用 Arc<Mutex> 在同步和异步边界之间共享状态，`
- `986: // 使用 Arc<Mutex> 共享状态来收集流式输出`
- `988: let state = std::sync::Arc::new(std::sync::Mutex::new((`
- `1095: #[path = "prompt_tests.rs"]`
- `1096: mod prompt_tests;`

### crates/vw-agent/src/session/llm/prompt_tests.rs

- `10: use std::sync::{Arc, Mutex};`
- `355: let prompt_events = Arc::new(Mutex::new(Vec::new()));`
- `369: let tool_events = Arc::new(Mutex::new(Vec::new()));`
- `392: let events = Arc::new(Mutex::new(Vec::new()));`
- `419: let events = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/llm/recovery_test_support.rs

- `61: #[path = "recovery_test_support_tests.rs"]`
- `62: mod tests;`

### crates/vw-agent/src/session/llm/stream.rs

- `834: #[path = "stream_tests.rs"]`
- `835: mod stream_tests;`
- `838: #[path = "transport_tests.rs"]`
- `839: mod transport_tests;`

### crates/vw-agent/src/session/llm/stream_tests.rs

- `10: use std::sync::{Arc, Mutex};`
- `280: let events = Arc::new(Mutex::new(Vec::new()));`
- `558: let events = Arc::new(Mutex::new(Vec::new()));`
- `672: let events = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/llm/transport_tests.rs

- `9: use std::sync::{Arc, Mutex};`
- `140: let observed = Arc::new(Mutex::new((String::new(), 0)));`

### crates/vw-agent/src/session/llm/types.rs

- `205: #[path = "types_tests.rs"]`
- `206: mod types_tests;`

### crates/vw-agent/src/session/message.rs

- `14: use std::sync::{Arc, LazyLock, Mutex, Weak};`
- `16: type HistoryLock = tokio::sync::Mutex<()>;`
- `17: static HISTORY_LOCKS: LazyLock<Mutex<HashMap<String, Weak<HistoryLock>>>> =`
- `18: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `50: pub mod event {`
- `254: #[path = "message_tests.rs"]`
- `255: mod message_tests;`

### crates/vw-agent/src/session/mod.rs

- `4: pub(crate) mod agent_binding;`
- `6: pub(crate) mod capability_policy;`
- `8: #[path = "capability_policy_tests.rs"]`
- `9: mod capability_policy_tests;`
- `11: pub mod child_agent;`
- `13: #[path = "child_agent_tests.rs"]`
- `14: mod child_agent_tests;`
- `16: mod codex_alignment_tests;`
- `18: pub mod compaction;`
- `19: pub(crate) mod compaction_checkpoint;`
- `20: pub(crate) mod compaction_summary;`
- `22: pub mod context_injection;`
- `24: pub(crate) mod fork_execution;`
- `26: pub mod instruction;`
- `28: pub mod instruction_discovery;`
- `30: pub mod llm;`
- `32: pub mod llm_raw;`
- `34: #[path = "llm_raw_tests.rs"]`
- `35: mod llm_raw_tests;`
- `37: pub mod message;`
- `39: pub mod processor;`
- `41: pub mod prompt;`
- `43: pub mod retry;`
- `45: pub mod revert;`
- `47: pub mod runtime;`
- `49: pub mod session;`
- `50: pub(crate) mod skill_checkpoint;`
- `52: pub mod status;`
- `55: pub mod store;`
- `57: #[path = "store_tests.rs"]`
- `58: mod store_tests;`
- `60: pub mod summary;`
- `62: pub mod system;`
- `64: pub(crate) mod task_session;`
- `66: #[path = "task_session_tests.rs"]`
- `67: mod task_session_tests;`
- `69: pub mod title;`
- `71: pub mod todo;`
- `73: pub mod ui_config;`
- `75: pub mod ui_store;`
- `77: pub mod ui_types;`
- `80: mod dynamic_tool_store_tests;`
- `84: pub mod dynamic_tool_store;`
- `87: mod dynamic_tool_acceptance_tests;`

### crates/vw-agent/src/session/processor/artifacts.rs

- `247: #[path = "artifacts_tests.rs"]`
- `248: mod artifacts_tests;`

### crates/vw-agent/src/session/processor/context_governance.rs

- `331: #[path = "context_governance_tests.rs"]`
- `332: mod context_governance_tests;`

### crates/vw-agent/src/session/processor/helpers.rs

- `105: #[path = "helpers_tests.rs"]`
- `106: mod helpers_tests;`

### crates/vw-agent/src/session/processor/llm_messages.rs

- `298: #[path = "llm_messages_tests.rs"]`
- `299: mod llm_messages_tests;`

### crates/vw-agent/src/session/processor/llm_runner.rs

- `783: #[path = "llm_runner_tests.rs"]`
- `784: mod llm_runner_tests;`

### crates/vw-agent/src/session/processor/llm_runner_tests.rs

- `26: static TEST_LLM_MESSAGES: std::sync::LazyLock<std::sync::Mutex<RecordedMessages>> =`
- `27: std::sync::LazyLock::new(|| std::sync::Mutex::new(Default::default()));`
- `52: static TEST_LLM_STREAMS: std::sync::LazyLock<std::sync::Mutex<VecDeque<Vec<llm::StreamEvent>>>> =`
- `53: std::sync::LazyLock::new(|| std::sync::Mutex::new(VecDeque::new()));`
- `55: static TEST_LLM_STREAM_TOOL_IDS: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<String>>>> =`
- `56: std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));`
- `58: static TEST_LLM_STREAM_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =`
- `59: std::sync::LazyLock::new(|| std::sync::Mutex::new(()));`
- `1344: static CHANNEL_CANCEL_STREAMS: std::sync::LazyLock<`
- `1345: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<ChannelCancelStream>>>,`
- `1346: > = std::sync::LazyLock::new(Default::default);`

### crates/vw-agent/src/session/processor/mod.rs

- `30: mod artifacts;`
- `31: pub(crate) mod context_governance;`
- `32: mod helpers;`
- `33: pub(crate) mod llm_messages;`
- `34: pub(crate) mod llm_runner;`
- `40: mod planning;`
- `41: mod prefetch;`
- `42: mod prompting;`
- `43: mod recovery;`
- `44: mod runner;`
- `45: mod todo_updates;`
- `46: mod todos;`
- `47: mod tools_exec;`
- `48: mod turn_policy;`
- `49: mod types;`
- `50: mod utils;`
- `77: #[path = "tests.rs"]`
- `78: mod tests;`

### crates/vw-agent/src/session/processor/planning.rs

- `326: #[path = "planning_tests.rs"]`
- `327: mod planning_tests;`

### crates/vw-agent/src/session/processor/prefetch.rs

- `373: #[path = "prefetch_tests.rs"]`
- `374: mod prefetch_tests;`

### crates/vw-agent/src/session/processor/prompting.rs

- `68: #[path = "prompting_tests.rs"]`
- `69: mod prompting_tests;`

### crates/vw-agent/src/session/processor/recovery.rs

- `225: #[path = "recovery_tests.rs"]`
- `226: mod tests;`

### crates/vw-agent/src/session/processor/runner.rs

- `2241: #[path = "runner_tests.rs"]`
- `2242: mod runner_tests;`

### crates/vw-agent/src/session/processor/runner_tests.rs

- `680: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `709: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `749: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `1744: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `2343: let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `2374: let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `2435: let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `2496: let completed = std::sync::Arc::new(std::sync::Mutex::new(None));`
- `2576: let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `2689: include_bytes!("../../../testdata/plugin_compat/t10/plugin.json"),`
- `2694: include_bytes!("../../../testdata/plugin_compat/t10/SKILL.md"),`
- `3226: #[path = "mcp_catalog_refresh_tests.rs"]`
- `3227: mod mcp_catalog_refresh_tests;`
- `3283: let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));`
- `3405: let card = std::sync::Arc::new(std::sync::Mutex::new(None));`

### crates/vw-agent/src/session/processor/tests.rs

- `19: mod tests {`

### crates/vw-agent/src/session/processor/todo_updates.rs

- `180: #[path = "todo_updates_tests.rs"]`
- `181: mod todo_updates_tests;`

### crates/vw-agent/src/session/processor/todos.rs

- `199: #[path = "todos_tests.rs"]`
- `200: mod todos_tests;`

### crates/vw-agent/src/session/processor/tools_exec.rs

- `777: let created_slot = Arc::new(std::sync::Mutex::new(None::<`
- `856: let created_slot = Arc::new(std::sync::Mutex::new(`
- `1892: #[path = "tools_exec_tests.rs"]`
- `1893: mod tools_exec_tests;`

### crates/vw-agent/src/session/processor/tools_exec_tests.rs

- `404: let created = Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/processor/turn_policy.rs

- `40: #[path = "turn_policy_tests.rs"]`
- `41: mod turn_policy_tests;`

### crates/vw-agent/src/session/processor/types.rs

- `263: #[path = "types_tests.rs"]`
- `264: mod types_tests;`

### crates/vw-agent/src/session/processor/utils/docs.rs

- `55: #[path = "docs_tests.rs"]`
- `56: mod docs_tests;`

### crates/vw-agent/src/session/processor/utils/file_links.rs

- `169: #[path = "file_links_tests.rs"]`
- `170: mod file_links_tests;`

### crates/vw-agent/src/session/processor/utils/message_format.rs

- `54: #[path = "message_format_tests.rs"]`
- `55: mod message_format_tests;`

### crates/vw-agent/src/session/processor/utils/output_compaction.rs

- `360: #[path = "output_compaction_tests.rs"]`
- `361: mod output_compaction_tests;`

### crates/vw-agent/src/session/processor/utils/session_ingest.rs

- `117: #[path = "session_ingest_tests.rs"]`
- `118: mod session_ingest_tests;`

### crates/vw-agent/src/session/processor/utils/tool_parsing.rs

- `108: #[path = "tool_parsing_tests.rs"]`
- `109: mod tool_parsing_tests;`

### crates/vw-agent/src/session/processor/utils.rs

- `3: mod docs;`
- `4: mod file_links;`
- `5: mod message_format;`
- `6: mod output_compaction;`
- `7: mod session_ingest;`
- `8: mod tool_parsing;`
- `34: #[path = "utils_tests.rs"]`
- `35: mod utils_tests;`

### crates/vw-agent/src/session/prompt.rs

- `17: const MAX_STEPS: &str = include_str!("prompt/max-steps.txt");`
- `267: #[path = "prompt_tests.rs"]`
- `268: mod prompt_tests;`

### crates/vw-agent/src/session/prompt_tests.rs

- `9: assert_eq!(text, include_str!("prompt/max-steps.txt").trim());`

### crates/vw-agent/src/session/retry.rs

- `321: #[path = "retry_tests.rs"]`
- `322: mod retry_tests;`

### crates/vw-agent/src/session/revert.rs

- `33: use std::sync::LazyLock;`
- `38: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `463: #[path = "revert_tests.rs"]`
- `464: mod revert_tests;`

### crates/vw-agent/src/session/runtime/dynamic_calls.rs

- `7: use std::sync::{Arc, Mutex, Weak};`
- `48: state: Mutex<ConnectionState>,`
- `77: state: Mutex::new(ConnectionState::Open(HashMap::new())),`

### crates/vw-agent/src/session/runtime/dynamic_hosts.rs

- `18: use std::sync::{Arc, LazyLock, Mutex};`
- `23: static HOSTS: LazyLock<Mutex<Vec<Arc<SessionHost>>>> = LazyLock::new(|| Mutex::new(Vec::new()));`
- `37: requests: Mutex<Option<ClientToolRequests>>,`
- `41: state: Mutex<HostCatalogState>,`
- `81: requests: Mutex::new(Some(requests)),`
- `85: state: Mutex::new(HostCatalogState { restored, closed: false }),`

### crates/vw-agent/src/session/runtime/lifecycle_tests.rs

- `7: use tokio::sync::Mutex;`
- `15: static LIFECYCLE_TEST_LOCK: Mutex<()> = Mutex::const_new(());`

### crates/vw-agent/src/session/runtime/mod.rs

- `3: mod lifecycle;`
- `4: mod prompt;`
- `5: mod types;`
- `20: #[path = "lifecycle_tests.rs"]`
- `21: mod lifecycle_tests;`
- `24: #[path = "prompt_tests.rs"]`
- `25: mod prompt_tests;`
- `27: pub mod dynamic_calls;`
- `29: mod dynamic_calls_tests;`
- `32: pub(crate) mod dynamic_hosts;`

### crates/vw-agent/src/session/runtime/prompt.rs

- `85: shell_cutoff: Arc::new(std::sync::Mutex::new(None)),`

### crates/vw-agent/src/session/runtime/types.rs

- `224: pub(crate) shell_cutoff: Arc<std::sync::Mutex<Option<std::time::Instant>>>,`

### crates/vw-agent/src/session/session.rs

- `44: use std::sync::LazyLock;`
- `46: #[path = "turn_fork.rs"]`
- `47: mod turn_fork;`
- `140: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `150: pub mod event {`
- `301: static RE: LazyLock<regex::Regex> =`
- `302: LazyLock::new(|| regex::Regex::new(r"^(.+) \(fork #(\d+)\)$").unwrap());`
- `1152: #[path = "session_tests.rs"]`
- `1153: mod session_tests;`

### crates/vw-agent/src/session/skill_checkpoint.rs

- `127: #[path = "skill_checkpoint_tests.rs"]`
- `128: mod tests;`

### crates/vw-agent/src/session/status.rs

- `32: use std::sync::LazyLock;`
- `33: use std::sync::Mutex;`
- `38: pub mod event {`
- `92: /// 使用 LazyLock<Mutex<HashMap>> 提供线程安全的全局状态管理。`
- `94: static STATE: LazyLock<Mutex<HashMap<String, Info>>> = LazyLock::new(|| Mutex::new(HashMap::new()));`
- `244: #[path = "status_tests.rs"]`
- `245: mod status_tests;`

### crates/vw-agent/src/session/status_tests.rs

- `6: use std::sync::{Arc, Mutex};`
- `93: let events = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/session/store/postgres.rs

- `2242: #[path = "postgres_tests.rs"]`
- `2243: mod postgres_tests;`

### crates/vw-agent/src/session/store.rs

- `22: mod history;`
- `23: mod postgres;`

### crates/vw-agent/src/session/summary.rs

- `19: use std::sync::LazyLock;`
- `24: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `717: #[path = "summary_tests.rs"]`
- `718: mod summary_tests;`

### crates/vw-agent/src/session/system.rs

- `35: const PROMPT_UNIFIED: &str = include_str!("prompt/unified.txt");`
- `638: #[path = "system_tests.rs"]`
- `639: mod system_tests;`

### crates/vw-agent/src/session/task_session.rs

- `6: use std::sync::OnceLock;`
- `7: use std::sync::{Arc, Mutex};`
- `71: state: Mutex<CoordinatorState>,`
- `86: state: Mutex::new(CoordinatorState::default()),`
- `279: static COORDINATOR: OnceLock<TaskSessionCoordinator> = OnceLock::new();`

### crates/vw-agent/src/session/title.rs

- `133: use std::sync::{Arc, Mutex};`
- `135: let output = Arc::new(Mutex::new(String::new()));`
- `361: #[path = "title_tests.rs"]`
- `362: mod title_tests;`

### crates/vw-agent/src/session/todo.rs

- `13: pub mod event {`
- `166: #[path = "todo_tests.rs"]`
- `167: mod todo_tests;`

### crates/vw-agent/src/session/ui_config.rs

- `100: #[path = "ui_config_tests.rs"]`
- `101: mod ui_config_tests;`

### crates/vw-agent/src/session/ui_config_tests.rs

- `9: static GLOBAL_PATHS_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =`
- `10: std::sync::OnceLock::new();`
- `175: .get_or_init(|| std::sync::Mutex::new(()))`

### crates/vw-agent/src/session/ui_store.rs

- `22: use std::sync::LazyLock;`
- `23: use std::sync::Mutex;`
- `37: static SESSION_SCOPE: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));`
- `38: static TASK_STATE_WRITE: Mutex<()> = Mutex::new(());`
- `39: static PINNED_THREADS_WRITE: Mutex<()> = Mutex::new(());`
- `117: static SESSION_DATA_DIR: LazyLock<PathBuf> = LazyLock::new(resolve_session_data_dir);`
- `119: static RUNTIME_SESSION_STORE: LazyLock<Mutex<Option<SessionStore>>> =`
- `120: LazyLock::new(|| Mutex::new(None));`
- `1659: #[path = "ui_store_tests.rs"]`
- `1660: mod ui_store_tests;`

### crates/vw-agent/src/session/ui_store_tests.rs

- `77: static UI_STORE_GLOBAL_STATE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));`

### crates/vw-agent/src/shell/tests.rs

- `6: mod tests {`

### crates/vw-agent/src/shiju/account.rs

- `25: #[path = "connectors.rs"]`
- `26: mod connectors;`
- `28: #[path = "knowledge.rs"]`
- `29: mod knowledge;`
- `31: #[path = "monthly_quota.rs"]`
- `32: mod monthly_quota;`
- `34: #[path = "model_token.rs"]`
- `35: mod model_token;`
- `561: #[path = "account_tests.rs"]`
- `562: mod account_tests;`

### crates/vw-agent/src/shiju/apps.rs

- `436: #[path = "apps_tests.rs"]`
- `437: mod apps_tests;`

### crates/vw-agent/src/shiju/connectors.rs

- `168: #[path = "connectors_tests.rs"]`
- `169: mod tests;`

### crates/vw-agent/src/shiju/knowledge.rs

- `167: #[path = "knowledge_tests.rs"]`
- `168: mod shiju_knowledge_tests;`
- `170: #[path = "knowledge_content.rs"]`
- `171: mod knowledge_content;`
- `173: #[path = "knowledge_content_tests.rs"]`
- `174: mod shiju_knowledge_content_tests;`

### crates/vw-agent/src/shiju/mod.rs

- `6: pub(crate) mod account;`
- `8: pub(crate) mod apps;`

### crates/vw-agent/src/shiju/model_token.rs

- `82: #[path = "model_token_tests.rs"]`
- `83: mod tests;`

### crates/vw-agent/src/skill/audit.rs

- `16: use std::sync::OnceLock;`
- `745: /// - 使用 OnceLock 缓存编译后的正则表达式`
- `748: static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();`
- `934: /// - 使用 OnceLock 缓存编译后的正则表达式，避免重复编译`
- `937: static HIGH_RISK_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();`
- `1016: #[path = "audit_tests.rs"]`
- `1017: mod audit_tests;`

### crates/vw-agent/src/skill/constants.rs

- `50: /// 技能的 Markdown 格式文档内容，在编译时通过 include_str! 宏嵌入`
- `68: markdown: include_str!(concat!(`
- `76: markdown: include_str!(concat!(`
- `84: markdown: include_str!(concat!(`
- `135: #[path = "constants_tests.rs"]`
- `136: mod constants_tests;`

### crates/vw-agent/src/skill/discovery.rs

- `33: use std::sync::LazyLock;`
- `39: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `366: #[path = "discovery_tests.rs"]`
- `367: mod discovery_tests;`

### crates/vw-agent/src/skill/install.rs

- `1024: #[path = "install_tests.rs"]`
- `1025: mod install_tests;`

### crates/vw-agent/src/skill/loader.rs

- `372: #[path = "loader_tests.rs"]`
- `373: mod loader_tests;`

### crates/vw-agent/src/skill/mod.rs

- `61: pub mod audit;`
- `71: mod discovery;`
- `77: mod skill;`
- `83: mod constants;`
- `89: mod install;`
- `95: mod loader;`
- `103: mod policy;`
- `109: mod prompt;`
- `115: mod source;`
- `125: mod types;`

### crates/vw-agent/src/skill/policy.rs

- `447: #[path = "policy_tests.rs"]`
- `448: mod policy_tests;`

### crates/vw-agent/src/skill/prompt.rs

- `141: #[path = "prompt_tests.rs"]`
- `142: mod prompt_tests;`

### crates/vw-agent/src/skill/skill.rs

- `42: use std::sync::LazyLock;`
- `48: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `649: #[path = "skill_tests.rs"]`
- `650: mod skill_tests;`

### crates/vw-agent/src/skill/source.rs

- `277: #[path = "source_tests.rs"]`
- `278: mod source_tests;`

### crates/vw-agent/src/skill/symlink_tests/mod.rs

- `21: #[path = "tests.rs"]`
- `31: mod tests;`

### crates/vw-agent/src/skill/symlink_tests/tests.rs

- `7: mod tests {`

### crates/vw-agent/src/skill/tests/mod.rs

- `3: mod download_policy;`
- `4: mod git_source;`
- `5: mod init;`
- `6: mod load_skills;`
- `7: mod md_skill;`
- `8: mod prompt;`
- `9: mod skills_sh;`
- `10: mod toml_skill;`

### crates/vw-agent/src/skill/types.rs

- `297: #[path = "types_tests.rs"]`
- `298: mod types_tests;`

### crates/vw-agent/src/skillforge/evaluate/mod.rs

- `460: #[path = "tests.rs"]`
- `461: mod tests;`

### crates/vw-agent/src/skillforge/evaluate/tests.rs

- `17: mod tests {`

### crates/vw-agent/src/skillforge/integrate/mod.rs

- `405: #[path = "tests.rs"]`
- `406: mod tests;`

### crates/vw-agent/src/skillforge/integrate/tests.rs

- `19: mod tests {`

### crates/vw-agent/src/skillforge/mod.rs

- `54: pub mod evaluate;`
- `55: pub mod integrate;`
- `56: pub mod scout;`
- `545: #[path = "tests.rs"]`
- `546: mod tests;`

### crates/vw-agent/src/skillforge/scout/mod.rs

- `601: #[path = "tests.rs"]`
- `602: mod tests;`

### crates/vw-agent/src/skillforge/scout/tests.rs

- `12: mod tests {`

### crates/vw-agent/src/skillforge/tests.rs

- `16: mod tests {`

### crates/vw-agent/src/skills/audit/manifest.rs

- `81: #[path = "manifest_tests.rs"]`
- `82: mod manifest_tests;`

### crates/vw-agent/src/skills/audit/markdown.rs

- `7: use std::sync::OnceLock;`
- `168: static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();`
- `187: #[path = "markdown_tests.rs"]`
- `188: mod markdown_tests;`

### crates/vw-agent/src/skills/audit/mod.rs

- `44: mod manifest;`
- `45: mod markdown;`
- `46: mod report;`
- `47: mod risk;`
- `48: mod scan;`
- `49: mod support;`
- `221: #[path = "tests.rs"]`
- `222: mod tests;`

### crates/vw-agent/src/skills/audit/report.rs

- `73: #[path = "report_tests.rs"]`
- `74: mod report_tests;`

### crates/vw-agent/src/skills/audit/risk.rs

- `4: use std::sync::OnceLock;`
- `13: static HIGH_RISK_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();`
- `80: #[path = "risk_tests.rs"]`
- `81: mod risk_tests;`

### crates/vw-agent/src/skills/audit/scan.rs

- `136: #[path = "scan_tests.rs"]`
- `137: mod scan_tests;`

### crates/vw-agent/src/skills/audit/support.rs

- `135: #[path = "support_tests.rs"]`
- `136: mod support_tests;`

### crates/vw-agent/src/skills/cli.rs

- `303: #[path = "cli_tests.rs"]`
- `304: mod cli_tests;`

### crates/vw-agent/src/skills/cli_tests.rs

- `11: use std::sync::{Mutex, OnceLock};`
- `14: fn env_lock() -> &'static Mutex<()> {`
- `15: static LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `16: LOCK.get_or_init(|| Mutex::new(()))`

### crates/vw-agent/src/skills/init.rs

- `96: #[path = "init_tests.rs"]`
- `97: mod init_tests;`

### crates/vw-agent/src/skills/installer.rs

- `77: markdown: include_str!(concat!(`
- `85: markdown: include_str!(concat!(`
- `93: markdown: include_str!(concat!(`
- `269: /// SKILL.md 文件内容（编译时通过 include_str! 嵌入）`
- `843: #[path = "installer_tests.rs"]`
- `844: mod installer_tests;`

### crates/vw-agent/src/skills/loader.rs

- `420: #[path = "loader_tests.rs"]`
- `421: mod loader_tests;`

### crates/vw-agent/src/skills/marketplace/mod.rs

- `7: pub(crate) mod install;`
- `9: pub(crate) mod provider;`
- `11: pub(crate) mod skillhub;`
- `14: #[path = "install_tests.rs"]`
- `15: mod install_tests;`
- `18: #[path = "skillhub_tests.rs"]`
- `19: mod skillhub_tests;`

### crates/vw-agent/src/skills/marketplace/provider.rs

- `220: #[path = "provider_tests.rs"]`
- `221: mod provider_tests;`

### crates/vw-agent/src/skills/mod.rs

- `35: mod agent_metadata;`
- `36: mod audit;`
- `37: mod cli;`
- `38: mod discovery;`
- `39: mod init;`
- `40: mod installer;`
- `41: mod invoked;`
- `42: mod listing;`
- `43: mod loader;`
- `44: pub(crate) mod marketplace;`
- `45: mod policy;`
- `46: mod prompt;`
- `47: pub mod session_mounts;`
- `48: mod session_state;`
- `49: pub(crate) mod shiju;`
- `50: mod source;`
- `51: mod types;`
- `550: mod symlink_tests;`
- `554: mod tests;`
- `557: #[path = "mod_tests.rs"]`
- `558: mod mod_tests;`
- `561: #[path = "discovery_tests.rs"]`
- `562: mod discovery_tests;`
- `565: #[path = "listing_tests.rs"]`
- `566: mod listing_tests;`
- `569: #[path = "invoked_tests.rs"]`
- `570: mod invoked_tests;`
- `572: #[path = "session_state_tests.rs"]`
- `573: mod session_state_tests;`
- `576: mod session_mounts_tests;`

### crates/vw-agent/src/skills/policy.rs

- `290: #[path = "policy_tests.rs"]`
- `291: mod policy_tests;`

### crates/vw-agent/src/skills/prompt.rs

- `375: #[path = "prompt_tests.rs"]`
- `376: mod prompt_tests;`

### crates/vw-agent/src/skills/session_mounts_tests.rs

- `2: mod supported {`

### crates/vw-agent/src/skills/shiju.rs

- `477: #[path = "shiju_tests.rs"]`
- `478: mod shiju_tests;`

### crates/vw-agent/src/skills/source.rs

- `214: #[path = "source_tests.rs"]`
- `215: mod source_tests;`

### crates/vw-agent/src/skills/symlink_tests/mod.rs

- `9: #[path = "tests.rs"]`
- `11: mod tests;`

### crates/vw-agent/src/skills/symlink_tests/tests.rs

- `27: mod tests {`

### crates/vw-agent/src/skills/tests/helpers.rs

- `23: use std::sync::{Mutex, OnceLock};`
- `32: /// 返回一个指向静态 Mutex<()> 的引用。该互斥锁在整个程序生命周期内存在，`
- `47: /// 使用 OnceLock 实现线程安全的单次初始化，确保全局只有一个互斥锁实例。`
- `48: pub fn env_var_lock() -> &'static Mutex<()> {`
- `49: static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();`
- `50: ENV_LOCK.get_or_init(|| Mutex::new(()))`

### crates/vw-agent/src/skills/tests/mod.rs

- `37: pub(crate) mod helpers;`
- `39: mod domain_trust;`
- `41: mod download_policy;`
- `43: mod git_source;`
- `45: mod load_skills;`
- `47: mod skills_init;`
- `49: mod skills_prompt;`
- `51: mod skills_sh_source;`

### crates/vw-agent/src/skills/types.rs

- `449: #[path = "types_tests.rs"]`
- `450: mod types_tests;`

### crates/vw-agent/src/snapshot/common.rs

- `11: use std::sync::LazyLock;`
- `23: pub(super) static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `110: #[path = "common_tests.rs"]`
- `111: mod common_tests;`

### crates/vw-agent/src/snapshot/mod.rs

- `38: mod common;`
- `40: mod native;`
- `42: mod wasm;`

### crates/vw-agent/src/snapshot/native.rs

- `488: #[path = "native_tests.rs"]`
- `489: mod native_tests;`

### crates/vw-agent/src/snapshot/wasm.rs

- `51: #[path = "wasm_tests.rs"]`
- `52: mod wasm_tests;`

### crates/vw-agent/src/storage/mod.rs

- `16: use std::sync::LazyLock;`
- `88: static STATE: LazyLock<OnceCell<StorageState>> = LazyLock::new(OnceCell::new);`
- `90: static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {`
- `630: mod tests;`

### crates/vw-agent/src/task_lifecycle/builtin.rs

- `701: #[path = "builtin_tests.rs"]`
- `702: mod builtin_tests;`

### crates/vw-agent/src/task_lifecycle/mod.rs

- `6: pub mod builtin;`
- `7: pub mod registry;`
- `8: pub mod store;`
- `17: #[path = "registry_tests.rs"]`
- `18: mod registry_tests;`

### crates/vw-agent/src/task_lifecycle/store/postgres.rs

- `188: #[path = "postgres_tests.rs"]`
- `189: mod postgres_tests;`

### crates/vw-agent/src/task_lifecycle/store.rs

- `17: mod postgres;`
- `456: #[path = "store_tests.rs"]`
- `457: mod store_tests;`

### crates/vw-agent/src/task_plan_run/control.rs

- `4: use std::sync::{Arc, Mutex, OnceLock};`
- `55: inner: Arc<Mutex<RegistryState>>,`
- `59: static GLOBAL_PLAN_RUN_CONTROL_REGISTRY: OnceLock<PlanRunControlRegistry> = OnceLock::new();`
- `68: inner: Arc::new(Mutex::new(RegistryState::default())),`

### crates/vw-agent/src/task_plan_run/mod.rs

- `6: mod control;`
- `7: mod goal;`
- `8: mod model;`
- `9: mod prompts;`
- `10: mod runner;`
- `11: mod schedule;`
- `12: mod session_bind;`
- `32: #[path = "schedule_tests.rs"]`
- `33: mod schedule_tests;`
- `36: #[path = "runner_tests.rs"]`
- `37: mod runner_tests;`
- `40: #[path = "prompts_tests.rs"]`
- `41: mod prompts_tests;`
- `44: #[path = "session_bind_tests.rs"]`
- `45: mod session_bind_tests;`
- `48: #[path = "model_tests.rs"]`
- `49: mod model_tests;`
- `52: #[path = "goal_tests.rs"]`
- `53: mod goal_tests;`
- `61: #[path = "control_tests.rs"]`
- `62: mod control_tests;`

### crates/vw-agent/src/task_plan_run/prompts.rs

- `15: include_str!("../../assets/runtime-skills/create-task-plans-runtime/SKILL.md");`
- `18: include_str!("../../assets/runtime-skills/do-task-plans-runtime/SKILL.md");`

### crates/vw-agent/src/task_plan_run/runner_tests.rs

- `11: use std::sync::{Arc, Mutex};`
- `19: responses: Arc<Mutex<VecDeque<Result<String, String>>>>,`
- `20: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `21: task_files: Arc<Mutex<Vec<PathBuf>>>,`
- `24: calls: Arc<Mutex<Vec<SessionRunRequest>>>,`
- `35: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `42: responses: Arc<Mutex<VecDeque<Result<String, String>>>>,`
- `43: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `44: task_files: Arc<Mutex<Vec<PathBuf>>>,`
- `45: calls: Arc<Mutex<Vec<SessionRunRequest>>>,`
- `51: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `64: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `68: calls: Arc<Mutex<Vec<SessionRunRequest>>>,`
- `73: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `78: calls: Arc<Mutex<Vec<SessionRunRequest>>>,`
- `95: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `98: calls: Arc::new(Mutex::new(Vec::new())),`
- `112: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `116: calls: Arc::new(Mutex::new(Vec::new())),`
- `124: responses: Arc::new(Mutex::new(VecDeque::from(responses))),`
- `125: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `126: task_files: Arc::new(Mutex::new(Vec::new())),`
- `128: calls: Arc::new(Mutex::new(Vec::new())),`
- `134: responses: Arc::new(Mutex::new(VecDeque::from(responses))),`
- `135: plan_fixture: Arc::new(Mutex::new(None)),`
- `136: task_files: Arc::new(Mutex::new(Vec::new())),`
- `138: calls: Arc::new(Mutex::new(Vec::new())),`
- `147: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `160: responses: Arc::new(Mutex::new(VecDeque::from(responses))),`
- `161: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `162: task_files: Arc::new(Mutex::new(Vec::new())),`
- `163: calls: Arc::new(Mutex::new(Vec::new())),`
- `173: plan_fixture: Arc::new(Mutex::new(Some(plan_fixture))),`
- `610: events: &Arc<Mutex<Vec<TaskPlanRunStreamEventDto>>>,`
- `638: let events = Arc::new(Mutex::new(Vec::new()));`
- `708: let events = Arc::new(Mutex::new(Vec::new()));`
- `837: let events = Arc::new(Mutex::new(Vec::new()));`
- `1094: let events = Arc::new(Mutex::new(Vec::new()));`
- `1095: let started_info_available = Arc::new(Mutex::new(Vec::new()));`
- `1197: let events = Arc::new(Mutex::new(Vec::new()));`
- `1278: let events = Arc::new(Mutex::new(Vec::new()));`
- `1337: let events = Arc::new(Mutex::new(Vec::new()));`
- `1376: let events = Arc::new(Mutex::new(Vec::new()));`
- `1424: let events = Arc::new(Mutex::new(Vec::new()));`
- `1486: let events = Arc::new(Mutex::new(Vec::new()));`
- `1548: let events = Arc::new(Mutex::new(Vec::new()));`
- `1592: let events = Arc::new(Mutex::new(Vec::new()));`
- `1751: calls: Mutex<Vec<String>>,`
- `1809: calls: Mutex::new(Vec::new()),`

### crates/vw-agent/src/task_plan_run/session_bind_tests.rs

- `10: use std::sync::{Arc, Mutex};`
- `15: responses: Arc<Mutex<VecDeque<Result<String, String>>>>,`
- `16: plan_fixture: Arc<Mutex<Option<PlanFixture>>>,`
- `17: task_files: Arc<Mutex<Vec<PathBuf>>>,`
- `191: responses: Arc::new(Mutex::new(VecDeque::from(vec![`
- `195: plan_fixture: Arc::new(Mutex::new(Some(PlanFixture {`
- `199: task_files: Arc::new(Mutex::new(Vec::new())),`
- `249: responses: Arc::new(Mutex::new(VecDeque::from(vec![`
- `253: plan_fixture: Arc::new(Mutex::new(Some(PlanFixture {`
- `257: task_files: Arc::new(Mutex::new(Vec::new())),`
- `267: let observed = Arc::new(Mutex::new(None));`
- `303: responses: Arc::new(Mutex::new(VecDeque::from(vec![`
- `307: plan_fixture: Arc::new(Mutex::new(Some(PlanFixture {`
- `311: task_files: Arc::new(Mutex::new(Vec::new())),`
- `321: let finished = Arc::new(Mutex::new(None));`

### crates/vw-agent/src/task_worktree_pool/mod.rs

- `8: use std::sync::{Arc, Mutex};`
- `41: pools: Arc<Mutex<HashMap<String, ProjectPoolState>>>,`
- `493: #[path = "tests.rs"]`
- `494: mod tests;`

### crates/vw-agent/src/test_capabilities.rs

- `188: #[path = "test_capabilities_tests.rs"]`
- `189: mod test_capabilities_tests;`

### crates/vw-agent/src/test_capabilities_tests.rs

- `7: use std::sync::{Mutex, MutexGuard};`
- `10: static ENV_LOCK: Mutex<()> = Mutex::new(());`

### crates/vw-agent/src/tools/agent_tool/mod.rs

- `88: permit: Arc<std::sync::Mutex<Option<OwnedSemaphorePermit>>>,`
- `903: mod tests;`
- `906: mod communication_tests;`
- `908: mod communication;`
- `911: #[path = "../../session/child_agent/collaboration_tests.rs"]`
- `912: mod collaboration_tests;`

### crates/vw-agent/src/tools/agent_tool/tests.rs

- `360: let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));`
- `361: let started_tx = Arc::new(std::sync::Mutex::new(Some(started_tx)));`
- `833: let cancelled_tx = Arc::new(std::sync::Mutex::new(Some(cancelled_tx)));`
- `1080: let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));`
- `1081: let started_tx = Arc::new(std::sync::Mutex::new(Some(started_tx)));`
- `1083: let created_tx = Arc::new(std::sync::Mutex::new(Some(created_tx)));`
- `1147: let created_ids = Arc::new(std::sync::Mutex::new(Vec::new()));`
- `1313: let dropped = Arc::new(std::sync::Mutex::new(Some(dropped_tx)));`

### crates/vw-agent/src/tools/apply_patch/mod.rs

- `623: include_str!("apply_patch.txt")`
- `654: #[path = "tests.rs"]`
- `655: mod tests;`
- `658: mod obsolete_apply_patch_impl {`
- `1155: include_str!("apply_patch.txt")`

### crates/vw-agent/src/tools/batch/mod.rs

- `91: include_str!("batch.txt")`
- `261: #[path = "tests.rs"]`
- `262: mod tests;`

### crates/vw-agent/src/tools/browser/actions.rs

- `523: #[path = "actions_tests.rs"]`
- `524: mod actions_tests;`

### crates/vw-agent/src/tools/browser/agent_browser.rs

- `405: #[path = "agent_browser_tests.rs"]`
- `406: mod agent_browser_tests;`

### crates/vw-agent/src/tools/browser/backend.rs

- `216: #[path = "backend_tests.rs"]`
- `217: mod backend_tests;`

### crates/vw-agent/src/tools/browser/computer_use/config.rs

- `94: #[path = "config_tests.rs"]`
- `95: mod config_tests;`

### crates/vw-agent/src/tools/browser/computer_use/execution.rs

- `126: #[path = "execution_tests.rs"]`
- `127: mod execution_tests;`

### crates/vw-agent/src/tools/browser/computer_use/mod.rs

- `35: mod config;`
- `36: mod execution;`
- `37: mod response;`
- `38: mod validation;`
- `94: mod tests;`

### crates/vw-agent/src/tools/browser/computer_use/response.rs

- `36: #[path = "response_tests.rs"]`
- `37: mod response_tests;`

### crates/vw-agent/src/tools/browser/computer_use/validation.rs

- `319: #[path = "validation_tests.rs"]`
- `320: mod validation_tests;`

### crates/vw-agent/src/tools/browser/helpers.rs

- `383: #[path = "helpers_tests.rs"]`
- `384: mod helpers_tests;`

### crates/vw-agent/src/tools/browser/installation.rs

- `136: #[path = "installation_tests.rs"]`
- `137: mod tests;`

### crates/vw-agent/src/tools/browser/mod.rs

- `29: mod actions;`
- `30: mod agent_browser;`
- `31: mod backend;`
- `32: mod computer_use;`
- `33: mod helpers;`
- `34: pub(crate) mod installation;`
- `35: mod tool;`
- `38: mod native_backend;`
- `130: native_state: tokio::sync::Mutex<native_backend::NativeBrowserState>,`
- `134: #[path = "tests.rs"]`
- `135: mod tests;`

### crates/vw-agent/src/tools/browser/native_backend/actions.rs

- `96: #[path = "actions_tests.rs"]`
- `97: mod actions_tests;`

### crates/vw-agent/src/tools/browser/native_backend/actions_tests.rs

- `14: use tokio::sync::Mutex;`
- `55: responses: Mutex<VecDeque<MockResponse>>,`
- `56: requests: Mutex<Vec<RecordedRequest>>,`
- `68: responses: Mutex::new(responses.into()),`
- `69: requests: Mutex::new(Vec::new()),`

### crates/vw-agent/src/tools/browser/native_backend/mod.rs

- `48: mod actions;`
- `49: mod session;`
- `50: mod snapshot;`
- `51: mod webdriver;`
- `501: mod tests;`

### crates/vw-agent/src/tools/browser/native_backend/session.rs

- `43: /// 该类型不是线程安全的。如需跨线程共享，请使用 Arc<Mutex<NativeBrowserState>>。`
- `243: #[path = "session_tests.rs"]`
- `244: mod session_tests;`

### crates/vw-agent/src/tools/browser/native_backend/session_tests.rs

- `12: use std::sync::{Arc, Mutex};`
- `24: requests: Arc<Mutex<Vec<CapturedRequest>>>,`
- `30: requests: Arc<Mutex<Vec<CapturedRequest>>>,`
- `40: let requests = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/tools/browser/native_backend/snapshot.rs

- `176: #[path = "snapshot_tests.rs"]`
- `177: mod snapshot_tests;`

### crates/vw-agent/src/tools/browser/native_backend/tests.rs

- `10: use std::sync::{Arc, Mutex};`
- `27: requests: Arc<Mutex<Vec<MockRequest>>>,`

### crates/vw-agent/src/tools/browser/native_backend/webdriver.rs

- `371: #[path = "webdriver_tests.rs"]`
- `372: mod webdriver_tests;`

### crates/vw-agent/src/tools/browser/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/tools/browser/tool.rs

- `67: native_state: tokio::sync::Mutex::new(`
- `676: #[path = "tool_tests.rs"]`
- `677: mod tool_tests;`

### crates/vw-agent/src/tools/browser_open/mod.rs

- `634: #[path = "tests.rs"]`
- `635: mod tests;`

### crates/vw-agent/src/tools/catalog.rs

- `9: use std::sync::{Arc, RwLock};`
- `67: current: RwLock<Arc<ToolCatalogSnapshot>>,`
- `81: current: RwLock::new(Arc::new(ToolCatalogSnapshot {`

### crates/vw-agent/src/tools/codesearch/mod.rs

- `92: include_str!("codesearch.txt")`
- `228: #[path = "tests.rs"]`
- `229: mod tests;`

### crates/vw-agent/src/tools/computer_control/chrome.rs

- `95: policy: &Arc<RwLock<ComputerControlPolicy>>,`
- `191: #[path = "chrome_tests.rs"]`
- `192: mod tests;`

### crates/vw-agent/src/tools/computer_control/chrome_tests.rs

- `6: type ApprovalLog = Arc<parking_lot::Mutex<Vec<(String, Value)>>>;`
- `19: let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));`
- `24: fn authority() -> (Arc<ControlHost>, Arc<RwLock<ComputerControlPolicy>>, ComputerControlRequest) {`
- `33: let p = Arc::new(RwLock::new(p));`
- `135: let runtime=NativeControlRuntime::with_factory(Arc::new(RwLock::new(p)),Arc::new(|_,_,_,_,_,_|panic!("Chrome must never enter native backend")));`
- `175: Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default()))),`

### crates/vw-agent/src/tools/computer_control.rs

- `3: #[path = "computer_control/chrome.rs"]`
- `4: mod chrome;`
- `13: use parking_lot::RwLock;`
- `23: sync::{Mutex, OnceCell},`
- `33: static GATEWAY_RUNTIME: RwLock<Weak<NativeControlRuntime>> = RwLock::new(Weak::new());`
- `37: Arc<RwLock<ComputerControlPolicy>>,`
- `78: pub(super) policy: Arc<RwLock<ComputerControlPolicy>>,`
- `82: RwLock<Option<Arc<crate::computer_control::office::transport::OfficeTransport>>>,`
- `84: input_gate: Arc<Mutex<()>>,`
- `85: sessions: Mutex<HashMap<Owner, Arc<NativeSession>>>,`
- `95: Arc::new(RwLock::new(ComputerControlPolicy::new(config))),`
- `107: policy: Arc<RwLock<ComputerControlPolicy>>,`
- `115: office: RwLock::new(None),`
- `117: input_gate: Arc::new(Mutex::new(())),`
- `118: sessions: Mutex::new(HashMap::new()),`
- `157: pub(crate) fn control_policy(&self) -> &Arc<RwLock<ComputerControlPolicy>> { &self.policy }`
- `249: client: Mutex::new(Client { stream: client, sequence: 1 }),`
- `297: client: Mutex<Client>,`

### crates/vw-agent/src/tools/computer_control_tests.rs

- `4: use parking_lot::RwLock;`
- `59: let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));`
- `329: let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));`

### crates/vw-agent/src/tools/config/mod.rs

- `819: #[path = "tests.rs"]`
- `820: mod tests;`

### crates/vw-agent/src/tools/content_search/mod.rs

- `11: use std::sync::{Arc, OnceLock};`
- `741: static MATCH_RE: OnceLock<regex::Regex> = OnceLock::new();`
- `742: static CONTEXT_RE: OnceLock<regex::Regex> = OnceLock::new();`
- `773: static COUNT_RE: OnceLock<regex::Regex> = OnceLock::new();`
- `808: #[path = "tests.rs"]`
- `809: mod tests;`

### crates/vw-agent/src/tools/context/mod.rs

- `12: pub(crate) mod run_worktree;`
- `27: use std::sync::{Arc, Mutex};`
- `117: hook_contexts: Arc<Mutex<Vec<String>>>,`
- `136: read_state: Arc<Mutex<FileReadStateCache>>,`
- `150: workflow: Arc<Mutex<WorkflowState>>,`
- `151: pending_image_reloads: Arc<Mutex<PendingImageReloadQueue>>,`
- `156: invoked_skills: Arc<Mutex<Vec<crate::skills::InvokedSkillRecord>>>,`
- `224: hook_contexts: Arc::new(Mutex::new(Vec::new())),`
- `243: read_state: Arc::new(Mutex::new(FileReadStateCache::default())),`
- `257: workflow: Arc::new(Mutex::new(WorkflowState::default())),`
- `258: pending_image_reloads: Arc::new(Mutex::new(PendingImageReloadQueue::default())),`
- `264: invoked_skills: Arc::new(Mutex::new(Vec::new())),`
- `517: pub fn read_state_handle(&self) -> Arc<Mutex<FileReadStateCache>> {`
- `740: pub fn with_read_state_handle(mut self, read_state: Arc<Mutex<FileReadStateCache>>) -> Self {`
- `746: pub fn with_workflow_handle(mut self, workflow: Arc<Mutex<WorkflowState>>) -> Self {`
- `754: pending_image_reloads: Arc<Mutex<PendingImageReloadQueue>>,`
- `768: pub fn pending_image_reloads_handle(&self) -> Arc<Mutex<PendingImageReloadQueue>> {`
- `995: mod tests;`

### crates/vw-agent/src/tools/context/run_worktree.rs

- `15: pub claims: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,`

### crates/vw-agent/src/tools/context/tests.rs

- `167: let read_state = Arc::new(Mutex::new(FileReadStateCache::new(8, 1024)));`
- `168: let workflow = Arc::new(Mutex::new(WorkflowState::default()));`

### crates/vw-agent/src/tools/cron_add/mod.rs

- `687: #[path = "tests.rs"]`
- `688: mod tests;`

### crates/vw-agent/src/tools/cron_list/mod.rs

- `64: #[path = "tests.rs"]`
- `65: mod tests;`

### crates/vw-agent/src/tools/cron_remove/mod.rs

- `234: #[path = "tests.rs"]`
- `235: mod tests;`

### crates/vw-agent/src/tools/cron_run/mod.rs

- `283: #[path = "tests.rs"]`
- `284: mod tests;`

### crates/vw-agent/src/tools/cron_runs/mod.rs

- `121: #[path = "tests.rs"]`
- `122: mod tests;`

### crates/vw-agent/src/tools/cron_update/mod.rs

- `373: #[path = "tests.rs"]`
- `374: mod tests;`

### crates/vw-agent/src/tools/decision/mod.rs

- `490: mod tests;`

### crates/vw-agent/src/tools/delegate/agentic.rs

- `130: #[path = "agentic_tests.rs"]`
- `131: mod agentic_tests;`

### crates/vw-agent/src/tools/delegate/execution.rs

- `197: #[path = "execution_tests.rs"]`
- `198: mod execution_tests;`

### crates/vw-agent/src/tools/delegate/mod.rs

- `15: mod agentic;`
- `16: mod execution;`
- `17: mod support;`
- `226: #[path = "tests.rs"]`
- `227: mod tests;`

### crates/vw-agent/src/tools/delegate/support.rs

- `20: #[path = "support_tests.rs"]`
- `21: mod support_tests;`

### crates/vw-agent/src/tools/delegated_tools.rs

- `170: #[path = "delegated_tools_tests.rs"]`
- `171: mod tests;`

### crates/vw-agent/src/tools/discovery.rs

- `5: use std::sync::{Arc, Mutex, OnceLock};`
- `46: static CACHE: OnceLock<Mutex<CatalogCache>> = OnceLock::new();`
- `48: .get_or_init(|| Mutex::new(CatalogCache::default()))`
- `358: #[path = "discovery_tests.rs"]`
- `359: mod tests;`

### crates/vw-agent/src/tools/enter_worktree/mod.rs

- `147: #[path = "tests.rs"]`
- `148: mod tests;`

### crates/vw-agent/src/tools/excel_control_tests.rs

- `8: use parking_lot::RwLock;`
- `20: static CERTS: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();`
- `83: let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));`
- `375: struct DocumentApproval(Arc<parking_lot::Mutex<Vec<(String, Value)>>>);`
- `396: let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/tools/executor/mod.rs

- `447: #[path = "tests.rs"]`
- `448: mod tests;`

### crates/vw-agent/src/tools/exit_worktree/mod.rs

- `225: #[path = "tests.rs"]`
- `226: mod tests;`

### crates/vw-agent/src/tools/exit_worktree/tests.rs

- `14: use std::sync::{Arc, OnceLock};`
- `18: static TEST_HOME: OnceLock<PathBuf> = OnceLock::new();`

### crates/vw-agent/src/tools/external_directory/mod.rs

- `284: #[path = "tests.rs"]`
- `285: mod tests;`

### crates/vw-agent/src/tools/file_edit/mod.rs

- `9: pub(crate) mod types;`
- `10: pub(crate) mod utils;`
- `310: #[path = "tests.rs"]`
- `311: mod tests;`

### crates/vw-agent/src/tools/file_edit/tests.rs

- `13: mod tests {`

### crates/vw-agent/src/tools/file_edit/types.rs

- `52: #[path = "types_tests.rs"]`
- `53: mod types_tests;`

### crates/vw-agent/src/tools/file_edit/utils.rs

- `370: #[path = "utils_tests.rs"]`
- `371: mod utils_tests;`

### crates/vw-agent/src/tools/file_read/mod.rs

- `1308: #[path = "tests.rs"]`
- `1309: mod tests;`

### crates/vw-agent/src/tools/file_read/tests.rs

- `32: mod tests {`
- `1028: mod e2e_helpers {`
- `1033: use std::sync::{Arc, Mutex};`
- `1038: pub type SharedRequests = Arc<Mutex<Vec<Vec<ChatMessage>>>>;`
- `1046: responses: Mutex<Vec<ChatResponse>>,`
- `1060: let requests: SharedRequests = Arc::new(Mutex::new(Vec::new()));`
- `1062: Self { responses: Mutex::new(responses), requests: requests.clone() };`

### crates/vw-agent/src/tools/file_write/mod.rs

- `284: include_str!("write.txt")`
- `336: #[path = "tests.rs"]`
- `337: mod tests;`

### crates/vw-agent/src/tools/file_write/tests.rs

- `26: mod tests {`

### crates/vw-agent/src/tools/git_operations/mod.rs

- `3: mod merge;`
- `520: #[path = "tests.rs"]`
- `521: mod tests;`

### crates/vw-agent/src/tools/git_operations/tests.rs

- `47: let source = include_str!("mod.rs");`

### crates/vw-agent/src/tools/glob/mod.rs

- `202: include_str!("glob.txt")`
- `364: #[path = "tests.rs"]`
- `365: mod tests;`

### crates/vw-agent/src/tools/glob/tests.rs

- `16: mod tests {`

### crates/vw-agent/src/tools/glob_search/mod.rs

- `364: #[path = "tests.rs"]`
- `365: mod tests;`

### crates/vw-agent/src/tools/grep/mod.rs

- `515: include_str!("./grep.txt")`
- `706: #[path = "tests.rs"]`
- `707: mod tests;`

### crates/vw-agent/src/tools/grep/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/tools/hooks/mod.rs

- `81: mod tests;`

### crates/vw-agent/src/tools/http_headers.rs

- `15: #[path = "http_headers_tests.rs"]`
- `16: mod tests;`

### crates/vw-agent/src/tools/http_request/mod.rs

- `477: #[path = "tests.rs"]`
- `478: mod tests;`

### crates/vw-agent/src/tools/image_asset_read/mod.rs

- `136: #[path = "tests.rs"]`
- `137: mod tests;`

### crates/vw-agent/src/tools/ls/mod.rs

- `366: include_str!("./ls.txt")`
- `548: #[path = "tests.rs"]`
- `549: mod tests;`

### crates/vw-agent/src/tools/ls/tests.rs

- `20: mod tests {`

### crates/vw-agent/src/tools/mcp_catalog_refresh_tests.rs

- `14: type RevisionProbe = Arc<std::sync::Mutex<Option<Arc<dyn Fn() -> u64 + Send + Sync>>>>;`
- `91: peer: Arc<tokio::sync::Mutex<Option<Peer<RoleServer>>>>,`
- `99: let peer = Arc::new(tokio::sync::Mutex::new(None));`

### crates/vw-agent/src/tools/mcp_common/mod.rs

- `31: mod wire_check;`
- `32: mod wire_http;`
- `33: mod wire_stdio;`
- `1003: #[path = "tests.rs"]`
- `1004: mod tests;`

### crates/vw-agent/src/tools/mcp_common/tests.rs

- `28: use std::sync::OnceLock;`

### crates/vw-agent/src/tools/mcp_common/wire_http.rs

- `16: Arc, Mutex,`
- `77: protocol: Arc<Mutex<HttpProtocol>>,`
- `165: protocol: Arc<Mutex<HttpProtocol>>,`

### crates/vw-agent/src/tools/mcp_common/wire_stdio.rs

- `13: sync::Mutex,`
- `64: input: Arc<Mutex<Option<ChildStdin>>>,`
- `102: input: Arc::new(Mutex::new(Some(input))),`

### crates/vw-agent/src/tools/mcp_inspect.rs

- `220: #[path = "mcp_inspect_tests.rs"]`
- `221: mod mcp_inspect_tests;`

### crates/vw-agent/src/tools/mcp_status/mod.rs

- `148: mod tests;`

### crates/vw-agent/src/tools/mcp_tool.rs

- `21: sync::{Arc, Mutex},`
- `310: let binding = Arc::new(Mutex::new(connections.tool_binding(&server)));`
- `359: binding: &Arc<Mutex<Option<crate::plugins::ToolCatalogBinding>>>,`

### crates/vw-agent/src/tools/mcp_tool_tests.rs

- `13: use std::sync::{Arc, Mutex};`
- `267: fn dynamic_tool(result: CallToolResult, calls: Arc<Mutex<Vec<(String, JsonObject)>>>) -> McpTool {`
- `309: let calls = Arc::new(Mutex::new(Vec::new()));`
- `327: let calls = Arc::new(Mutex::new(Vec::new()));`
- `349: let calls = Arc::new(Mutex::new(Vec::new()));`
- `377: let calls = Arc::new(Mutex::new(Vec::new()));`
- `395: let calls = Arc::new(Mutex::new(Vec::new()));`
- `412: let calls = Arc::new(Mutex::new(Vec::new()));`
- `472: let calls = Arc::new(Mutex::new(Vec::new()));`
- `498: let calls = Arc::new(Mutex::new(Vec::new()));`

### crates/vw-agent/src/tools/memory_forget/mod.rs

- `158: #[path = "tests.rs"]`
- `159: mod tests;`

### crates/vw-agent/src/tools/memory_recall/mod.rs

- `89: #[path = "tests.rs"]`
- `90: mod tests;`

### crates/vw-agent/src/tools/memory_store/mod.rs

- `176: #[path = "tests.rs"]`
- `177: mod tests;`

### crates/vw-agent/src/tools/mod.rs

- `28: pub mod agent_tool;`
- `31: pub mod apply_patch;`
- `32: pub mod batch;`
- `34: pub mod browser;`
- `35: pub mod browser_open;`
- `36: pub mod catalog;`
- `38: pub mod codesearch;`
- `40: pub mod computer_control;`
- `42: mod computer_control_tests;`
- `43: pub mod config;`
- `45: pub mod content_search;`
- `46: mod context;`
- `47: pub mod cron_add;`
- `48: pub mod cron_list;`
- `49: pub mod cron_remove;`
- `50: pub mod cron_run;`
- `51: pub mod cron_runs;`
- `52: pub mod cron_update;`
- `53: mod decision;`
- `54: pub mod delegate;`
- `55: pub(crate) mod delegated_tools;`
- `56: pub(crate) mod discovery;`
- `58: pub mod enter_worktree;`
- `60: pub mod excel_control;`
- `62: pub mod word_control;`
- `64: mod word_control_tests;`
- `66: mod excel_control_tests;`
- `67: mod executor;`
- `69: pub mod exit_worktree;`
- `71: pub mod external_directory;`
- `73: pub mod file_edit;`
- `75: pub mod file_read;`
- `77: pub mod file_write;`
- `79: pub mod git_operations;`
- `81: pub mod glob;`
- `83: pub mod glob_search;`
- `85: pub mod grep;`
- `86: mod hooks;`
- `87: mod http_headers;`
- `88: pub mod http_request;`
- `89: pub mod image_asset_read;`
- `91: pub mod ls;`
- `92: pub mod mcp_common;`
- `93: pub mod mcp_inspect;`
- `94: pub(crate) mod mcp_resource;`
- `95: pub mod mcp_status;`
- `96: pub mod mcp_tool;`
- `97: pub mod memory_forget;`
- `98: pub mod memory_recall;`
- `99: pub mod memory_store;`
- `101: pub mod pdf_read;`
- `103: pub mod powershell;`
- `105: pub mod question;`
- `106: mod read_state;`
- `107: pub mod registry;`
- `108: pub mod remote_trigger;`
- `109: pub mod schedule;`
- `110: mod scheduler;`
- `112: pub mod screenshot;`
- `113: pub mod session_tool;`
- `115: pub mod shell;`
- `116: pub mod skill;`
- `117: pub mod sleep;`
- `118: pub mod thread_tool;`
- `119: pub mod todo;`
- `120: pub(crate) mod tool_search;`
- `121: mod toolset;`
- `122: pub mod traits;`
- `123: pub mod url_validation;`
- `124: pub mod web_fetch;`
- `125: pub mod web_search_tool;`
- `126: pub mod websearch;`
- `280: mod mcp_tool_tests;`
- `283: #[path = "mod_tests.rs"]`
- `284: mod coverage_tests;`
- `287: #[path = "tests/mod_tests.rs"]`
- `288: mod tests;`
- `295: mod catalog_tests;`
- `297: pub mod client_tool;`
- `299: mod client_tool_tests;`
- `302: pub(crate) mod mcp_catalog_refresh_tests;`

### crates/vw-agent/src/tools/pdf_read/mod.rs

- `47: mod render;`
- `436: #[path = "tests.rs"]`
- `437: mod tests;`

### crates/vw-agent/src/tools/pdf_read/render.rs

- `90: #[path = "render_tests.rs"]`
- `91: mod tests;`

### crates/vw-agent/src/tools/pdf_read/tests.rs

- `262: mod extraction {`

### crates/vw-agent/src/tools/powershell/mod.rs

- `157: mod tests;`

### crates/vw-agent/src/tools/question/mod.rs

- `653: include_str!("question.txt")`
- `781: #[path = "tests.rs"]`
- `782: mod tests;`

### crates/vw-agent/src/tools/read_state/mod.rs

- `250: mod tests;`

### crates/vw-agent/src/tools/registry/mod.rs

- `363: mod tests;`

### crates/vw-agent/src/tools/remote_trigger/mod.rs

- `92: #[path = "tests.rs"]`
- `93: mod tests;`

### crates/vw-agent/src/tools/schedule/mod.rs

- `391: #[path = "tests.rs"]`
- `392: mod tests;`

### crates/vw-agent/src/tools/scheduler/mod.rs

- `102: #[path = "tests.rs"]`
- `103: mod tests;`

### crates/vw-agent/src/tools/screenshot/mod.rs

- `865: #[path = "tests.rs"]`
- `866: mod tests;`

### crates/vw-agent/src/tools/session_tool/mod.rs

- `519: mod tests;`

### crates/vw-agent/src/tools/session_tool/tests.rs

- `106: let started_tx = std::sync::Mutex::new(Some(started_tx));`
- `107: let finish_rx = std::sync::Mutex::new(Some(finish_rx));`
- `346: let request_tx = std::sync::Mutex::new(Some(request_tx));`

### crates/vw-agent/src/tools/shell/ast/command_info/ast_walk.rs

- `404: #[path = "ast_walk_tests.rs"]`
- `405: mod ast_walk_tests;`

### crates/vw-agent/src/tools/shell/ast/command_info/mod.rs

- `8: mod ast_walk;`
- `9: mod scan;`
- `10: mod wrappers;`
- `62: mod tests;`

### crates/vw-agent/src/tools/shell/ast/command_info/scan.rs

- `723: #[path = "scan_tests.rs"]`
- `724: mod scan_tests;`

### crates/vw-agent/src/tools/shell/ast/command_info/wrappers.rs

- `201: #[path = "wrappers_tests.rs"]`
- `202: mod wrappers_tests;`

### crates/vw-agent/src/tools/shell/ast/mod.rs

- `6: pub mod command_info;`
- `7: pub mod nodes;`
- `8: pub mod parser;`
- `110: #[path = "parser_tests.rs"]`
- `111: mod parser_tests;`
- `114: #[path = "mod_tests.rs"]`
- `115: mod mod_tests;`
- `118: #[path = "nodes_tests.rs"]`
- `119: mod nodes_tests;`
- `122: #[path = "command_info_tests.rs"]`
- `123: mod command_info_tests;`

### crates/vw-agent/src/tools/shell/ast/parser.rs

- `3: use parking_lot::Mutex;`
- `6: use std::sync::{Arc, LazyLock};`
- `11: static LAST_PARSED_COMMAND: LazyLock<Mutex<Option<(String, BashAst)>>> =`
- `12: LazyLock::new(|| Mutex::new(None));`

### crates/vw-agent/src/tools/shell/compound/mod.rs

- `12: pub mod semantics;`
- `532: #[path = "tests.rs"]`
- `533: mod tests;`

### crates/vw-agent/src/tools/shell/compound/semantics.rs

- `100: #[path = "semantics_tests.rs"]`
- `101: mod semantics_tests;`

### crates/vw-agent/src/tools/shell/mod.rs

- `38: pub mod ast;`
- `39: pub mod compound;`
- `40: pub mod path;`
- `41: pub mod permissions;`
- `43: pub(crate) mod processes;`
- `46: pub mod readonly;`
- `47: pub mod sandbox;`
- `48: pub mod security;`
- `49: pub mod sed;`
- `51: pub mod session_tools;`
- `644: include_str!("./shell.txt")`
- `878: #[path = "tests.rs"]`
- `879: mod tests;`
- `882: mod environment_tests;`
- `885: #[path = "session_tools_tests.rs"]`
- `886: mod session_tools_tests;`
- `889: mod long_running_tests;`

### crates/vw-agent/src/tools/shell/path/mod.rs

- `9: mod extractors;`
- `373: #[path = "extractors_tests.rs"]`
- `374: mod extractors_tests;`
- `377: #[path = "mod_tests.rs"]`
- `378: mod mod_tests;`

### crates/vw-agent/src/tools/shell/permissions/mod.rs

- `9: pub mod mode;`
- `11: pub mod rules;`
- `13: pub mod sandbox_allow;`
- `15: pub mod warning;`
- `100: #[path = "rules_tests.rs"]`
- `101: mod rules_tests;`
- `104: #[path = "sandbox_allow_tests.rs"]`
- `105: mod sandbox_allow_tests;`
- `108: #[path = "mode_tests.rs"]`
- `109: mod mode_tests;`
- `112: #[path = "warning_tests.rs"]`
- `113: mod warning_tests;`
- `116: #[path = "mod_tests.rs"]`
- `117: mod mod_tests;`

### crates/vw-agent/src/tools/shell/processes.rs

- `3: use parking_lot::Mutex;`
- `7: use std::sync::{Arc, OnceLock};`
- `12: #[path = "output.rs"]`
- `13: mod output;`
- `14: #[path = "termination.rs"]`
- `15: pub(crate) mod termination;`
- `20: static HOST: OnceLock<Result<ShellProcesses, String>> = OnceLock::new();`
- `73: output: Arc<Mutex<OutputBuffer>>,`
- `78: type Records = Arc<Mutex<HashMap<String, Entry>>>;`
- `94: thread: Mutex<Option<JoinHandle<()>>>,`
- `109: Ok(Self { sender, records, stopping, thread: Mutex::new(Some(thread)) })`
- `366: let output = Arc::new(Mutex::new(OutputBuffer::default()));`
- `439: output: Arc<Mutex<OutputBuffer>>,`
- `461: #[path = "processes_tests.rs"]`
- `462: mod tests;`
- `465: #[path = "output_tests.rs"]`
- `466: mod output_tests;`
- `469: #[path = "termination_tests.rs"]`
- `470: mod termination_tests;`
- `473: #[path = "termination_windows_tests.rs"]`
- `474: mod termination_windows_tests;`

### crates/vw-agent/src/tools/shell/readonly/flag_validator.rs

- `41: #[path = "flag_validator_tests.rs"]`
- `42: mod flag_validator_tests;`

### crates/vw-agent/src/tools/shell/readonly/mod.rs

- `10: mod allowlist;`
- `11: mod flag_validator;`
- `140: #[path = "allowlist_tests.rs"]`
- `141: mod allowlist_tests;`
- `144: #[path = "mod_tests.rs"]`
- `145: mod mod_tests;`

### crates/vw-agent/src/tools/shell/sandbox/executor.rs

- `167: #[path = "executor_tests.rs"]`
- `168: mod executor_tests;`

### crates/vw-agent/src/tools/shell/sandbox/mod.rs

- `6: pub mod executor;`
- `7: pub mod policy;`
- `242: #[path = "sandbox_tests.rs"]`
- `243: mod sandbox_tests;`
- `246: #[path = "mod_tests.rs"]`
- `247: mod mod_tests;`

### crates/vw-agent/src/tools/shell/sandbox/policy.rs

- `86: #[path = "policy_tests.rs"]`
- `87: mod policy_tests;`

### crates/vw-agent/src/tools/shell/security/mod.rs

- `10: pub mod injection;`
- `11: pub mod obfuscation;`
- `12: mod validators;`
- `163: #[path = "validators_tests.rs"]`
- `164: mod validators_tests;`
- `167: #[path = "injection_tests.rs"]`
- `168: mod injection_tests;`
- `171: #[path = "obfuscation_tests.rs"]`
- `172: mod obfuscation_tests;`
- `175: #[path = "mod_tests.rs"]`
- `176: mod mod_tests;`
- `179: mod tests;`

### crates/vw-agent/src/tools/shell/security/validators/backslash.rs

- `9: use std::sync::LazyLock;`
- `11: static ESCAPED_WHITESPACE_RE: LazyLock<Regex> =`
- `12: LazyLock::new(|| Regex::new(r"\\[ \t\n]").expect("valid escaped whitespace regex"));`
- `13: static ESCAPED_OPERATOR_RE: LazyLock<Regex> =`
- `14: LazyLock::new(|| Regex::new(r"\\[;&|]").expect("valid escaped operator regex"));`
- `36: #[path = "backslash_tests.rs"]`
- `37: mod backslash_tests;`

### crates/vw-agent/src/tools/shell/security/validators/brace_expansion.rs

- `9: use std::sync::LazyLock;`
- `11: static BRACE_EXPANSION_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `34: #[path = "brace_expansion_tests.rs"]`
- `35: mod brace_expansion_tests;`

### crates/vw-agent/src/tools/shell/security/validators/comment_quote_desync.rs

- `29: #[path = "comment_quote_desync_tests.rs"]`
- `30: mod comment_quote_desync_tests;`

### crates/vw-agent/src/tools/shell/security/validators/control_chars.rs

- `27: #[path = "control_chars_tests.rs"]`
- `28: mod control_chars_tests;`

### crates/vw-agent/src/tools/shell/security/validators/dangerous_vars.rs

- `9: use std::sync::LazyLock;`
- `11: static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `23: static WARN_RE: LazyLock<Regex> =`
- `24: LazyLock::new(|| Regex::new(r"(?i)\$(SHLVL|SECONDS)\b").expect("valid fingerprint regex"));`
- `57: #[path = "dangerous_vars_tests.rs"]`
- `58: mod dangerous_vars_tests;`

### crates/vw-agent/src/tools/shell/security/validators/empty_command.rs

- `36: #[path = "empty_command_tests.rs"]`
- `37: mod empty_command_tests;`

### crates/vw-agent/src/tools/shell/security/validators/git_commit.rs

- `40: #[path = "git_commit_tests.rs"]`
- `41: mod git_commit_tests;`

### crates/vw-agent/src/tools/shell/security/validators/hash_comment.rs

- `27: #[path = "hash_comment_tests.rs"]`
- `28: mod hash_comment_tests;`

### crates/vw-agent/src/tools/shell/security/validators/heredoc.rs

- `9: use std::sync::LazyLock;`
- `11: static HEREDOC_RE: LazyLock<Regex> =`
- `12: LazyLock::new(|| Regex::new(r"<<-?\s*([A-Za-z_][A-Za-z0-9_]*)").expect("valid heredoc regex"));`
- `13: static QUOTED_HEREDOC_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `37: #[path = "heredoc_tests.rs"]`
- `38: mod heredoc_tests;`

### crates/vw-agent/src/tools/shell/security/validators/ifs_injection.rs

- `29: #[path = "ifs_injection_tests.rs"]`
- `30: mod ifs_injection_tests;`

### crates/vw-agent/src/tools/shell/security/validators/jq.rs

- `9: use std::sync::LazyLock;`
- `11: static DANGEROUS_JQ_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `38: #[path = "jq_tests.rs"]`
- `39: mod jq_tests;`

### crates/vw-agent/src/tools/shell/security/validators/malformed.rs

- `37: #[path = "malformed_tests.rs"]`
- `38: mod malformed_tests;`

### crates/vw-agent/src/tools/shell/security/validators/mod.rs

- `6: mod backslash;`
- `7: mod brace_expansion;`
- `8: mod comment_quote_desync;`
- `9: mod control_chars;`
- `10: mod dangerous_vars;`
- `11: mod empty_command;`
- `12: mod git_commit;`
- `13: mod hash_comment;`
- `14: mod heredoc;`
- `15: mod ifs_injection;`
- `16: mod jq;`
- `17: mod malformed;`
- `18: mod metacharacters;`
- `19: mod newline_injection;`
- `20: mod obfuscated_flags;`
- `21: mod proc_environ;`
- `22: mod quoted_newline;`
- `23: mod redirection;`
- `24: mod substitution;`
- `25: mod unicode_whitespace;`
- `26: mod zsh_dangerous;`
- `29: #[path = "metacharacters_tests.rs"]`
- `30: mod metacharacters_tests;`
- `32: #[path = "newline_injection_tests.rs"]`
- `33: mod newline_injection_tests;`
- `35: #[path = "obfuscated_flags_tests.rs"]`
- `36: mod obfuscated_flags_tests;`
- `38: #[path = "proc_environ_tests.rs"]`
- `39: mod proc_environ_tests;`
- `41: #[path = "quoted_newline_tests.rs"]`
- `42: mod quoted_newline_tests;`
- `44: #[path = "redirection_tests.rs"]`
- `45: mod redirection_tests;`
- `47: #[path = "substitution_tests.rs"]`
- `48: mod substitution_tests;`
- `50: mod tests;`
- `52: #[path = "unicode_whitespace_tests.rs"]`
- `53: mod unicode_whitespace_tests;`
- `55: #[path = "zsh_dangerous_tests.rs"]`
- `56: mod zsh_dangerous_tests;`
- `61: use std::sync::LazyLock;`
- `310: pub(super) fn contains_regex(regex: &LazyLock<Regex>, command: &str) -> bool {`

### crates/vw-agent/src/tools/shell/security/validators/obfuscated_flags.rs

- `9: use std::sync::LazyLock;`
- `11: static ANSI_C_FLAG_RE: LazyLock<Regex> = LazyLock::new(|| {`
- `14: static EMPTY_QUOTE_FLAG_RE: LazyLock<Regex> =`
- `15: LazyLock::new(|| Regex::new(r#"['"]{2}\s*--?[A-Za-z]"#).expect("valid empty quote flag regex"));`
- `16: static QUOTE_ADJACENT_FLAG_RE: LazyLock<Regex> = LazyLock::new(|| {`

### crates/vw-agent/src/tools/shell/security/validators/proc_environ.rs

- `9: use std::sync::LazyLock;`
- `11: static PROC_ENVIRON_RE: LazyLock<Regex> =`
- `12: LazyLock::new(|| Regex::new(r"/proc/(\*|self|\d+)/environ").expect("valid proc environ regex"));`

### crates/vw-agent/src/tools/shell/security/validators/tests.rs

- `7: use std::sync::LazyLock;`
- `10: static WORD_RE: LazyLock<Regex> =`
- `11: LazyLock::new(|| Regex::new(r"\bdanger\b").expect("valid test regex"));`

### crates/vw-agent/src/tools/shell/security/validators/zsh_dangerous.rs

- `9: use std::sync::LazyLock;`
- `11: static ZSH_DANGEROUS_RE: LazyLock<Regex> = LazyLock::new(|| {`

### crates/vw-agent/src/tools/shell/sed/mod.rs

- `6: mod parser;`
- `7: mod validation;`
- `13: #[path = "parser_tests.rs"]`
- `14: mod parser_tests;`
- `17: #[path = "validation_tests.rs"]`
- `18: mod validation_tests;`

### crates/vw-agent/src/tools/shell/session_tools.rs

- `8: use parking_lot::Mutex;`
- `10: use std::sync::OnceLock;`
- `29: fn metadata() -> &'static Mutex<HashMap<String, ProcessMetadata>> {`
- `30: static RECORDS: OnceLock<Mutex<HashMap<String, ProcessMetadata>>> = OnceLock::new();`
- `31: RECORDS.get_or_init(Mutex::default)`

### crates/vw-agent/src/tools/shell/termination.rs

- `8: #[path = "termination_windows.rs"]`
- `9: mod windows;`

### crates/vw-agent/src/tools/shell/tests.rs

- `15: mod tests {`

### crates/vw-agent/src/tools/skill/mod.rs

- `24: mod invocation;`
- `25: mod prompt;`
- `961: #[path = "tests.rs"]`
- `962: mod tests;`
- `965: #[path = "mod_tests.rs"]`
- `966: mod mod_tests;`
- `969: #[path = "invocation_tests.rs"]`
- `970: mod invocation_tests;`

### crates/vw-agent/src/tools/sleep/mod.rs

- `110: #[path = "tests.rs"]`
- `111: mod tests;`
- `114: #[path = "mod_tests.rs"]`
- `115: mod mod_tests;`

### crates/vw-agent/src/tools/thread_tool/mod.rs

- `569: mod tests;`
- `572: mod execution_tests;`

### crates/vw-agent/src/tools/todo/mod.rs

- `6: mod normalize;`
- `7: mod schema;`
- `8: mod store;`
- `11: #[path = "normalize_tests.rs"]`
- `12: mod normalize_tests;`
- `14: #[path = "schema_tests.rs"]`
- `15: mod schema_tests;`
- `17: #[path = "store_tests.rs"]`
- `18: mod store_tests;`
- `20: mod tests;`
- `135: include_str!("todoread.txt")`
- `253: include_str!("todowrite.txt")`

### crates/vw-agent/src/tools/todo/store.rs

- `3: use std::sync::LazyLock;`
- `4: use std::sync::Mutex;`
- `17: static TODOS: LazyLock<Mutex<HashMap<String, Vec<Todo>>>> =`
- `18: LazyLock::new(|| Mutex::new(HashMap::new()));`
- `20: pub(super) fn recover_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {`

### crates/vw-agent/src/tools/todo/store_tests.rs

- `7: use std::sync::Mutex;`
- `22: let values = Arc::new(Mutex::new(vec!["before"]));`

### crates/vw-agent/src/tools/tool_search.rs

- `9: use std::sync::Mutex;`
- `17: exposure: Mutex<ToolExposureState>,`
- `20: last_search: Mutex<Option<(String, Value)>>,`
- `32: exposure: Mutex::new(ToolExposureState::default()),`
- `35: last_search: Mutex::new(None),`
- `220: #[path = "tool_search_tests.rs"]`
- `221: mod tests;`

### crates/vw-agent/src/tools/toolset/mod.rs

- `684: #[path = "mod_tests.rs"]`
- `685: mod mod_tests;`
- `687: mod tests;`

### crates/vw-agent/src/tools/traits/mod.rs

- `649: #[path = "mod_tests.rs"]`
- `650: mod mod_tests;`
- `652: #[path = "tests.rs"]`
- `653: mod tests;`
- `656: mod dynamic_tools_tests;`

### crates/vw-agent/src/tools/url_validation/mod.rs

- `588: #[path = "mod_tests.rs"]`
- `589: mod mod_tests;`
- `591: #[path = "tests.rs"]`
- `592: mod tests;`

### crates/vw-agent/src/tools/web_fetch/mod.rs

- `825: include_str!("./web_fetch.txt")`
- `1084: #[path = "mod_tests.rs"]`
- `1085: mod mod_tests;`
- `1087: #[path = "tests.rs"]`
- `1088: mod tests;`

### crates/vw-agent/src/tools/web_fetch/tests.rs

- `18: mod tests {`

### crates/vw-agent/src/tools/web_search_tool/mod.rs

- `1050: #[path = "mod_tests.rs"]`
- `1051: mod mod_tests;`
- `1053: #[path = "tests.rs"]`
- `1054: mod tests;`

### crates/vw-agent/src/tools/websearch/mod.rs

- `409: include_str!("websearch.txt")`
- `723: #[path = "mod_tests.rs"]`
- `724: mod mod_tests;`
- `726: #[path = "tests.rs"]`
- `727: mod tests;`

### crates/vw-agent/src/tools/word_control_tests.rs

- `8: use parking_lot::RwLock;`
- `20: static CERTS: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();`
- `83: let policy = Arc::new(RwLock::new(ComputerControlPolicy::new(Default::default())));`
- `368: struct DocumentApproval(Arc<parking_lot::Mutex<Vec<(String, Value)>>>);`
- `389: let seen = Arc::new(parking_lot::Mutex::new(Vec::new()));`

### crates/vw-agent/src/tunnel/cloudflare/mod.rs

- `252: mod tests;`

### crates/vw-agent/src/tunnel/custom/mod.rs

- `53: /// 使用 Arc<Mutex> 包装，支持跨异步任务共享和并发访问`
- `276: mod tests;`

### crates/vw-agent/src/tunnel/mod.rs

- `5: mod cloudflare;`
- `6: mod custom;`
- `7: mod ngrok;`
- `8: mod none;`
- `9: mod tailscale;`
- `21: use tokio::sync::Mutex;`
- `55: pub(crate) type SharedProcess = Arc<Mutex<Option<TunnelProcess>>>;`
- `59: Arc::new(Mutex::new(None))`
- `142: mod tests;`

### crates/vw-agent/src/tunnel/ngrok/mod.rs

- `260: mod tests;`

### crates/vw-agent/src/tunnel/none/mod.rs

- `34: mod tests;`

### crates/vw-agent/src/tunnel/tailscale/mod.rs

- `34: use tokio::sync::Mutex;`
- `41: type SharedState = Arc<Mutex<Option<TailscaleState>>>;`
- `44: Arc::new(Mutex::new(None))`
- `206: /// 内部使用 SharedProcess（基于 Arc<Mutex<Option<TunnelProcess>>>），`
- `411: mod tests;`

### crates/vw-agent/src/tunnel/tests.rs

- `346: /// 当共享进程中没有子进程时（即 Arc<Mutex<Option<TunnelProcess>>> 值为 None），`

### crates/vw-agent/src/usage_bridge/events.rs

- `4: use std::{collections::BTreeMap, sync::OnceLock};`
- `19: static CHANNEL: OnceLock<tokio::sync::broadcast::Sender<CacheChange>> = OnceLock::new();`

### crates/vw-agent/src/usage_bridge/mod.rs

- `2: pub(crate) mod context;`
- `3: pub(crate) mod native;`
- `4: pub(crate) mod relay;`
- `5: pub(crate) mod events;`

### crates/vw-agent/src/util/mod.rs

- `36: pub mod archive;`
- `37: pub mod filesystem;`
- `38: pub mod format;`
- `39: pub mod locale;`
- `40: pub mod log;`
- `41: pub mod rpc;`
- `42: pub mod signal;`
- `45: #[path = "archive_tests.rs"]`
- `46: mod archive_tests;`
- `48: #[path = "filesystem_tests.rs"]`
- `49: mod filesystem_tests;`
- `51: #[path = "locale_tests.rs"]`
- `52: mod locale_tests;`
- `54: #[path = "rpc_tests.rs"]`
- `55: mod rpc_tests;`
- `57: #[path = "signal_tests.rs"]`
- `58: mod signal_tests;`
- `60: mod tests;`

### crates/vw-agent/src/workflow/code_runner.rs

- `369: #[path = "code_runner_tests.rs"]`
- `370: mod code_runner_tests;`

### crates/vw-agent/src/workflow/code_runner_http.rs

- `432: #[path = "code_runner_http_tests.rs"]`
- `433: mod code_runner_http_tests;`

### crates/vw-agent/src/workflow/conditions.rs

- `128: #[path = "conditions_tests.rs"]`
- `129: mod conditions_tests;`

### crates/vw-agent/src/workflow/dsml_tests.rs

- `6: Arc, Mutex,`
- `13: messages: Mutex<Vec<Vec<ChatMessage>>>,`
- `95: messages: Mutex::new(Vec::new()),`

### crates/vw-agent/src/workflow/iteration_parallel_tests.rs

- `159: let events = Arc::new(std::sync::Mutex::new(Vec::new()));`
- `264: let events = Arc::new(std::sync::Mutex::new(Vec::new()));`

### crates/vw-agent/src/workflow/mod.rs

- `5: mod code_runner;`
- `6: mod code_runner_http;`
- `7: mod conditions;`
- `8: mod model;`
- `10: pub(crate) mod native;`
- `11: mod runner;`
- `12: mod template;`
- `13: mod variables;`
- `26: mod dsml_tests;`
- `28: #[path = "tests.rs"]`
- `29: mod tests;`
- `32: pub mod video_edit;`
- `35: pub(crate) mod store;`
- `37: pub mod templates;`
- `39: mod templates_tests;`
- `41: #[path = "video_edit/budget.rs"]`
- `42: pub mod budget;`

### crates/vw-agent/src/workflow/model.rs

- `107: #[path = "model_tests.rs"]`
- `108: mod model_tests;`

### crates/vw-agent/src/workflow/native.rs

- `14: #[path = "native_agent.rs"]`
- `15: mod agent;`
- `16: #[path = "native_video_progress.rs"]`
- `17: mod video_progress;`
- `270: let progress = Arc::new(std::sync::Mutex::new(video_progress::Progress::new(`
- `348: #[path = "native_tests.rs"]`
- `349: mod tests;`

### crates/vw-agent/src/workflow/native_tests.rs

- `3: use std::sync::{Arc, Mutex};`
- `64: let events = Arc::new(Mutex::new(Vec::new()));`
- `102: let events = Arc::new(Mutex::new(vec![]));`
- `350: include_str!("../../testdata/workflow/talking_head_cloud.yaml").into(),`

### crates/vw-agent/src/workflow/native_video_progress.rs

- `171: #[path = "native_video_progress_tests.rs"]`
- `172: mod tests;`

### crates/vw-agent/src/workflow/runner.rs

- `3468: #[path = "runner_tests.rs"]`
- `3469: mod runner_tests;`
- `3472: #[path = "iteration_parallel_tests.rs"]`
- `3473: mod iteration_parallel_tests;`
- `3475: #[path = "iteration_parallel.rs"]`
- `3476: mod iteration_parallel;`

### crates/vw-agent/src/workflow/runner_tests.rs

- `745: calls: std::sync::Mutex<Vec<String>>,`

### crates/vw-agent/src/workflow/store/postgres.rs

- `264: #[path = "postgres_tests.rs"]`
- `265: mod postgres_tests;`

### crates/vw-agent/src/workflow/store.rs

- `13: mod error;`
- `18: mod postgres;`
- `19: mod template_records;`
- `338: #[path = "store_tests.rs"]`
- `339: mod store_tests;`

### crates/vw-agent/src/workflow/template.rs

- `7: use std::sync::LazyLock;`
- `9: static TEMPLATE_RE: LazyLock<Regex> =`
- `10: LazyLock::new(|| Regex::new(r"\{\{#([^#]+)#\}\}").expect("valid workflow template regex"));`
- `48: #[path = "template_tests.rs"]`
- `49: mod template_tests;`

### crates/vw-agent/src/workflow/tests.rs

- `15: use std::sync::{Arc, Mutex};`
- `87: calls: Mutex<Vec<String>>,`
- `124: calls: Mutex<Vec<String>>,`
- `159: requests: Mutex<Vec<WorkflowKnowledgeRequest>>,`
- `179: requests: Mutex<Vec<WorkflowToolRequest>>,`
- `220: requests: Mutex<Vec<WorkflowAgentRequest>>,`
- `265: states: Mutex<BTreeMap<String, WorkflowPauseState>>,`
- `598: let provider = Arc::new(ModelFallbackProvider { calls: Mutex::new(Vec::new()) });`
- `1493: calls: Mutex::new(Vec::new()),`
- `1533: calls: Mutex::new(Vec::new()),`
- `1575: calls: Mutex::new(Vec::new()),`
- `1608: calls: Mutex::new(Vec::new()),`
- `1693: calls: Mutex::new(Vec::new()),`
- `1732: calls: Mutex::new(Vec::new()),`
- `1767: calls: Mutex::new(Vec::new()),`
- `1866: let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });`
- `1970: let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });`
- `2039: let knowledge_provider = Arc::new(FakeKnowledgeProvider { requests: Mutex::new(Vec::new()) });`
- `2454: let tool_provider = Arc::new(FakeWorkflowToolProvider { requests: Mutex::new(Vec::new()) });`
- `2524: requests: Mutex::new(Vec::new()),`
- `2621: let agent_provider = Arc::new(FakeWorkflowAgentProvider { requests: Mutex::new(Vec::new()) });`
- `2622: let tool_provider = Arc::new(FakeWorkflowToolProvider { requests: Mutex::new(Vec::new()) });`
- `2663: requests: Mutex::new(Vec::new()),`
- `2666: requests: Mutex::new(Vec::new()),`
- `2695: requests: Mutex::new(Vec::new()),`
- `2908: let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });`
- `2940: let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });`
- `2981: let pause_store = Arc::new(FakeWorkflowPauseStore { states: Mutex::new(BTreeMap::new()) });`
- `3324: calls: Mutex<Vec<String>>,`
- `3376: calls: Mutex::new(vec![]),`
- `3391: workflow_yaml: Some(include_str!("../../testdata/workflow/talking_head.yaml").into()),`
- `3451: let graph = parse_workflow_yaml(include_str!("../../testdata/workflow/talking_head.yaml"))`

### crates/vw-agent/src/workflow/variables.rs

- `109: #[path = "variables_tests.rs"]`
- `110: mod variables_tests;`

### crates/vw-agent/src/workflow/video_edit/apply_test_cloud.rs

- `7: sync::{Arc, Mutex},`
- `29: pub(super) state: Arc<Mutex<Remote>>,`
- `38: State(state): State<Arc<Mutex<Remote>>>,`
- `153: let state = Arc::new(Mutex::new(Remote::default()));`

### crates/vw-agent/src/workflow/video_edit/assets_tests.rs

- `9: Arc, Mutex,`
- `27: state: Arc<Mutex<Remote>>,`
- `36: let state = Arc::new(Mutex::new(Remote::default()));`

### crates/vw-agent/src/workflow/video_edit/budget.rs

- `14: progress: Arc<std::sync::Mutex<crate::workflow::video_edit::assets::VideoAssetProgress>>,`
- `19: assets: Arc<std::sync::Mutex<std::collections::BTreeMap<String, VideoEditPermit>>>,`
- `146: #[path = "budget_tests.rs"]`
- `147: mod tests;`

### crates/vw-agent/src/workflow/video_edit/cloud.rs

- `491: #[path = "cloud_assets.rs"]`
- `492: mod asset_transport;`
- `493: #[path = "cloud_delivery.rs"]`
- `494: mod delivery_transport;`
- `495: #[path = "cloud_memory.rs"]`
- `496: mod memory;`
- `497: #[path = "cloud_prepare.rs"]`
- `498: mod prepare_transport;`
- `499: #[path = "cloud_transcript.rs"]`
- `500: mod transcript_transport;`
- `502: #[path = "cloud_probe.rs"]`
- `503: pub(super) mod probe;`

### crates/vw-agent/src/workflow/video_edit/cloud_delivery.rs

- `280: #[path = "cloud_delivery_read.rs"]`
- `281: mod read;`
- `282: #[path = "cloud_delivery_write.rs"]`
- `283: mod write;`

### crates/vw-agent/src/workflow/video_edit/cloud_memory.rs

- `6: const CALLER_PROCESS: &str = include_str!("cloud_caller.py");`
- `169: #[path = "cloud_memory_tests.rs"]`
- `170: mod tests;`

### crates/vw-agent/src/workflow/video_edit/cloud_tests.rs

- `335: use std::sync::{Arc, Mutex};`
- `336: struct Audit(Arc<Mutex<Vec<String>>>);`
- `360: let events = Arc::new(Mutex::new(vec![]));`

### crates/vw-agent/src/workflow/video_edit/e2e_test_cloud.rs

- `15: Arc, Mutex,`
- `32: pub capture: Mutex<Vec<(String, String)>>,`
- `43: 300 => include_str!("../../../testdata/workflow/video_edit/synthetic_300.json"),`
- `44: 600 => include_str!("../../../testdata/workflow/video_edit/synthetic_600.json"),`
- `51: cloud_receipt: serde_json::from_str(include_str!(`
- `131: pub writes: Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>,`

### crates/vw-agent/src/workflow/video_edit/entry.rs

- `12: use tokio::sync::Mutex;`
- `80: state: Mutex::new(State::default()),`
- `122: state: Mutex<State>,`

### crates/vw-agent/src/workflow/video_edit/entry_contract.rs

- `5: pub const DEFAULT_YAML: &str = include_str!("../../../resources/workflows/cloud_video_edit.yaml");`
- `126: #[path = "entry_presentation_tests.rs"]`
- `127: mod presentation_tests;`

### crates/vw-agent/src/workflow/video_edit/entry_test_transport.rs

- `15: sync::{Arc, LazyLock, Mutex},`
- `19: static EXTERNALS: LazyLock<Mutex<BTreeMap<String, ExternalFactory>>> =`
- `20: LazyLock::new(Default::default);`
- `26: static SESSIONS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);`
- `53: static PROJECTS: LazyLock<Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>> =`
- `54: LazyLock::new(Default::default);`
- `57: writes: Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>,`

### crates/vw-agent/src/workflow/video_edit/mod.rs

- `1: mod cloud;`
- `2: pub(crate) mod cloud_failure;`
- `4: mod command;`
- `5: mod types;`
- `9: mod command_tests;`
- `12: mod cloud_tests;`
- `15: mod cloud_prepare_tests;`
- `16: pub mod manifest;`
- `18: mod manifest_tests;`
- `19: pub mod prepare;`
- `21: mod prepare_tests;`
- `22: pub mod segments;`
- `24: mod segments_tests;`
- `25: pub mod transcript;`
- `27: mod transcript_tests;`
- `31: pub mod assets;`
- `33: mod assets_tests;`
- `35: mod cloud_assets_tests;`
- `36: pub mod merge;`
- `38: mod merge_tests;`
- `39: pub mod planning;`
- `41: mod planning_tests;`
- `43: pub mod apply;`
- `44: mod apply_journal;`
- `46: mod apply_tests;`
- `47: pub mod preview;`
- `49: mod preview_tests;`
- `53: mod apply_test_cloud;`
- `56: mod cloud_delivery_tests;`
- `59: mod entry_tests;`
- `61: pub(crate) mod entry;`
- `62: mod entry_contract;`
- `70: pub(crate) mod entry_test_transport;`
- `73: mod e2e_tests;`
- `76: mod e2e_test_cloud;`

### crates/vw-agent/src/workflow/video_edit/prepare_tests.rs

- `4: use std::sync::{Arc, Mutex};`
- `22: remote: Arc<Mutex<Remote>>,`

### crates/vw-agent/src/worktree/cleanup_host.rs

- `3: use std::{collections::HashSet, sync::LazyLock, time::Duration};`
- `7: static CREATED: LazyLock<Notify> = LazyLock::new(Notify::new);`

### crates/vw-agent/src/worktree/environment_tests.rs

- `179: use std::sync::{Arc, Mutex};`
- `188: let observed = Arc::new(Mutex::new(vec![]));`
- `238: use std::sync::{Arc, Mutex};`
- `247: let failures = Arc::new(Mutex::new(vec![]));`

### crates/vw-agent/src/worktree/management.rs

- `109: static SECRET: std::sync::LazyLock<String> =`
- `110: std::sync::LazyLock::new(|| uuid::Uuid::new_v4().to_string());`

### crates/vw-agent/src/worktree/mod.rs

- `28: pub mod worktree;`
- `37: mod tests;`

### crates/vw-agent/src/worktree/recovery.rs

- `377: #[path = "recovery_restore.rs"]`
- `378: mod restoration;`

### crates/vw-agent/src/worktree/recovery_restore_tests.rs

- `12: static GIT_PROCESS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/worktree/recovery_tests.rs

- `11: static GIT_PROCESS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/src/worktree/worktree.rs

- `6: #[path = "worktree_event.rs"]`
- `7: pub mod event;`
- `9: #[path = "worktree_naming.rs"]`
- `10: mod naming;`
- `12: #[path = "worktree_native.rs"]`
- `13: mod native;`
- `14: #[path = "worktree_ops.rs"]`
- `15: mod ops;`
- `16: #[path = "worktree_types.rs"]`
- `17: mod types;`
- `27: #[path = "worktree_event_tests.rs"]`
- `28: mod worktree_event_tests;`
- `30: #[path = "worktree_naming_tests.rs"]`
- `31: mod worktree_naming_tests;`
- `33: #[path = "worktree_native_tests.rs"]`
- `34: mod worktree_native_tests;`
- `36: #[path = "worktree_ops_tests.rs"]`
- `37: mod worktree_ops_tests;`
- `39: #[path = "worktree_types_tests.rs"]`
- `40: mod worktree_types_tests;`
- `43: #[path = "registry.rs"]`
- `44: pub mod registry;`
- `46: #[path = "registry_tests.rs"]`
- `47: mod registry_tests;`
- `50: #[path = "creation_policy.rs"]`
- `51: mod creation_policy;`
- `53: #[path = "creation_policy_tests.rs"]`
- `54: mod creation_policy_tests;`
- `57: #[path = "environment.rs"]`
- `58: pub mod environment;`
- `60: #[path = "environment_tests.rs"]`
- `61: mod environment_tests;`
- `64: #[path = "recovery.rs"]`
- `65: pub mod recovery;`
- `67: #[path = "recovery_tests.rs"]`
- `68: mod recovery_tests;`
- `71: #[path = "recovery_restore_tests.rs"]`
- `72: mod recovery_restore_tests;`
- `75: #[path = "cleanup.rs"]`
- `76: pub mod cleanup;`
- `78: #[path = "cleanup_tests.rs"]`
- `79: mod cleanup_tests;`
- `82: #[path = "management.rs"]`
- `83: pub mod management;`
- `86: #[path = "cleanup_flow_tests.rs"]`
- `87: mod cleanup_flow_tests;`
- `90: #[path = "cleanup_host.rs"]`
- `91: pub mod cleanup_host;`
- `94: #[path = "management_e2e_tests.rs"]`
- `95: mod management_e2e_tests;`

### crates/vw-agent/src/worktree/worktree_naming.rs

- `2: use std::sync::LazyLock;`
- `47: static RE_NON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").unwrap());`
- `48: static RE_LEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-+").unwrap());`
- `49: static RE_TRAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-+$").unwrap());`

### crates/vw-agent/src/worktree/worktree_native_tests.rs

- `17: static WORKTREE_PROJECT_CONTEXT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());`

### crates/vw-agent/tests/acp_gemini_chat.rs

- `41: let received = Arc::new(parking_lot::Mutex::new(Vec::<Value>::new()));`

### crates/vw-agent/tests/model_protocol_014_messages.rs

- `6: #[path = "../src/model_relay/messages_request_http_tests.rs"]`
- `7: mod messages_request_http_tests;`
- `54: #[path = "../src/model_relay/chat.rs"]`
- `55: mod chat;`
- `57: #[path = "../src/model_relay/messages.rs"]`
- `58: mod messages;`
- `59: #[path = "../src/model_relay/messages_request_core_tests.rs"]`
- `60: mod request_core_tests;`

### crates/vw-agent/tests/model_protocol_015_messages.rs

- `4: #[path = "../src/model_relay/messages_thinking_http_tests.rs"]`
- `5: mod wire;`

## 后续验收（本轮不执行）

各领域沿编号卡迁移原测试和断言；权限/审批覆盖拒绝与取消，记忆/存储覆盖后端及共享数据库，会话覆盖中断恢复，提供商覆盖流式/工具调用与模型协议，工具/MCP覆盖授权及子进程退出，调度/目标/工作流覆盖执行注入与恢复，技能/插件覆盖资源路径，渠道覆盖消息路由，计算机控制覆盖平台授权，网关覆盖租户/错误映射，agent编排覆盖端到端状态推进。
全部编译、平台条件、feature组合、单元/集成、资源实际加载、UI与性能均未验证。只在以后获授权时按仓库槽位协议进行；本审计完成仅表示实施计划准备完毕。

## 当前基础库真实导出核对

以下仅列已在磁盘存在且为 vw-agent 路径依赖的库；不把计划目标当现有接口。

- `crates/libs/acp-access-policy/src/lib.rs`：`pub mod access_policy;`；`pub mod codex_compat;`；`pub mod session_mode_preference;`；`pub use access_policy::{AccessPolicy, AccessPolicyReason, FullAccessSupport};`
- `crates/libs/acp-agent-registry/src/lib.rs`：`pub mod agent_registry;`；`pub use agent_registry::{`；`pub use install::install_builtin_agent;`；`pub use managed_runtime::{reviewed_agent_command, verify_reviewed_agent_executable};`
- `crates/libs/acp-cli-model/src/lib.rs`：`pub mod cli;`；`pub mod cli_core;`；`pub mod cli_public;`；`pub mod version;`；`pub use cli_core::*;`；`pub use cli_public::*;`
- `crates/libs/acp-client/src/lib.rs`：`pub use client::{AcpClient, ConnectionDiagnostic};`
- `crates/libs/acp-config/src/lib.rs`：`pub mod config;`；`pub mod mcp_servers;`；`pub use config::*;`；`pub use mcp_servers::*;`
- `crates/libs/acp-host-io/src/lib.rs`：`pub mod filesystem;`；`pub mod permission_prompt;`；`pub mod permissions;`；`pub mod spawn_command_options;`；`pub mod terminal;`
- `crates/libs/acp-lifecycle/src/lib.rs`：`pub mod lifecycle;`；`pub use lifecycle::{`
- `crates/libs/acp-server/src/lib.rs`：`pub mod server;`；`pub use server::*;`
- `crates/libs/acp-storage/src/lib.rs`：`pub mod persisted_key_policy;`；`pub mod session_conversation_model;`；`pub mod session_events;`；`pub mod session_persistence;`；`pub use persisted_key_policy::*;`；`pub use session_conversation_model::*;`；`pub use session_events::*;`；`pub use session_persistence::*;`
- `crates/libs/acp-types/src/lib.rs`：`pub mod types;`；`pub mod prompt_content;`；`pub mod acp_jsonrpc;`；`pub mod agent_session_id;`；`pub mod runtime_session_id;`；`pub mod session_mounts;`；`pub mod error;`；`pub mod errors;`；`pub mod acp_error_shapes;`；`pub mod jsonrpc_error;`；`pub mod error_normalization;`；`pub mod session_event_log;`；`pub use types::*;`；`pub use prompt_content::{`；`pub use acp_jsonrpc::{`；`pub use agent_session_id::{`；`pub use runtime_session_id::{`；`pub use session_mounts::*;`；`pub use error::AcpError;`；`pub use errors::{`；`pub use acp_error_shapes::{`；`pub use jsonrpc_error::{`；`pub use error_normalization::{`；`pub use session_event_log::{`
- `crates/libs/agent-async/src/lib.rs`：`pub mod abort;`；`pub mod eventloop;`；`pub mod lock;`；`pub mod queue;`；`pub mod task_set;`；`pub mod timeout;`
- `crates/libs/agent-bus/src/lib.rs`：`pub struct Definition {`；`pub mod event {`；`pub struct GlobalEvent {`；`pub fn define(r#type: &'static str) -> Definition {`；`pub fn publish<T: Serialize>(`；`pub fn publish_value(def: Definition, properties: Value, directory: Option<String>) {`；`pub fn subscribe(def: Definition, callback: impl Fn(Value) + Send + Sync + 'static) -> Unsubscribe {`；`pub fn subscribe_all(callback: impl Fn(Value) + Send + Sync + 'static) -> Unsubscribe {`；`pub fn once(def: Definition, callback: impl Fn(Value) -> bool + Send + Sync + 'static) {`；`pub fn global_subscribe(callback: impl Fn(GlobalEvent) + Send + Sync + 'static) -> Unsubscribe {`
- `crates/libs/agent-context/src/lib.rs`：`pub mod context;`；`pub mod defer;`；`pub mod r#fn;`；`pub mod iife;`；`pub mod lazy;`；`pub mod proxied;`；`pub mod scrap;`
- `crates/libs/agent-database/src/lib.rs`：`pub mod global_database;`；`pub use global_database::*;`
- `crates/libs/agent-identity/src/lib.rs`：`pub use model::{`；`pub use normalize::normalize_aieos_identity;`；`pub use prompt::aieos_to_system_prompt;`
- `crates/libs/agent-log/src/lib.rs`：`pub enum Level {`；`pub struct InitOptions {`；`pub fn file() -> Option<PathBuf> {`；`pub fn init(options: InitOptions) {`；`pub struct Logger {`；`pub static DEFAULT: LazyLock<Logger> = LazyLock::new(|| {`；`pub fn create(tags: Option<Map<String, Value>>) -> Logger {`；`pub struct Timer {`
- `crates/libs/agent-paths/src/lib.rs`：`pub const APP: &str = config_runtime_paths::APP_DIR_NAME;`；`pub const CACHE_VERSION: &str = "21";`；`pub struct GlobalPaths {`；`pub fn paths() -> &'static GlobalPaths {`
- `crates/libs/agent-text/src/lib.rs`：`pub mod color;`；`pub mod keybind;`；`pub mod token;`；`pub mod wildcard;`
- `crates/libs/api-acp-model/src/lib.rs`：`pub mod acp_model_source;`；`pub mod acp_session;`
- `crates/libs/api-agent-scheduler/src/lib.rs`：`pub use tasks::*;`；`pub use capabilities::*;`；`pub use events::*;`；`pub use commands::*;`
- `crates/libs/api-app-auth/src/lib.rs`：`pub mod agent_apps_auth;`；`pub use agent_apps_auth::*;`
- `crates/libs/api-app-identity/src/lib.rs`：`pub struct CustomIconEntry {`；`pub struct AddCustomIconRequest {`；`pub struct AddCustomIconResult {`；`pub struct RenameCustomIconRequest {`；`pub enum AgentAppsErrorCode {`；`pub struct AgentAppsErrorResponse {`；`pub mod catalog;`；`pub use catalog::*;`；`pub mod installations;`；`pub use installations::*;`；`pub mod launch;`；`pub use launch::*;`；`pub mod preferences;`；`pub use preferences::*;`；`pub mod read;`；`pub use read::*;`
- `crates/libs/api-app-provider-base/src/lib.rs`：`pub enum ProviderProtocol {`；`pub struct AppProviderWrite {`；`pub struct AppProviderSummary {`；`pub struct ProviderLaunchRequest {`；`pub mod basic;`；`pub mod claude;`；`pub mod claude_login;`；`pub mod create;`；`pub mod update;`；`pub mod validation;`
- `crates/libs/api-app-provider-profiles/src/lib.rs`：`pub mod agy;`；`pub mod claude_desktop;`；`pub mod codex_catalog;`；`pub mod desktop_mapping;`；`pub mod gemini;`
- `crates/libs/api-chat/src/lib.rs`：`pub mod chat;`；`pub use chat::*;`
- `crates/libs/api-chat-identity/src/lib.rs`：`pub enum ReasoningEffortDto {`；`pub struct SessionAgentIdentity {`
- `crates/libs/api-file/src/lib.rs`：`pub mod file;`；`pub use file::*;`
- `crates/libs/api-git/src/lib.rs`：`pub mod git;`；`pub use git::*;`
- `crates/libs/api-knowledge/src/lib.rs`：`pub use dataset::{`；`pub use document::{`；`pub use retrieval::{`
- `crates/libs/api-model-catalog/src/lib.rs`：`pub struct ModelCatalogIdentity {`；`pub struct CodexNativeModel {`；`pub struct CodexReasoningLevel {`；`pub struct SavedCodexCatalog {`；`pub struct ScopedModelCatalogIdentity {`；`pub enum ModelCapabilityState {`；`pub struct ModelMetadataDto {`；`pub struct ModelTokenCost {`；`pub enum ModelMetadataField {`；`pub enum ModelMetadataSource {`；`pub struct ModelMetadataProvenance {`；`pub struct ModelCatalogEntry {`；`pub struct ModelCatalogQuery {`；`pub struct ModelCatalogReadResponse {`；`pub struct ModelCatalogEditRequest {`；`pub enum ModelCatalogEdit {`；`pub struct ModelCatalogDefaultRequest {`；`pub enum ModelCatalogDefaultChange {`；`pub struct ModelCatalogWriteResponse {`；`pub struct ModelCatalogFillPreviewRequest {`；`pub struct ModelCatalogFillPreviewResponse {`；`pub enum ModelCatalogCacheStatus {`；`pub struct ModelCatalogFillCommitRequest {`；`pub struct ModelDiscoveryRequest {`；`pub enum ModelDiscoveryMethod {`；`pub struct ModelDiscoveryStatus {`；`pub enum ModelDiscoveryState {`；`pub enum ModelDiscoveryFailure {`；`pub struct ModelCatalogProject {`；`pub struct ModelCatalogPreset {`；`pub struct ModelCatalogTemplates {`；`pub struct ModelCatalogCapabilities {`
- `crates/libs/api-project/src/lib.rs`：`pub mod project;`；`pub use project::*;`
- `crates/libs/api-session/src/lib.rs`：`pub mod session;`；`pub use session::*;`
- `crates/libs/api-task/src/lib.rs`：`pub use task_execution::*;`；`pub use task_plan::*;`；`pub use task_pool::*;`
- `crates/libs/api-task-lifecycle/src/lib.rs`：`pub mod task_lifecycle;`；`pub use task_lifecycle::{`
- `crates/libs/api-todo/src/lib.rs`：`pub mod todo;`；`pub use todo::*;`
- `crates/libs/api-tools/src/lib.rs`：`pub mod tools;`；`pub use tools::*;`
- `crates/libs/api-transport/src/lib.rs`：`pub mod common;`；`pub mod error;`；`pub mod id;`；`pub use common::*;`；`pub use error::*;`；`pub use id::*;`
- `crates/libs/api-workflow/src/lib.rs`：`pub use workflow::*;`
- `crates/libs/api-worktree/src/lib.rs`：`pub mod worktree;`；`pub use worktree::*;`
- `crates/libs/auth-file-store/src/lib.rs`：`pub use store::{all_from, get_from, remove_from, resolve_filepath, set_to};`
- `crates/libs/auth-types/src/lib.rs`：`pub const OAUTH_DUMMY_KEY: &str = "vibewindow-oauth-dummy-key";`；`pub struct OauthInfo {`；`pub struct ApiInfo {`；`pub struct WellKnownInfo {`；`pub enum Info {`
- `crates/libs/build-identity/src/lib.rs`：`pub use build_identity::{current_executable_build_id, executable_build_id};`
- `crates/libs/canary-guard/src/lib.rs`：`pub struct CanaryGuard {`
- `crates/libs/command-arity/src/lib.rs`：`pub fn prefix(tokens: &[&str]) -> Vec<String> {`
- `crates/libs/config-acp-install/src/lib.rs`：`pub enum AcpInstallPlatform {`；`pub fn acp_install_command(agent: &str) -> Option<(&'static str, &'static [&'static str])> {`；`pub fn acp_install_command_for_platform(`
- `crates/libs/config-agent/src/lib.rs`：`pub use config_agent_profile::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};`；`pub const DEFAULT_MAX_ITERATIONS: usize = 50_000;`；`pub struct WorkspacesConfig {`；`pub enum DelegateCapabilityPolicy {`；`pub struct AgentDefinitionConfig {`；`pub type DelegateAgentConfig = AgentDefinitionConfig;`；`pub enum BuiltinAgentKind {`；`pub struct BuiltinAgentSpec {`；`pub const BUILTIN_AGENT_SPECS: [BuiltinAgentSpec; 5] = [`；`pub fn is_retired_builtin_agent_key(key: &str) -> bool {`；`pub fn builtin_agent_spec(key: &str) -> Option<&'static BuiltinAgentSpec> {`；`pub fn builtin_agent_keys() -> Vec<&'static str> {`；`pub fn builtin_agent_config(key: &str) -> Option<AgentDefinitionConfig> {`；`pub fn merged_agent_configs(`；`pub fn effective_main_agent_config(`；`pub fn ensure_main_agent_config(`
- `crates/libs/config-agent-profile/src/lib.rs`：`pub enum AgentRuntime {`；`pub struct AgentModelReference {`；`pub enum AgentDefault<T> {`；`pub struct AgentDefaults {`
- `crates/libs/config-aggregate/src/lib.rs`：`pub struct AcpAgentConfig {`；`pub struct Config {`
- `crates/libs/config-automation/src/lib.rs`：`pub struct SchedulerConfig {`；`pub struct CronConfig {`；`pub struct HeartbeatConfig {`
- `crates/libs/config-channel-services/src/lib.rs`：`pub mod clawdtalk;`；`pub mod email;`；`pub mod lark;`；`pub mod other;`；`pub mod wechat;`；`pub mod wecom;`；`pub use clawdtalk::*;`；`pub use email::*;`；`pub use lark::*;`；`pub use other::*;`；`pub use wechat::*;`；`pub use wecom::*;`
- `crates/libs/config-channel-types/src/lib.rs`：`pub enum StreamMode {`；`pub enum GroupReplyMode {`；`pub struct GroupReplyConfig {`；`pub fn resolve_group_reply_mode(`；`pub fn clone_group_reply_allowed_sender_ids(group_reply: Option<&GroupReplyConfig>) -> Vec<String> {`；`pub struct TelegramConfig {`；`pub struct DiscordConfig {`；`pub struct SlackConfig {`；`pub struct MattermostConfig {`；`pub struct WebhookConfig {`；`pub struct IMessageConfig {`
- `crates/libs/config-channels/src/lib.rs`：`pub mod chat;`；`pub use chat::*;`
- `crates/libs/config-computer-control/src/lib.rs`：`pub struct ComputerControlConfig {`；`pub struct ComputerControlUserConfig {`；`pub enum GrantDecision {`
- `crates/libs/config-gateway/src/lib.rs`：`pub const DEFAULT_GATEWAY_PORT: u16 = 42_617;`；`pub const DEFAULT_GATEWAY_PORT: u16 = 14_682;`；`pub struct GatewayConfig {`；`pub struct GatewaySkey {`；`pub struct NodeControlConfig {`；`pub struct TunnelConfig {`；`pub struct CloudflareTunnelConfig {`；`pub struct TailscaleTunnelConfig {`；`pub struct NgrokTunnelConfig {`；`pub struct CustomTunnelConfig {`
- `crates/libs/config-hooks/src/lib.rs`：`pub struct HooksConfig {`；`pub struct BuiltinHooksConfig {`；`pub struct PluginHooksConfig {`
- `crates/libs/config-memory/src/lib.rs`：`pub struct StorageConfig {`；`pub struct StorageProviderSection {`；`pub struct StorageProviderConfig {`；`pub struct QdrantConfig {`；`pub struct MemoryConfig {`
- `crates/libs/config-observability/src/lib.rs`：`pub struct ObservabilityConfig {`
- `crates/libs/config-plugins/src/lib.rs`：`pub struct PluginsConfig {`
- `crates/libs/config-provider/src/lib.rs`：`pub enum CompatibleApiMode {`；`pub enum ProviderApiMode {`；`pub struct ModelProviderConfig {`；`pub struct ProviderConfig {`
- `crates/libs/config-proxy/src/lib.rs`：`pub const SUPPORTED_PROXY_SERVICE_KEYS: &[&str] = &[`；`pub const SUPPORTED_PROXY_SERVICE_SELECTORS: &[&str] =`；`pub enum ProxyScope {`；`pub struct ProxyConfig {`；`pub fn normalize_proxy_url_option(raw: Option<&str>) -> Option<String> {`；`pub fn normalize_no_proxy_list(values: Vec<String>) -> Vec<String> {`；`pub fn normalize_service_list(values: Vec<String>) -> Vec<String> {`；`pub fn normalize_comma_values(values: Vec<String>) -> Vec<String> {`；`pub fn is_supported_proxy_service_selector(selector: &str) -> bool {`；`pub fn service_selector_matches(selector: &str, service_key: &str) -> bool {`
- `crates/libs/config-reliability/src/lib.rs`：`pub struct ReliabilityConfig {`
- `crates/libs/config-routing/src/lib.rs`：`pub struct ModelRouteConfig {`；`pub struct EmbeddingRouteConfig {`；`pub struct QueryClassificationConfig {`；`pub struct ClassificationRule {`
- `crates/libs/config-runtime/src/lib.rs`：`pub struct RuntimeConfig {`；`pub struct DockerRuntimeConfig {`
- `crates/libs/config-runtime-paths/src/lib.rs`：`pub const CONFIG_JSON_FILENAME: &str = "shiju.json";`；`pub const PROD_HOME_CONFIG_DIR_NAME: &str = ".shiju-harness";`；`pub const DEV_HOME_CONFIG_DIR_NAME: &str = ".shiju-harness-dev";`；`pub const PROD_APP_DIR_NAME: &str = "shiju";`；`pub const DEV_APP_DIR_NAME: &str = "shiju-dev";`；`pub const TASK_WORKTREES_DIR_NAME: &str = "task-worktrees";`；`pub const HOME_CONFIG_DIR_NAME: &str = DEV_HOME_CONFIG_DIR_NAME;`；`pub const HOME_CONFIG_DIR_NAME: &str = PROD_HOME_CONFIG_DIR_NAME;`；`pub const ESTOP_STATE_FILE_PATH: &str = "~/.shiju-harness-dev/estop-state.json";`；`pub const ESTOP_STATE_FILE_PATH: &str = "~/.shiju-harness/estop-state.json";`；`pub const APP_DIR_NAME: &str = DEV_APP_DIR_NAME;`；`pub const APP_DIR_NAME: &str = PROD_APP_DIR_NAME;`；`pub fn home_config_dir(home: impl AsRef<Path>) -> PathBuf {`；`pub fn global_database_path(home: impl AsRef<Path>) -> PathBuf {`；`pub fn global_database_config_path(home: impl AsRef<Path>) -> PathBuf {`；`pub fn task_worktrees_dir(home: impl AsRef<Path>) -> PathBuf {`；`pub fn task_plan_storage_dir(home: impl AsRef<Path>) -> PathBuf {`；`pub fn root_config_dir() -> PathBuf {`；`pub fn tilde_config_path(relative: &str) -> String {`；`pub fn estop_state_file_path() -> String {`
- `crates/libs/config-security/src/lib.rs`：`pub use autonomy::{`；`pub use bash::{BashPolicyChecksConfig, BashSecurityConfig, BashValidatorChecksConfig};`；`pub use monitoring::{AuditConfig, EstopConfig, OtpConfig, OtpMethod, SyscallAnomalyConfig};`；`pub use sandbox::{ResourceLimitsConfig, SandboxBackend, SandboxConfig};`；`pub struct SecurityConfig {`
- `crates/libs/config-skills/src/lib.rs`：`pub enum SkillsDirectoryProvider {`；`pub enum SkillsPromptInjectionMode {`；`pub fn parse_skills_prompt_injection_mode(raw: &str) -> Option<SkillsPromptInjectionMode> {`；`pub fn parse_skills_directory_provider(raw: &str) -> Option<SkillsDirectoryProvider> {`；`pub struct SkillSecurityConfig {`；`pub struct SkillsConfig {`
- `crates/libs/config-tools/src/lib.rs`：`pub use browser::{BrowserComputerUseConfig, BrowserConfig};`；`pub use catalog::{`；`pub use multimodal::{MultimodalConfig, MultimodalOcrConfig};`；`pub use network::{`
- `crates/libs/config-ui/src/lib.rs`：`pub use gateway_client::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};`；`pub use preferences::{`；`pub use quick_messages::{`；`pub struct AppUiConfig {`；`pub struct AppSystemSettingsConfig {`
- `crates/libs/config-voice/src/lib.rs`：`pub const MAX_DICTIONARY_ENTRIES: usize = 100;`；`pub const MAX_DICTIONARY_ENTRY_CHARS: usize = 80;`；`pub const MAX_PROMPT_CHARS: usize = 2000;`；`pub enum VoiceProvider {`；`pub enum TranscriptionModel {`；`pub enum RealtimeModel {`；`pub enum RecognitionLanguage {`；`pub enum VoiceName {`；`pub struct VoiceConfig {`
- `crates/libs/config-worktree/src/lib.rs`：`pub struct WorktreeSettings {`
- `crates/libs/content-guard/src/lib.rs`：`pub mod leak_detector;`；`pub mod prompt_guard;`
- `crates/libs/display-format/src/lib.rs`：`pub use util::{duration, format_duration, number, pluralize, titlecase, truncate, truncate_middle};`
- `crates/libs/domain-matcher/src/lib.rs`：`pub mod domain_matcher;`
- `crates/libs/gateway-app-diagnostics/src/lib.rs`：`pub struct GatewayAppDiagnosticsClient {`；`pub use gateway_app_settings::AgentAppsSettingsError;`
- `crates/libs/gateway-app-failover/src/lib.rs`：`pub struct GatewayAppFailoverClient {`；`pub use agent_apps_high_availability_api::AgentAppsHighAvailabilityError;`
- `crates/libs/gateway-app-lifecycle/src/lib.rs`：`pub struct GatewayAppLifecycleClient {`；`pub use agent_apps_lifecycle_api::{AgentAppsLifecycleError, operation_path};`
- `crates/libs/gateway-app-provider/src/lib.rs`：`pub use gateway_app_read::AgentAppsReadError;`；`pub struct GatewayAppProviderClient {`
- `crates/libs/gateway-app-read/src/lib.rs`：`pub use gateway_http::AgentAppsReadError;`；`pub struct GatewayAppReadClient {`
- `crates/libs/gateway-app-settings/src/lib.rs`：`pub struct GatewayAppSettingsClient {`；`pub use agent_apps_settings_api::AgentAppsSettingsError;`
- `crates/libs/gateway-chat-stream/src/lib.rs`：`pub use types::*;`；`pub use normalize::{normalize_chat_stream_event, normalize_chat_tool_event};`；`pub use sse::{GatewaySseFrame, parse_sse_frame, take_next_sse_event};`；`pub use transport::{apply_stream_resume, stream_chat, stream_chat_resumable, stream_json_sse};`；`pub use transport::{stream_chat_blocking, stream_chat_blocking_with_idle_timeout, stream_chat_with_idle_timeout};`
- `crates/libs/gateway-endpoint/src/lib.rs`：`pub struct GatewayEndpoint {`；`pub struct GatewayAuth {`
- `crates/libs/gateway-files/src/lib.rs`：`pub use file_api::{DirectoryFileReadResponse, DirectoryFileWriteResponse};`；`pub struct GatewayFilesClient {`
- `crates/libs/gateway-http/src/lib.rs`：`pub use http::*;`；`pub use request_helpers::get_json_blocking;`；`pub struct GatewayTransport {`；`pub use agent_apps_error::AgentAppsReadError;`
- `crates/libs/gateway-model-catalog/src/lib.rs`：`pub use gateway_http::AgentAppsReadError;`；`pub struct GatewayModelCatalogClient {`
- `crates/libs/gateway-model-provider/src/lib.rs`：`pub use provider_api::{`；`pub struct GatewayModelProviderClient {`
- `crates/libs/gateway-pairing/src/lib.rs`：`pub mod pairing;`
- `crates/libs/gateway-session/src/lib.rs`：`pub use ui_load::SessionUiLoadResult;`；`pub struct GatewaySessionClient {`
- `crates/libs/git-discovery/src/lib.rs`：`pub mod git_repository;`
- `crates/libs/goal-engine/src/lib.rs`：`pub mod engine;`
- `crates/libs/json-extract/src/lib.rs`：`pub use json::extract_json_values;`
- `crates/libs/memory-algorithms/src/lib.rs`：`pub mod chunker;`；`pub mod vector;`
- `crates/libs/memory-lucid/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-mariadb/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-markdown/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-postgres/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-response-cache/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-snapshot/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-sqlite/src/lib.rs`：`pub mod memory;`
- `crates/libs/memory-types/src/lib.rs`：`pub mod memory;`
- `crates/libs/message-types/src/lib.rs`：`pub use types::*;`
- `crates/libs/model-protocols/src/lib.rs`：`pub use protocols::*;`
- `crates/libs/model-reasoning/src/lib.rs`：`pub use reasoning::*;`
- `crates/libs/model-state/src/lib.rs`：`pub use state::*;`
- `crates/libs/model-types/src/lib.rs`：`pub use types::*;`
- `crates/libs/observer-backends/src/lib.rs`：`pub mod log;`；`pub mod multi;`；`pub mod noop;`；`pub mod verbose;`；`pub use log::LogObserver;`；`pub use multi::MultiObserver;`；`pub use noop::NoopObserver;`；`pub use verbose::VerboseObserver;`
- `crates/libs/observer-otel/src/lib.rs`：`pub struct OtelObserver {`
- `crates/libs/observer-prometheus/src/lib.rs`：`pub struct PrometheusObserver {`
- `crates/libs/observer-types/src/lib.rs`：`pub enum ObserverEvent {`；`pub enum ObserverMetric {`；`pub trait Observer: Send + Sync + 'static {`
- `crates/libs/office-control-sessions/src/lib.rs`：`pub enum OfficeSurface {`；`pub enum OfficeCapability {`；`pub struct OfficeHandle {`；`pub struct OfficeSession {`；`pub struct OfficeSessions {`
- `crates/libs/ordered-id/src/lib.rs`：`pub enum Prefix {`；`pub enum Error {`；`pub fn schema(prefix: Prefix, value: &str) -> bool {`；`pub fn ascending(prefix: Prefix, given: Option<&str>) -> Result<String, Error> {`；`pub fn descending(prefix: Prefix, given: Option<&str>) -> Result<String, Error> {`；`pub fn create(`；`pub fn timestamp(id: &str) -> Option<u64> {`
- `crates/libs/permission-engine/src/lib.rs`：`pub use permission_types::{Action, Rule, Ruleset};`；`pub fn wildcard_match(text: &str, pat: &str) -> bool {`；`pub fn from_config(permission: &Value, expand: impl Fn(&str) -> String) -> Ruleset {`；`pub fn merge(rulesets: &[Ruleset]) -> Ruleset {`；`pub fn evaluate(permission: &str, pattern: &str, rulesets: &[Ruleset]) -> Rule {`；`pub fn disabled(tools: &[String], ruleset: &Ruleset) -> HashSet<String> {`
- `crates/libs/permission-types/src/lib.rs`：`pub use permission::{Action, Rule, Ruleset};`
- `crates/libs/plugin-types/src/lib.rs`：`pub struct PluginIdentity {`；`pub struct PluginInstallMetadata {`；`pub struct PluginPath {`；`pub struct PluginManifest {`；`pub enum PluginMcpServers {`；`pub enum PluginHookSource {`；`pub enum PluginAuthor {`；`pub struct PluginAuthorDetails {`；`pub struct PluginInterface {`
- `crates/libs/process-sandbox/src/lib.rs`：`pub mod traits;`；`pub mod docker;`；`pub mod bubblewrap;`；`pub mod firejail;`；`pub mod landlock;`；`pub use traits::{NoopSandbox, Sandbox, SandboxBounds};`
- `crates/libs/project-storage-paths/src/lib.rs`：`pub mod paths;`
- `crates/libs/project-types/src/lib.rs`：`pub use project::*;`
- `crates/libs/provider-errors/src/lib.rs`：`pub use error::{`
- `crates/libs/provider-model-cache/src/lib.rs`：`pub use model_reasoning::ReasoningOption;`；`pub use provider_model_catalog::{`；`pub enum ModelsRefreshStatus {`；`pub struct ModelsRefresh {`；`pub struct ModelsSnapshot {`；`pub fn invalidate_cache() {`；`pub async fn refresh() -> ModelsRefresh {`；`pub async fn get() -> HashMap<String, Provider> {`；`pub async fn snapshot() -> Arc<ModelsSnapshot> {`；`pub async fn get() -> HashMap<String, Provider> {`；`pub fn init() {}`；`pub fn init() {`
- `crates/libs/provider-model-discovery/src/lib.rs`：`pub use http::{DiscoveryError, DiscoveredModel, is_real_bearer, discover_openai_compatible_models, discover_catalog_models};`；`pub use merge::{resolve_api_base, merge_remote_models, discover_and_merge_provider};`
- `crates/libs/provider-protocol-routes/src/lib.rs`：`pub use protocol_routes::*;`
- `crates/libs/provider-runtime-paths/src/lib.rs`：`pub const APP: &str = config_runtime_paths::APP_DIR_NAME;`；`pub const CACHE_VERSION: &str = "21";`；`pub struct GlobalPaths {`；`pub fn paths() -> &'static GlobalPaths {`
- `crates/libs/question-types/src/lib.rs`：`pub use question::*;`
- `crates/libs/runtime-flags/src/lib.rs`：`pub static VIBEWINDOW_AUTO_SHARE: LazyLock<bool> =`；`pub static VIBEWINDOW_GIT_BASH_PATH: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_CONFIG: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_CONFIG_CONTENT: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_DISABLE_AUTOUPDATE: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_PRUNE: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_TERMINAL_TITLE: LazyLock<bool> =`；`pub static VIBEWINDOW_PERMISSION: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_DISABLE_DEFAULT_PLUGINS: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_LSP_DOWNLOAD: LazyLock<bool> =`；`pub static VIBEWINDOW_ENABLE_EXPERIMENTAL_MODELS: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_AUTOCOMPACT: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_CLAUDE_CODE: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_CLAUDE_CODE_PROMPT: LazyLock<bool> = LazyLock::new(|| {`；`pub static VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS: LazyLock<bool> = LazyLock::new(|| {`；`pub static VIBEWINDOW_DISABLE_EXTERNAL_SKILLS: LazyLock<bool> = LazyLock::new(|| {`；`pub static VIBEWINDOW_FAKE_VCS: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_SERVER_PASSWORD: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_SERVER_USERNAME: LazyLock<Option<String>> =`；`pub static VIBEWINDOW_EXPERIMENTAL: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_FILEWATCHER: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_ICON_DISCOVERY: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_DISABLE_COPY_ON_SELECT: LazyLock<bool> =`；`pub static VIBEWINDOW_ENABLE_EXA: LazyLock<bool> = LazyLock::new(|| {`；`pub static VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS: LazyLock<Option<u64>> =`；`pub static VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX: LazyLock<Option<u64>> =`；`pub static VIBEWINDOW_EXPERIMENTAL_OXFMT: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_LSP_TY: LazyLock<bool> =`；`pub static VIBEWINDOW_DISABLE_FILETIME_CHECK: LazyLock<bool> =`；`pub static VIBEWINDOW_EXPERIMENTAL_MARKDOWN: LazyLock<bool> =`；`pub static VIBEWINDOW_MODELS_PATH: LazyLock<Option<String>> =`；`pub fn vibewindow_disable_project_config() -> bool {`；`pub fn vibewindow_config_dir() -> Option<String> {`；`pub fn vibewindow_client() -> String {`
- `crates/libs/runtime-time/src/lib.rs`：`pub use time::{ActiveTimer, now, now_ms};`
- `crates/libs/secret-store/src/lib.rs`：`pub mod secrets;`
- `crates/libs/security-audit/src/lib.rs`：`pub enum AuditEventType {`；`pub struct Actor {`；`pub struct Action {`；`pub struct ExecutionResult {`；`pub struct SecurityContext {`；`pub struct AuditEvent {`；`pub struct AuditLogger {`；`pub struct CommandExecutionLog<'a> {`
- `crates/libs/session-info/src/lib.rs`：`pub use info::{Info, RevertInfo, ShareInfo, Summary, TimeInfo};`
- `crates/libs/session-paths/src/lib.rs`：`pub use path::*;`
- `crates/libs/session-text/src/lib.rs`：`pub use session_utils::*;`
- `crates/libs/session-ui-types/src/lib.rs`：`pub use ui_types::*;`
- `crates/libs/snapshot-types/src/lib.rs`：`pub use snapshot::{DiffStatus, FileDiff};`
- `crates/libs/syscall-anomaly/src/lib.rs`：`pub enum SyscallAnomalyKind {`；`pub struct SyscallAnomalyAlert {`；`pub struct SyscallAnomalyDetector {`
- `crates/libs/task-types/src/lib.rs`：`pub mod models;`；`pub use models::*;`
- `crates/libs/todo-types/src/lib.rs`：`pub use todo::*;`
