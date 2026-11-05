# 任务 01：应用目录共享类型

2026-11-06，macOS。仅共享协议，未实施安装、启动、授权执行或 UI。

## 依赖核实与解除阻塞

母任务1、2状态仍为未开始，不能宣称母能力全部完成。复核固定来源清单与 `app_config.rs`、`src/lib/api/types.ts` 的 SHA256 相符，源提交为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。正式包、图标 API/SDK/存储及底座交付映射已有实际产物。`docs/vibewindow/agent-apps/底座交付映射.md` 和 `任务事件与错误契约.md` 明确将目标执行端及长任务交给各领域后续实施。

用户授权突破阻塞后重新界定真实依赖：本任务仅使用已有 serde 依赖定义 DTO，不需要主机、数据库、操作或取消接口。因此独立完成协议，不伪造或调用尚未交付符号，也不扩大到执行底座。原计划及来源快照只读。

## 变更与边界

- 将现有 agent_apps.rs 原样迁入 agent_apps/mod.rs，保留图标协议及导出路径；lib.rs 已注册，无需重复修改。
- AppId 精确表达14个公开ID，拒绝未知值、grok、大小写和旧桌面别名；CLI名称作为独立元数据，grokbuild 对应 grok。
- AppRuntimeKind 明确区分 native_cli、acp_adapter、review_runtime、desktop_app、editor_integration。
- AppCatalogResponse/apps 条目包含 id、name、cliCommand、runtimeKinds；不以登记表示支持安装，不提供安装状态或动作承诺。实际14项能力目录由任务2实现。

## TDD 与检查

所有 Cargo 阶段原子领取槽位2：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，锁记录计划、任务、会话和持有 shell PID。阶段退出释放自己的锁，保留缓存。

- `cargo fmt --all`：退出0；审查无无关格式化差异。
- 红：`cargo test -p vw-api-types --lib cc_apps_catalog_roundtrip -- --nocapture`：退出101，E0432 缺失 AppCatalogResponse/AppId/AppRuntimeKind；0项执行，4项新增测试因本任务尚无协议无法编译，不以外部依赖错误代替预期失败。
- 绿：相同聚焦命令退出0，4/4通过；覆盖14个ID、未知与内部ID拒绝、五种形态互异、目录往返和嵌套未知ID拒绝。
- `cargo test -p vw-api-types --lib cc_base_icon_contract -- --nocapture`：退出0，4/4通过，保留现有图标契约。

初轮发现仓库 build.build-dir=target 独立于 target-dir；虽然 CARGO_TARGET_DIR 已指向槽位，中间产物仍落在共享 target/debug。后续将 CARGO_BUILD_BUILD_DIR 同时指向领取槽位，重新验证；不改仓库配置。

补充全快照检查：1489项中1488项 SHA256 与清单相符；services/subscription.rs 存在既有漂移，已确认与 HEAD 字节相同，非本任务产生，不修复、不作为本任务失败。任务实际来源 app_config.rs 与 types.ts 均匹配；不据此声称母任务1完整验收。

最终隔离验证：重新原子领取槽位2，同时设置 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；测试输出确认二进制位于该槽位 debug/deps。

- `cargo test -p vw-api-types --lib cc_apps_catalog_roundtrip -- --nocapture`：退出0，4/4通过。
- `cargo test -p vw-api-types --lib cc_base_icon_contract -- --nocapture`：退出0，4/4通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出0，无警告（初轮1m33s、隔离重跑1m26s）。
- `git diff --check`：退出0。现有图标协议逐字节保留，仅新增领域模块声明。未更改 lib.rs 的既有注册；无UI改动，无需渲染验收。
- 最终状态：已完成。验证进程全部结束后释放本任务槽位锁，缓存保留；下一步执行任务2实际能力目录。
