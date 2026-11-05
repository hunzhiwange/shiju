# 任务03：现有ACP注册投影

2026-11-06，macOS。纯只读领域投影，无安装、启动、权限扩展或UI改动。

## 依赖与范围

批次1、2已交付共享AppId、运行形态及14项原生目录；提交分别为c4d9600e5、a0da8ddc4。现有vw-acp已公开built_in_agent_specs、built_in_agent_registry、reviewed_agent_command及verify_reviewed_agent_executable。本任务不依赖未交付的主机执行、存储、事件或取消接口；未伪造底座。使用仓库RustCodeGraph技能检查注册与审查调用图，保持来源快照及plan.md只读。

新增apps::acp_projection：结构化视图逐项保留22项ACP注册，旧视图保留claude-legacy等兼容key。catalog_with_acp保留14项原生目录，为8项匹配应用增加ACP形态，目录外ACP仍留在完整注册视图。原生CLI与ACP没有互相覆盖。新增vw-acp依赖及最小模块声明；lib.rs已有apps注册，不重复改动。

reviewed_command直接返回原审查契约的Result<Option<...>>，缺失、校验失败及没有审查契约保持区别；Ok(None)不代表已安装或审查通过。verify_reviewed_executable沿用原版本/校验和/路径门控，未知结构化key（包括旧别名）明确拒绝，不回退任意命令。不以CLI存在推断adapter可用；未修改vw-acp或固定版本数据，无安装副作用。没有UI变化，无需渲染验收，不宣称各平台实机安装通过。

## TDD与验证

两个阶段均原子mkdir动态领取槽位2，记录任务、会话和持有shell PID；阶段结束释放自己的锁，保留缓存。所有Cargo命令同时设置：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

后者覆盖仓库独立build.build-dir，避免中间产物逃离槽位。

红：`cargo test -p vw-agent-apps --lib cc_apps_acp_projection_preserves_registry -- --nocapture`，退出101；新增3项测试因本任务缺失acp_registry、catalog_with_acp及审查投影接口产生E0425，0项执行。这是缺失目标契约的预期失败，不是无关底座错误。

`cargo fmt --all`，退出0。审查差异仅本任务代码及Cargo依赖；任务1、2既有删除不纳入提交。未运行全量测试、fmt检查、wasm检查或Makefile。

绿：`cargo test -p vw-agent-apps --lib cc_apps_acp_projection_preserves_registry -- --nocapture`，退出0，3/3通过，28项过滤。覆盖22项完整结构化规格相等、旧兼容视图相等、14项应用保留多运行形态、22项审查结果与真实所有者相等、未知key拒绝及普通pi命令不能通过审查门控。运行二进制位于槽位2/debug/deps。

`cargo clippy --all-targets --all-features -- -D warnings`，退出0，1m24s，无警告。`git diff --check`退出0。全部验证子进程已结束，槽位2锁已释放，缓存保留。状态：已完成；推荐执行任务4。
