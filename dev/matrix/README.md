# Local Matrix Lab

本目录提供本地 Synapse + Element Web，用于验证 VibeWindow Agent Scheduler 的 Matrix 调度闭环。

## 启动本地 Matrix

```bash
dev/matrix/matrix-local.sh up
```

- Synapse: `http://localhost:8008`
- Element Web: `http://localhost:8081`
- 调度房间 ID: `!pBPalASRisJteUrmut:localhost`
- 调度房间 alias: `#vibewindow-scheduler:localhost`
- bot: `@vibewindow-bot:localhost`
- workers: `@developer:localhost`, `@tester:localhost`, `@reviewer:localhost`

脚本会注册本地用户、创建或复用调度房间、邀请三个 worker 用户，并写出：

- `dev/matrix/runtime/state.env`
- `dev/matrix/runtime/config/vibewindow.json`

`state.env` 保存本次本地 Synapse 的实际 room id 和本地 token；`vibewindow.json` 保存 bot 默认 Matrix 通道、三个 worker Matrix 实例，以及默认员工到 Matrix profile 的绑定。真实 token 只写在 `runtime` 目录，不要提交。

## 启动 VibeWindow

生产一致路径是让 daemon 监督 gateway、scheduler 和 Matrix workers：

```bash
cargo run -p vw-cli --features channel-matrix --bin vibewindow -- \
  --config-dir dev/matrix/runtime/config \
  daemon
```

也可以只启动 gateway/app，再用脚本单独启动 Rust worker。注册 worker 前设置目标项目和 gateway：

```bash
export PROJECT_PATH=/absolute/path/to/project
export VIBEWINDOW_GATEWAY_URL=http://127.0.0.1:42617
```

## 注册 Worker

```bash
dev/matrix/matrix-local.sh register-workers
```

这会把三个 Agent Scheduler worker 快照注册到同一个 Matrix 房间：

```text
matrix-dev    -> @developer:localhost (executor)
matrix-test   -> @tester:localhost (planner)
matrix-review -> @reviewer:localhost (reviewer)
```

也可以在桌面端“系统设置 → 员工配置 → Workers”里用同一组员工 Matrix 绑定注册 worker。脚本只输出 worker id 和 room，不输出 token。为避免旧本地快照抢任务，`register-workers` 会把历史 `matrix-local` 标记为 `unavailable`。

## 启动 Rust Worker

启动全部 worker：

```bash
dev/matrix/matrix-local.sh worker-run-all
```

只启动某一个 worker：

```bash
dev/matrix/matrix-local.sh worker-run dev
dev/matrix/matrix-local.sh worker-run test
dev/matrix/matrix-local.sh worker-run review
```

脚本会优先使用当前仓库已构建的 `target/debug/vibewindow`，再退到 PATH 中的
`vibewindow` 或 `cargo run`；如需指定二进制，设置 `VIBEWINDOW_BIN=/path/to/vibewindow`。
脚本启动前会停止同一 project/worker 的旧进程，避免多个 Matrix 进程抢同一会话。

等价的底层 CLI：

```bash
vibewindow --config-dir dev/matrix/runtime/config \
  agent-scheduler matrix-worker \
  --project-dir "$PROJECT_PATH" \
  --worker-id matrix-dev
```

## 创建任务并验证

1. 在任务看板中把“调度方式”设为 `Matrix`。
2. 创建任务，或把任务移动到待执行。
3. 确认 Matrix 房间出现 `com.vibewindow.agent_scheduler.dispatch.v1` 结构化事件。
4. 确认对应 Rust worker 发送 `started`、`submitted`、`completed` 或 `failed` worker-events。
5. 确认任务板进入完成或失败状态。

最近房间事件可用脚本查看：

```bash
dev/matrix/matrix-local.sh sync
```

## 本地聊天测试

```bash
dev/matrix/matrix-local.sh send-text "scheduler smoke test"
dev/matrix/matrix-local.sh send-task-event T-local-1 running
dev/matrix/matrix-local.sh sync
```

## 停止

```bash
dev/matrix/matrix-local.sh down
```

保留数据卷时下次启动会复用用户和房间。完全清理可执行：

```bash
dev/matrix/matrix-local.sh reset
```
