# 任务 2: sidecar 截图约束对齐

批次：【批次 1】无

状态：已完成，待回归验证

目的：让 `computer_use` 的 `screen_capture` 请求验证并转发与截图工具一致的显示器和矩形约束。

来源任务：无

预计会话范围：只处理 browser computer-use 客户端的请求验证与测试，不修改仓库外 sidecar 服务或原生截图工具。

## 文件

- 修改：`crates/vw-agent/src/tools/browser/computer_use/validation.rs`
- 修改：`crates/vw-agent/src/tools/browser/computer_use/execution.rs`
- 测试：`crates/vw-agent/src/tools/browser/computer_use/validation_tests.rs`
- 测试：`crates/vw-agent/src/tools/browser/computer_use/execution_tests.rs`

## 上下文

- `ComputerUseClient::validate_action` 已校验坐标、键盘输入及 `screen_capture.path`，`execute_action` 会把安全解析后的输出路径发给 sidecar。
- 请求中已携带窗口白名单与坐标上限；不可依赖 sidecar 自行拒绝非法几何参数。
- 外部 sidecar 的响应格式为 `success/data/error`，本任务不改变该响应协议。

## 测试计划

- 行为：`screen_capture` 的合法 `display_id` 与矩形参数被原样放入请求；非法整数、负值、零尺寸或超过已配置坐标上限的请求在网络调用前失败。
- 失败验证测试：在 `validation_tests.rs` 新增 `screen_capture_rejects_invalid_geometry`，在 `execution_tests.rs` 新增 `screen_capture_forwards_valid_geometry`。
- 失败验证命令：`cargo test -p vw-agent screen_capture_`
- 预期失败原因：现有 `screen_capture` 只验证可选路径，不验证或转发规范化的几何约束。
- 通过验证命令：`cargo test -p vw-agent computer_use`
- 模拟策略：仅 mock loopback HTTP sidecar，断言它收到的 JSON；不连接真实远程端点，也不执行系统操作。

## 步骤

1. 为输入校验和 loopback 请求体分别写失败验证测试。
2. 运行失败验证命令并确认当前实现接受非法几何或未在请求中体现约束。
3. 为 `screen_capture` 增加严格的显示器和矩形校验，复用现有坐标上限语义；不接受未知截图字段。
4. 只将通过校验的参数写入 sidecar payload，并维持路径解析、端点和认证检查。
5. 运行通过验证命令和规定的 clippy 检查；仅在测试保持通过时整理局部代码。

## 验证

- 运行：`cargo test -p vw-agent computer_use`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：computer-use 测试全部通过，非法请求无网络副作用，clippy 退出码为 0。
- 所需证据：失败测试记录、mock 请求体断言、通过测试输出、clippy 退出码和已审查差异。

## 完成

完成时应证明 sidecar 只能收到已验证的截图目标与工作区内输出路径，且配置坐标边界在客户端生效。

## 回归验证记录

- 已运行：`cargo test -p vw-agent screen_capture_`（两次）。测试进程因工作区内其他任务持有 Cargo artifact lock 而排队，未获得可记录的退出输出。
- 已运行：`cargo test -p vw-agent computer_use`。同样阻塞在 artifact lock，未获得可记录的退出输出。
- 已运行：`cargo clippy --all-targets --all-features -- -D warnings`。同样由无关并发构建占用，未获得可记录的退出输出。
- 待锁释放后依次重跑上述三项命令；通过后删除本任务文件。
