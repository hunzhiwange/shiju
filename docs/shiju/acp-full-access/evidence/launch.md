# 任务 4：普通与托管启动权限一致

日期：2026-11-06，macOS / Darwin arm64。依赖任务 3 提交 `0c52b3b7b`，此前任务 1、2 的能力矩阵与访问策略已具备。

## 行为与范围

- `agent_registry.rs` 的 Gemini、Qwen、Qoder 普通入口与本地候选仅描述 ACP 启动方式，不固定授予 `yolo`、`bypass_permissions` 或 `--skip-trust`。`managed_runtime.rs` 原有中性入口与工件哈希校验保持原样。
- `AcpClient::launch_args` 在真实进程启动边界，根据当前 `full_access && PermissionMode::ApproveAll` 生成参数。已知官方命令与精确参数形状才追加原有权限参数；`DenyAll`、`ApproveReads` 优先，配置本身不会被修改，重建启动时重新取当前授权。
- 托管 Gemini、Qwen 识别驱动已追加的精确模型路由后缀，原样保留后缀、环境与模型标识。没有新配置、全局设置写入、权限缓存或安装行为。
- 自定义参数不静默删除或改写。显式携带已识别的绕过参数且当前授权不足时，在进程启动前返回明确错误；已有完整授权时保留显式参数，不重复追加。未知自定义启动形状保持原样。`npx -y` 是包安装确认，不作为权限绕过。
- 必要相邻修复：Qwen 模型驱动原先只接受固定 yolo 参数，既拒绝托管运行时已有的 `--acp` 入口，也会拒绝新的中性普通入口。现在接受这两种中性形状，同时兼容旧显式参数；旧显式绕过仍受进程边界授权检查。模型路由、凭证隔离与工件校验未变。
- 能力报告保持任务 2 的保守分类。本任务仅证明授权与启动 argv 的关系，不证明供应商接受参数后的原生免审批、关闭原生沙箱或撤销现有进程权限；这些由后续供应商及生命周期任务验收。

## 验证记录

默认 features；全仓 Clippy 使用全部 features。动态原子领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

红测：`cargo test -p vw-acp --lib acp_full_access_launch -- --nocapture` 退出 101，有效测试 2，均在具体行为断言失败：注册表 Gemini 默认包含 yolo；自定义绕过在未授权时仍启动。首次已设置上述 CARGO_TARGET_DIR，但仓库另设的 build-dir 让中间产物落到 `target/debug`；后续阶段同时设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，最终测试二进制均来自槽位 2。一次中间验证发现测试访问私有方法编译失败，已将共享启动方法限定为客户端模块内可见并修复，不将该失败作为红测证据。

最终验证仍使用槽位 2，同步设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`：

| 命令 | 退出码 | 有效测试 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 不适用 |
| `cargo test -p vw-acp --lib acp_full_access_launch -- --nocapture` | 0 | 6 通过 |
| `cargo test -p vw-agent --lib acp_full_access_launch -- --nocapture` | 0 | 1 通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 不适用，无警告 |

`vw-agent` 测试链接器报告 `__eh_frame section too large (max 16MB)`，测试正常运行退出 0；没有调整 unwind 配置或压制告警。

脱敏边界摘要：脚本替代外部 Gemini 进程，真实 `spawn_child` 传入授权开启时的 `--experimental-acp --approval-mode yolo --skip-trust`；关闭后只传 `--experimental-acp`。自定义进程接收到原样未知参数与环境；显式绕过与未授权冲突时不创建进程。托管后缀与模型路由保持原字节。此任务不发送新的 ACP 权限控制请求，也不启动真实供应商 CLI，不构成第三方实机能力验收。

审查仅包含注册表、进程启动、Qwen 必要相邻驱动、独立测试与本证据；不包含其他工作区改动，不修改只读 plan.md。

最终槽位 2 的 owner 与锁已释放，编译缓存保留。下一步继续任务 5 的 Codex 权限适配。
