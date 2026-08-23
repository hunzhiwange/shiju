# Gateway API CLI 覆盖

`shiju api` 是 Gateway 的脚本化管理入口。覆盖门槛直接读取 Gateway 启动摘要使用的静态路由清单；当前基线为 260 条，不维护第二份路由表。测试要求每条路由属于 CLI 命令、协议别名或明确排除之一。

## 覆盖结果

| 分类 | 数量 | 说明 |
|---|---:|---|
| CLI 命令 | 247 | 映射到 22 个 `shiju api` 接口族 |
| 协议别名 | 7 | APIQL 尾斜杠 4 条、Cron 更新方法别名 2 条、Cron runs 路径别名 1 条 |
| 明确排除 | 6 | 第三方 webhook 1 条、501 占位 2 条、内部生产者回调 3 条 |
| 合计 | 260 | 由 `documented_gateway_routes()` 实时提供 |

运行时条件路由（WhatsApp、Linq、WATI、Nextcloud Talk、QQ webhook 与 node-control）不属于 260 条默认基线；它们只在对应能力启用时注册，并由 Gateway 启动摘要测试约束。

## 能力矩阵

| 接口族 | 主要能力 |
|---|---|
| `system` | health、metrics、models、状态、工具与实例发现 |
| `config` / `provider` / `control` | 配置、Provider、认证、Hook 与实例控制 |
| `operations` | Cron、集成、记忆和诊断 |
| `workspace` | 搜索、文件与受限 Git 操作 |
| `interactive` | 权限、问题和 PTY/WebSocket |
| `project` | 项目解析、详情、变更记录与 worktree |
| `knowledge` | 数据集、文档、索引、Embedding 与检索 |
| `task-pool` / `scheduler` / `staff` | 工作树池、任务调度与 Staff 编排 |
| `apiql` / `workflow` | APIQL 查询与 Workflow 应用/流 |
| `realtime` | 推理、SSE、可恢复 stop 与 WebSocket chat |
| `session` / `session-content` | 会话生命周期、消息 part、UI、归档与 scope |
| `task-lifecycle` | 任务生命周期 Workflow CRUD/reset |
| `shiju-apps` / `skill-market` | 识矩账号、应用、技能与统一市场 |
| `extensions` / `desktop` | 本地 skills/plugins 与远程宿主 desktop 接口 |

## 连接、认证与租户

```bash
shiju api \
  --host 127.0.0.1 --port 9527 \
  --platform local --company local \
  --skey-env VIBEWINDOW_SKEY \
  --directory /absolute/project \
  system status
```

`--skey-env NAME` 从环境变量读取 skey；`--skey-file PATH` 从文件读取，两者互斥。每个受管请求都携带 `X-Platform-ID` 与 `X-Company-ID`。`--directory` 是项目上下文，不与租户 ID 混用。

## 输入与输出

复杂正文统一使用 `--input`：普通路径读取 JSON 文件，`-` 从 stdin 读取，`env:NAME` 从环境变量读取。凭据、密码、登录 state 和 SHIJU credential 应使用 stdin、受限文件或环境变量，不要直接写入命令行参数。

```bash
printf '%s' '{"title":"新会话"}' |
  shiju api --directory /repo --yes session create --input -

shiju api --output json system status
shiju api --output jsonl realtime gateway-events --limit 3
```

普通响应支持 `json` 和 `jsonl`。SSE/实时协议逐事件输出 JSONL，并保留服务端错误或终止状态。Session UI 默认输出不包含只读 raw LLM 载荷；只有 `session-content raw` 会显式读取该数据。

## 写入与远程宿主确认

写入、删除、安装、推理成本操作及 desktop open/reveal/service 默认拒绝，必须显式添加 `--yes`。拒绝发生在建立业务请求之前。Desktop 确认文本包含远程 Gateway endpoint 和精确目标，避免把远程副作用误认为本机操作。

```bash
shiju api --yes --directory /repo extensions skill-enable rust --enabled true
shiju api --host gateway.example --port 9527 --yes desktop reveal /repo/output.txt
```

## 别名与排除

- APIQL 的尾斜杠路径是无尾斜杠规范路由的协议别名。
- Cron 的 `POST`/`PUT /v1/cron/{id}` 是 `PATCH` 更新别名；CLI 只提供一个 `operations cron update`。`/v1/cron/runs/{id}` 是 `/{id}/runs` 的兼容路径。
- `/v1/webhook` 是第三方入站入口，不包装为用户 CLI。
- `/v1/doc` 与 `/v1/find/symbol` 当前明确返回 501，作为占位接口排除。
- Task Pool Matrix publish、Scheduler worker-events 与 Staff IPC messages 是已认证内部生产者协议；对应 CLI 命令只返回明确不支持且不发送请求。
- 技能文件和 README 仅作为文本/JSON 输出，不会被 CLI 自动执行。

新增或修改 Gateway 路由时，必须更新 CLI 映射、别名或排除理由；否则覆盖测试会因基线或未分类路由失败。
