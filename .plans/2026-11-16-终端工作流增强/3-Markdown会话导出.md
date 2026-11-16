# 任务 3：Markdown会话导出

先阅读并遵守[公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】；依赖：任务2。同批次可并行，推荐按编号阅读。

状态：未开始

目的：Markdown会话导出，解决requirements.md对应的实际工作流缺口。

来源任务：用户要求进一步发现有价值需求，不是细分旧输入任务。

预计会话范围：一个独立用户能力及其真实接线和聚焦验收。

## 文件与上下文

- 核查并按需修改：`crates/libs/cli-interaction/src/commands.rs`。
- 核查并按需修改：`crates/libs/cli-interaction/src/commands/routes.rs`。
- 核查并按需修改：`crates/libs/cli-interaction/src/app/runner_commands.rs`。
- 核查并按需修改：`crates/libs/cli-interaction/src/app/view.rs`。
- 上游只读参考：`/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src/slash_command.rs`，仅提取交互与边界，不复制其运行时协议。
- 新建测试：`crates/libs/cli-interaction/src/workflow_req_03_tests.rs`，在所属lib.rs或最近模块以#[cfg(test)]接线，保持单测独立文件。

## 实施步骤

1. 注册/export及帮助，用户指定输出路径并回报实际保存位置。
2. 从当前会话权威视图生成Markdown，保留角色、原文和代码围栏，二进制附件只导出安全说明而非内联密钥/路径/大数据。
3. 缺少的未加载历史明确标注，不声称全量。
4. 默认不覆盖已有文件，拒绝越界/符号链接绕过，沿用工作区校验模式。
5. 写入失败不得遗留被宣称成功的半文件，不改变会话或触发聊天请求。

## 验证计划

- 先在新测试文件定义 `workflow_req_03_` 族，覆盖以上正常、取消、错误与Unicode边界，至少6项实际有效用例。
- 失败命令：`python3 scripts/cargo_slot.py cargo test -j 1 -p cli-interaction --lib workflow_req_03_`；当前未提供目标能力时对应行为断言失败。编译失败、零测试不是红灯；若已有能力满足某项，记录并复用已有证明，不为了重写造失败。
- 实现后同命令通过：有效数至少6、failed=0、退出0。记录新输出、确切命令、槽位双路径、PID/进程组及实际运行数量。
- 真实领域模型、组合器、消息归属和请求映射不可mock；仅替身OS剪贴板/终端/故障注入/网络传输边界。文件导出使用临时工作区与真实文件操作。
- 本任务承担直接受影响其它包的必要聚焦验证，修改生命周期时在 `crates/libs/tui-terminal-lifecycle/src/tests.rs` 补同前缀，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p tui-terminal-lifecycle --lib workflow_req_03_` 并确认非零通过。
- 先核对当前相关现有测试并复用；不运行全量测试。新增签名与依赖后使用原生Cargo验证，不用旧结果替代。

## 验收与专属约束

- 要有当前失败/通过证据、实际用户入口及帮助、真实调用链、边界和平台限制；仅库内实现不能标完成。
- 共享命令表与runner接线按编号串行读取最新内容，不覆盖其它任务；本任务自行接线和提交，不能等待主调度代做。
- 上一计划聚焦输入与粘贴，本任务不可顺手改其未完成行为。源码参考和来源许可保留，不删除待审模块。
