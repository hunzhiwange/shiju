# 任务18 安装升级预览契约

2026-11-06，macOS。已完成。

已读取执行技能、只读plan.md、来源与接口、任务17证据、RustCodeGraph索引及底座交付映射/任务事件与错误契约。批次17已提交17c23a99c，前序共享AppId、AppInstallationId、环境/来源枚举及正式vw-agent-apps均已存在。沿用lib.rs已有pub mod agent_apps，只补领域模块声明。

新增固定Install/Update/Uninstall、环境或实例ID选择、服务端预览ID、来源/摘要/只读displayCommand、显式确认及执行结果DTO。预览请求与提交请求拒绝未知字段，提交只接受previewId/confirmed，不回传命令、路径、环境变量或完整计划；目标选择也拒绝额外字段。未知动作/目标形态、缺少实例ID、缺少WSL发行版、缺少确认决定均拒绝。

执行结果直接引用已有crate::id::TaskId和crate::task::TaskStatus，无新增调度器、事件、存储或RPC。底座文档将完整领域长任务实现交给母任务3；现有TaskStatus仅queued/running/completed/failed/cancelled，不能宣称已提供cancelling/interrupted恢复语义。任务26/34负责真实执行生命周期及网关接入时对接完整底座合同。当前DTO不授予权限：动作/目标兼容性、空值/失效预览、作用域授权、确认策略、锁内重新发现和计划复核由领域处理器执行，未在DTO复制业务规则。

动态领取slot-2：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；因.cargo/config.toml设置build.build-dir=target，另设置CARGO_BUILD_BUILD_DIR为相同绝对路径。所有Cargo继承两者，缓存保留。

验证：
- 初始缺失类型探针：cargo test -p vw-api-types --lib cc_apps_action_contract_roundtrip -- --nocapture，退出101；目标类型缺失，0项执行。未将该编译失败作为唯一行为证据。
- 红：补齐可编译DTO但尚未拒绝未知字段；同一命令退出101，3项执行、2通过1失败、71过滤。真实反序列化接受command字段，拒绝测试失败，符合本任务行为缺失。
- cargo fmt --all：退出0；仅保留本任务Rust差异。
- 绿：cargo test -p vw-api-types --lib cc_apps_action_contract_roundtrip -- --nocapture，退出0，3通过、0失败、71过滤；覆盖三动作往返、只读预览、命令注入字段拒绝、未知/缺失目标拒绝、显式确认及现有任务身份/状态往返。
- cargo clippy --all-targets --all-features -- -D warnings，退出0，1m28s，无警告。测试等待同槽位Cargo文件锁，未绕过槽位。
- git diff --check及git diff --cached --check退出0，审查只提交actions.rs、actions_tests.rs、agent_apps/mod.rs和本证据；排除既有编号任务删除。plan.md保持只读，无UI变更，无全局安装副作用，未执行无过滤全量测试、fmt检查、wasm检查或Makefile。

所有验证及子进程结束后释放本任务slot-2锁，保留缓存。下一步任务19；完整授权、安装副作用和恢复验收由后续任务负责。
