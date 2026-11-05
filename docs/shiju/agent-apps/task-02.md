# 任务02：十四应用能力目录

2026-11-06，macOS；纯目录领域规则，无进程、网络、文件探测或UI动作。

## 前置与范围

批次1提交 c4d9600e5 提供 AppId/AppCatalogEntry/AppRuntimeKind；正式 vw-agent-apps 已注册。首次按计划前置登记阻塞；本次用户明确授权突破，核实纯目录不调用执行主机、存储、事件或取消接口后解除。未伪造底座，未修改只读 plan.md、来源与接口.md 或 upstream。

来源为固定快照 app_config.rs::AppType::all 与 src/config/appConfig.tsx 的 APP_IDS、APP_ICON_MAP；来源与接口.md 的逐应用 CLI 映射提供13项白名单。名称和顺序逐项核对，grokbuild 的命令为 grok，Claude Desktop 没有CLI标识。运行形态只声明原生CLI或桌面，ACP投影归任务3。

公开 apps::catalog::catalog 返回14项，复用共享元数据。每项对发现、版本、安装、更新、卸载、终端启动、偏好提供三态与原因。CLI动作尚未迁入，明确 Unverified 并引用后续任务；Claude Desktop 的CLI安装/更新/卸载/终端启动为 Unsupported，仅限制CLI动作，不宣称桌面能力永久不支持。Supported 类型保留明确表达，当前不冒称任何动作已可执行。代理/MCP/技能只引用母任务8/11/12，不实现其规则。无安装状态字段，无注册即安装的推断。

## TDD 与验证

原子 mkdir 动态领取槽位2，记录任务、会话和持有进程；全部 Cargo 命令继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2（仓库单独配置 build-dir，需同时覆盖以隔离中间产物）

红：`cargo test -p vw-agent-apps --lib cc_apps_catalog_capabilities -- --nocapture`，退出101。先出现缺失 Operation/capability 的目标契约编译错误；补最小签名后实际执行2项，1失败、1通过，明确断言空目录0项不等于14项。后续加强能力测试的非空断言，避免空目录导致循环空通过。

`cargo fmt --all` 退出0；差异审查仅本任务新增代码，既有任务1删除不触碰。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。

绿：`cargo test -p vw-agent-apps --lib cc_apps_catalog_capabilities -- --nocapture`，退出0，2/2通过；覆盖14项精确身份/名称/CLI映射、13项CLI边界以及98个应用操作组合的状态/原因。拒绝场景为桌面CLI生命周期/终端启动 Unsupported，其余保持 Unverified，不执行任何动作。构建日志确认二进制在槽位2 debug/deps。

`cargo clippy --all-targets --all-features -- -D warnings`，退出0，耗时1m08s，无警告。`git diff --check` 退出0。无UI改动，无需渲染验收；不宣称任何平台真实安装通过。

状态：已完成。所有验证进程结束后释放自己的槽位2锁并保留缓存；下一步任务3现有ACP注册投影。仅提交本任务规则、测试及本证据，任务1既有删除不纳入。
