# ACP 会话外挂

`shiju acp` 提供真实 stdio 服务端。初始化响应的 `_meta["shiju.mounts"]`
声明 `version: 1`、`skills: true`、`setMounts: true` 后，客户端才可使用下列
ShiJu 命名空间扩展。这些字段不是 ACP 标准。

创建会话（`session/new`）示例：

```json
{
  "cwd": "/absolute/project",
  "mcpServers": [],
  "_meta": {
    "shiju.skillMounts": {
      "version": 1,
      "skills": [{"name": "example", "path": "/absolute/skills/example"}]
    }
  }
}
```

`path` 是**服务端执行环境可访问**的绝对技能目录，包含 `SKILL.md`，附件、
references 和 scripts 保持原生目录结构。它不是跨机器上传接口。服务端重新
审核来源，建立租户、项目、会话及 revision 绑定的目录视图。目录发现只暴露
技能元数据；正文由原生 `skill` 工具按需读取，不预先拼入用户 prompt。
用户显式点名技能时，仍沿用原生的显式引用预加载规则。
挂载不授予 full_access；文件与脚本仍受原生工具权限检查。

新建/加载响应 `_meta["shiju.mounts"]` 返回当前 `revision`、技能名称 `skills`
及 MCP 名称 `mcpServers`，不回传凭据。`mcpServers` 沿用标准 ACP 的 stdio、
HTTP 与 SSE 配置结构。

空闲时发送 `_shiju/session/set_mounts`（JSON-RPC wire 方法名，含开头下划线）：

```json
{
  "sessionId": "returned-session-id",
  "expectedRevision": 0,
  "skills": [{"name": "example", "path": "/absolute/skills/example-v2"}],
  "mcpServers": []
}
```

这是完整替换：必须提供 `skills` 和 `mcpServers`，空数组明确清空。成功才
发布新集合并返回递增 revision；预期版本冲突、会话忙碌、审核或 MCP 准备
失败均返回错误并保留旧集合。忙碌时先取消并等候 prompt 结束，再用旧 revision
重试；不要在未确认结果时猜测版本。

进程重启后，用 `session/load` 加载同租户、同项目的会话。省略
`_meta["shiju.skillMounts"]` 会恢复已提交技能描述并重新审核；显式
`{"version":1,"skills":[]}` 则清空。源码变化或失效会拒绝恢复。
**MCP 凭据不落盘，客户端必须重新传入 `mcpServers`**；省略/空数组不会恢复
旧连接。恢复描述不代表跳过审核或恢复远端授权。

## 验证入口

先按仓库共享槽位规则领取锁，并设置绝对路径 `CARGO_TARGET_DIR`，在同一阶段执行：

```sh
cargo build -p vw-cli --bin shiju
cargo build -p vw-acp --bin acp
cargo test -p vw-agent --lib acp_server_mount_acceptance_ -- --nocapture --test-threads=1
python3 scripts/verify_acp_integration.py --shiju "$CARGO_TARGET_DIR/debug/shiju" --acp "$CARGO_TARGET_DIR/debug/acp" --mounts --output "$CARGO_TARGET_DIR/acp-mount-server-evidence"
```

输出目录必须是新的。`mount-results.json` 保留每项结果及失败项；使用随机附件、
真实 stdio ACP 进程、真实 Skill/文件/脚本工具和三种 MCP 工具传输。仅模型 HTTP
由本地 fixture 驱动；临时偏好配置仅授权 `mount_probe` 的模型调用，不授予
full_access，也不修改用户配置。独立 Rust 测试补充同目录会话和跨租户隔离证据。

2026-11-05 本仓库验收：2 项 Rust 聚焦测试通过（0 ignored），正式入口 19 项检查
通过；两个独立服务端进程正常退出，stdio/HTTP/SSE 实际工具调用分别为 3/1/1 次。
`cargo clippy --all-targets --all-features -- -D warnings` 退出 0。最终使用共享槽位 3，
报告保存在 `target/rust-slot-3/acp-mount-server-evidence/`；`validation.json` 记录
确切命令、退出码和测试数量，`mount-results.json` 记录逐项行为结果。

## 加载、清空与关闭示例

先从 initialize 响应读取能力；缺少声明或版本未知即拒绝扩展调用。`session/load` 省略技能命名空间以恢复目录描述，同时重新传输 MCP：

```json
{"sessionId":"returned-session-id","cwd":"/absolute/project","mcpServers":[{"name":"local-tools","command":"/absolute/bin/mcp-server","args":[],"env":[]}]}
```

明确清空两集合时发送：

```json
{"sessionId":"returned-session-id","expectedRevision":1,"skills":[],"mcpServers":[]}
```

使用最近一次成功响应的 revision；冲突后先确认生效状态再重试。SDK ext_method 输入不带 wire 前导下划线。目录不存在、来源审核失败、未知能力、忙碌或 MCP 连接失败均不得退回文本注入或忽略技能。

关闭客户端时结束 stdio 子进程并关闭管道；服务端释放会话目录视图和 MCP 连接，远端 MCP 服务由其所有者管理。恢复仍需执行端目录可访问以及有效的客户端 MCP 凭据。

2026-11-06 同轮生产复验仍为 19/19，工具调用 3/1/1；本次文档发布审查原始报告与源码散列，没有再次启动进程。固定发行物和 5 项用户豁免的准确边界见 [交付验收](acp-skill-mounts/acceptance.md)，确切命令与槽位见 [回归证据](acp-skill-mounts/evidence/regression.json)。
