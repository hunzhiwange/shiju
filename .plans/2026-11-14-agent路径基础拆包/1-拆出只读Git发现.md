# 任务 1：拆出只读Git发现

先阅读并遵守 [公共执行规则](plan.md) 及其引用的总计划，再执行本任务。

批次：【本计划批次 1；总批次 1】依赖：.plans/2026-11-14-基础能力拆包/prompt.md 中全部任务完成。所有新任务线性执行，不并行。

状态：未开始

目的：正式迁出生产包 `git-discovery`，建立独立编译测试边界，保持原业务行为。

来源任务：无。

预计会话范围：一个既有组件及其直接依赖接线；不包含其它包的重构。

## 文件

- 来源与相邻测试：`crates/vw-agent/src/git_repository.rs`、`crates/vw-agent/src/git_repository_tests.rs`（均相对仓库根目录，目录含其原测试文件）。
- 新建：`crates/libs/git-discovery/Cargo.toml`、`crates/libs/git-discovery/src/lib.rs` 以及下述模块映射所需的实现/测试文件。
- 新包模块布局：`src/git_repository.rs`、`src/git_repository_tests.rs`，库根 `pub mod git_repository`。
- 修改：根 `Cargo.toml` 的 members、必要 `Cargo.lock` hunk、`crates/vw-agent/Cargo.toml`，以及上述来源的导出入口；只迁移本任务明确归属的测试。
- 新建调用方测试：`crates/vw-agent/src/package_boundaries/git_discovery_tests.rs`；由 `crates/vw-agent/src/package_boundaries/mod.rs` 声明，再从 lib.rs 以 cfg(test) 引入。目录和总模块首次使用时创建，后续串行追加，不引入生产抽象。

## 上下文与依赖

运行依赖：git2（沿用 agent 原有默认关闭、HTTPS/SSH、vendored 设置）；测试 tempfile。版本、target 条件、feature 设置沿用来源，不升级依赖；新包不能依赖 vw-agent、vw-shared 或桌面。原路径仅重导出同一个类型、trait、静态实例，不能留第二份实现。

## 具体实施

1. 确认外部/上一个任务完成，保存来源测试名、cfg、ignore 与本次开始时 Git 差异，核对生产调用者与工厂引用。
2. 建立包及 workspace member，agent 使用 `path = "../libs/git-discovery"` 的正式依赖；如果原模块受 feature/平台限制，依赖保持等价 optional/target 条件，不扩大默认能力。
3. 原样移动 DiscoveredRepository、discover、current_branch 和私有 canonicalize/root_commit_ids；旧 agent git_repository 仅重导出。包保持只读，不新增 Git 初始化、提交或远程操作。
4. 搬迁相邻测试及私有辅助结构，只调整跨包路径；不把私有函数普遍改 pub。不需要新包里的测试继续依赖 agent；无法切断时保留证据并停止扩大搬迁。
5. 在本任务调用方测试文件新增一个 `git_discovery_production_boundary`：显式将旧入口对象借用/赋值为新包公开类型或 trait，并通过原入口执行一个真实本地行为。无可用类型的纯路径 API 以普通仓库/linked worktree 的实际结果断言验证；不写仅比较字符串名称的空测试。
6. 按下列顺序验证，审查新包 `cargo tree` 及生产函数体的 rename diff，交接提交与当前结果。

## 验证计划

结构迁移不改变业务行为，人为制造红灯不适用。优先迁移既有行为测试，测试数量以执行前清单和平台 cfg 为准；迁出后不能通过减断言或保留重复副本凑数量。边界测试使用临时数据和真实本地实现；网络/系统边界规则见专属约束。

```sh
cargo test -j 1 -p git-discovery --lib git_repository:: &&
cargo test -j 1 -p vw-agent --lib package_boundaries::git_discovery_tests:: &&
cargo tree -p git-discovery -e normal
```

预期：原 6 个单测保留并通过，普通仓库、空仓库、linked worktree、detached HEAD 语义不变；新边界测试 1 个有效通过。所有必要命令退出 0、failed=0。新包单测编译过同一目标不重复 check；agent 边界测试同时编译直接调用者。此阶段不运行整仓检查。

所需证据：原/新测试清单映射及实际有效数、失败/ignored/内部跳过情况、生产依赖接线与树、旧入口类型身份或实际路径行为、迁移函数体无语义变化、槽位双路径/退出码/进程收尾。统一回归由总计划 D 承接，不在本任务重复。

## 专属约束

git2 及其原生依赖只在来源实际支持的平台声明；没有 WASM 构建证据不得宣称跨平台通过。边界测试使用临时仓库，绝不写用户真实仓库。
