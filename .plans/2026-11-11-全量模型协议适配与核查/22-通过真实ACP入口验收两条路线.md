# 任务 22: 通过真实ACP入口验收两条路线

批次：【批次 16】 依赖批次 15；直接依赖任务 21

状态：未开始

目的：在新构建网关中证明原生与转换策略贯穿CLI、会话和实际工具行为。

来源任务：用户要求全量核查 api.json、shiju.json 并实现原生优先与协议转换。

预计会话范围：围绕本任务的一个主要验证目标执行，复用前序已交付合同和现有同职责模块。资料任务只写本组报告，代码任务按批次串行。

## 文件

- 修改：`scripts/verify_model_protocol_routes.py`
- 修改：`scripts/test_verify_model_protocol_routes.py`
- 参考：`scripts/verify_acp_model_source.py`
- 新建：`docs/dev/model-protocols/acp-acceptance.json`

## 上下文

- 先读本目录plan.md、调研基线.md、目录基线.json及本任务；所有新路径与测试名称是拟定交付。前置任务文件删除后，以其正式产物和提交证据确认交付。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
- 开始前检查当前源码与Git差异，核实前置交付，保留其它任务变更；只在本任务文件写进度和阻塞。
- 本批次只有本任务；共享解析、路由、转换、格式化及Clippy与其它代码任务串行执行。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

本仓库`.cargo/config.toml`还设置了共享`build-dir = "target"`。领取槽位后同时设置`export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，使中间产物落在同一已领取槽位。测试使用所列crate的默认features，Clippy使用全workspace全部targets/features。

## 验证计划

- 行为：同一可用模型在真实ACP原生及强制转换测试中均能完成流式写读并正常结束。
- 测试驱动开发不直接适用于官方资料查询、真实服务验收及最终汇总；先检查缺项和已知失败，随后以资料、实际响应与覆盖结果验证。脚本或生产行为需要修改时，仍先新增可编译的失败测试。
- 主验证命令：`python3 -m unittest discover -s scripts -p "test_verify_model_protocol_routes.py"`。
- 模拟策略：查询直接访问官方资料；实时验收使用现有账号与真实服务，离线矩阵不得标为实测通过。

- 脚本失败验证测试：在`scripts/test_verify_model_protocol_routes.py`新增`test_acp_result_requires_binding_and_tool_output`，预期红灯为握手成功却缺少模型绑定或实际工具文件仍记为通过；与主验证命令相同，修复后该测试与周边用例须全部通过。

## 步骤

1. 给脚本增加acp子命令，参数为inventory/output，复用既有ACP验证脚本的清单、会话及退出码检查。
2. 从槽位构建新的网关二进制，在隔离测试配置和临时工作区中运行；验证二进制与源修订、租户、工作区、模型及源协议绑定。
3. 覆盖Messages、Responses、Gemini和Chat四类已安装可用客户端。阿里和DeepSeek必须包含同模型原生与强制转换对照，Shiju路由按实际公开接口核查。
4. 强制转换只放在隔离验收夹具与明确诊断候选中，生产默认原生优先。真实CLI执行Write/Read或等价工具，检查文件哈希及目录范围。
5. 读取完整SSE和最终事件、握手与acp-options等实际入口；无法取得CLI或凭据时记录具体受限覆盖，继续其它客户端，最终不能伪报已验证。
6. Rust修改后执行`cargo fmt --all`并审查差异范围；执行聚焦验证和全仓Clippy，只处理本项引入的问题。

## 验证

- 运行：`python3 -m unittest discover -s scripts -p "test_verify_model_protocol_routes.py"`。
- 运行：`cargo build -p vw-cli --bin shiju`。
- 运行：`python3 scripts/verify_model_protocol_routes.py acp --inventory docs/dev/model-protocols/catalog-inventory.json --output docs/dev/model-protocols/acp-acceptance.json`。
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`。
- 预期：本任务主要行为得到当前证据，覆盖计数匹配实际源文件，失败与未验证状态可追溯。
- 所需证据：确切命令、退出码、有效测试数或完整性检查计数、失败前后差异、核查日期、已审查文件；Rust/Cargo任务还记录槽位和绝对CARGO_TARGET_DIR。
- 接口/流式验收记录HTTP、实际绑定、事件序列和最终状态。只编译成功、握手成功或HTTP200不足以判定任务完成。

## 完成与阻塞

- 所需证据齐全后标记`已完成`。有实际代码、测试、配置或文档变更，使用技能`$git-commit`核对并只提交本任务变更，报告证据后删除本任务文件。只有本任务状态更新或删除则不单独提交。
- 遇到问题先调查并在既有授权内处理实现原目标所需的相邻依赖；仍有实际依赖缺失则在本文件标记`已阻塞`，记录复现、已尝试路径和所需输入。
- 无关既有问题导致回归无法执行，可标记`已完成，待回归`并保留任务文件；按上述规则提交实际变更。缺少完成本任务本身所必需的实现或验证，不能据此宣称完成。
- plan.md为只读，prompt.md不是进度来源；正式证据写入本任务列明的产物。
