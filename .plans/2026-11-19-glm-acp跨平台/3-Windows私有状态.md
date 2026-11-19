# 任务 3 · Windows私有状态

先阅读并遵守 [公共执行规则](plan.md)。

批次：1，依赖无。

状态：已阻塞

目的：使 Windows GLM 模型驱动能在受验证的私有状态目录中启动。

## 文件与步骤

- 负责 `crates/libs/llm-runtime/src/acp/model_source/drivers/glm_acp_agent.rs` 及同级独立 `glm_acp_agent_tests.rs`。如需依赖声明只调整 `llm-runtime/Cargo.toml` 的 Windows 项。测试模块接线写进 GLM 驱动文件。
- 利用项目现有 Windows 安全处理及 ACL API。以实际进程身份核对目录所有权与 ACL，对当前绑定建立私有目录并拒绝不安全继承、重解析点和权限验证失败。覆盖已存在目录及并发复用。核对代理私有配置与会话目录实际落在该目录。继续验证回环地址及模型能力，保持 Unix 所有权和 `0700` 校验。
- Windows 单测以当前用户临时目录验证可用和不安全路径，使用占位令牌验证拒绝后没有启动副作用。新增过滤前缀 `glm_state_directory_`。

## 验证

在 Windows 原生隔离环境用公共槽位约定执行 `cargo test -j 1 -p llm-runtime --lib glm_state_directory_`。红测应看到安全私有目录仍因 Windows 分支拒绝。绿测须确认合法目录成功，不安全 ACL 或重解析点拒绝。复用同一过滤条件验证 Unix 路径，同时确认实际测试数。

## 执行记录

- 基线已包含 Windows 私有目录入口。复用该入口和 Windows ACL API，补充完整 DACL、所有者及目录路径检查，并验证 GLM 的凭据子目录。ACL 核验位于相邻的 `windows_private_dir.rs`，因此一并更新该文件。
- 读取固定版 GLM 1.17.1，调用其凭据及会话路径解析函数确认结果分别为绑定目录下的 `glm-acp-agent/credentials.json` 和 `<session-id>.json`。路径探查使用占位会话名，退出码 0。
- macOS arm64 运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib glm_state_directory_`。槽位 6，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。退出码 0，3 项通过，0 失败，497 项被过滤。并发复用、宽松权限和符号链接拒绝均已覆盖。
- 回归阶段确认 ACL 实现、GLM 驱动和定向测试文件与 `b443bee040` 一致，复用 macOS 3 项通过的证据。
- Windows 回归阻塞。本机是 macOS arm64。运行 `VBoxManage list vms` 时，本地入口报告 VirtualBox 应用缺失，现有磁盘为 Ubuntu；`limactl list` 仅列出 Linux 实例。未找到已授权的远程 Windows 主机或定向 CI。仓库的 Windows CI 执行全工作区测试及桌面构建，超出本任务定向范围。交叉编译无法提供 Windows ACL 和重解析点的运行证据。
- 恢复后以 `4fece76e16` 复核，ACL 实现、驱动和定向测试文件与 `b443bee040` 仍无差异。Lima 仍只有已停止的 `rcg-linux`，VirtualBox 可执行文件仍缺失且仅发现 Ubuntu 配置，SSH 配置与现有工具未提供 Windows 执行入口。Windows CI 仍无此过滤条件，未发现新的授权原生环境。沿用已有 macOS 回归结果。
- 需要一台有当前代码、Rust/MSVC 工具链、Python 和 Windows PowerShell 的隔离 Windows 验证机。使用当前用户和支持 ACL 与目录联接点的本地临时目录，按公共槽位入口运行 `cargo test -j 1 -p llm-runtime --lib glm_state_directory_`，记录红绿退出码和实际测试数。修复版应执行 4 项，覆盖 ACL、并发复用、状态重解析点和凭据重解析点。保留任务卡至通过。
