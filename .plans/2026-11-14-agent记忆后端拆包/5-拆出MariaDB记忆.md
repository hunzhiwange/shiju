# 任务 5：拆出MariaDB记忆

先阅读并遵守 [公共执行规则](plan.md) 及其引用的总计划，再执行本任务。

批次：【本计划批次 5；总批次 10】依赖：.plans/2026-11-14-agent记忆后端拆包/4-拆出PostgreSQL记忆.md。所有新任务线性执行，不并行。

状态：未开始

目的：正式迁出生产包 `memory-mariadb`，建立独立编译测试边界，保持原业务行为。

来源任务：无。

预计会话范围：一个既有组件及其直接依赖接线；不包含其它包的重构。

## 文件

- 来源与相邻测试：`crates/vw-agent/src/memory/mariadb/`（含所有私有实现与测试），阅读 `crates/vw-agent/src/memory/mod.rs` 的对应构造/工厂（均相对仓库根目录，目录含其原测试文件）。
- 新建：`crates/libs/memory-mariadb/Cargo.toml`、`crates/libs/memory-mariadb/src/lib.rs` 以及下述模块映射所需的实现/测试文件。
- 新包模块布局：`src/memory/mariadb/`；`src/memory/mod.rs` 声明对应模块，并按实际引用重导出 memory-types 的 traits/embeddings、路径包及算法包；只导出本模块真实使用项。
- 修改：根 `Cargo.toml` 的 members、必要 `Cargo.lock` hunk、`crates/vw-agent/Cargo.toml`，以及上述来源的导出入口；只迁移本任务明确归属的测试。
- 新建调用方测试：`crates/vw-agent/src/package_boundaries/memory_mariadb_tests.rs`；由 `crates/vw-agent/src/package_boundaries/mod.rs` 声明，再从 lib.rs 以 cfg(test) 引入。目录和总模块首次使用时创建，后续串行追加，不引入生产抽象。

## 上下文与依赖

运行依赖：memory-types、anyhow、async-trait、chrono、mysql、uuid；测试 tokio。版本、target 条件、feature 设置沿用来源，不升级依赖；新包不能依赖 vw-agent、vw-shared 或桌面。原路径仅重导出同一个类型、trait、静态实例，不能留第二份实现。

## 具体实施

1. 确认外部/上一个任务完成，保存来源测试名、cfg、ignore 与本次开始时 Git 差异，核对生产调用者与工厂引用。
2. 建立包及 workspace member，agent 使用 `path = "../libs/memory-mariadb"` 的正式依赖；如果原模块受 feature/平台限制，依赖保持等价 optional/target 条件，不扩大默认能力。
3. 移动 mariadb 实现及原测试，crate::memory 引用改为新包同名 memory 命名空间；crate::app::agent 路径直接改为契约/路径包导入，不新增 app 兼容镜像。agent 原 memory::mariadb 薄导出，memory 根部类型导出及工厂继续使用原构造接口。agent 原 memory-mariadb feature 转发可选生产依赖；mysql 若已无 agent 使用再移除，默认不引入。保持标识符校验、重试、连接选项及错误文本。
4. 搬迁相邻测试及私有辅助结构，只调整跨包路径；不把私有函数普遍改 pub。不需要新包里的测试继续依赖 agent；无法切断时保留证据并停止扩大搬迁。
5. 在本任务调用方测试文件新增一个 `memory_mariadb_production_boundary`：用编译期泛型约束证明旧入口后端类型实现新包 Memory trait，并将原构造函数签名绑定为返回新包类型的函数指针；运行原构造器的非法标识符拒绝分支，不为此强行建立网络连接。无可用类型的纯路径 API 以普通仓库/linked worktree 的实际结果断言验证；不写仅比较字符串名称的空测试。
6. 按下列顺序验证，审查新包 `cargo tree` 及生产函数体的 rename diff，交接提交与当前结果。

## 验证计划

结构迁移不改变业务行为，人为制造红灯不适用。优先迁移既有行为测试，测试数量以执行前清单和平台 cfg 为准；迁出后不能通过减断言或保留重复副本凑数量。边界测试使用临时数据和真实本地实现；网络/系统边界规则见专属约束。

```sh
cargo test -j 1 -p memory-mariadb --lib memory::mariadb:: &&
cargo test -j 1 -p vw-agent --features memory-mariadb --lib package_boundaries::memory_mariadb_tests:: &&
cargo tree -p memory-mariadb -e normal
```

预期：原 20 个静态测试集合全部保留，按 cfg/外部条件核对有效与跳过数；新边界测试 1 个有效通过。所有必要命令退出 0、failed=0。新包单测编译过同一目标不重复 check；agent 边界测试同时编译直接调用者。此阶段不运行整仓检查。

所需证据：原/新测试清单映射及实际有效数、失败/ignored/内部跳过情况、生产依赖接线与树、旧入口类型身份或实际路径行为、迁移函数体无语义变化、槽位双路径/退出码/进程收尾。统一回归由总计划 D 承接，不在本任务重复。

## 专属约束

调用方边界测试通过旧入口执行真实的本地行为，使用临时独立工作区；远程后端使用上面的编译期 Memory 约束验证；纯缓存/快照以文件或缓存往返验证，不只验证类型名。保持原条件编译；测试需 Tokio/tempfile 的，仅按测试实际使用加入 dev-dependencies。 agent 命令及边界测试模块均使用原 `memory-mariadb` feature 门禁；实际执行下方 agent 边界命令时加入 `--features memory-mariadb`。真实连接测试仅使用既有隔离测试服务和 `VIBEWINDOW_TEST_MARIADB_URL`；缺失环境时内部跳过必须单列，不声称服务端往返已通过。边界测试选无网络的参数拒绝/格式校验行为，服务端覆盖仅按已具备隔离环境执行。

模块接线补充：新包 `memory` 模块按来源实际引用重导出 `memory_types::memory::traits`、`memory_types::memory::MemoryCategory`（memory-types 的 memory 根部须重导出该现有类型）；需要空嵌入器时重导出 `memory_types::memory::embeddings`，路径为 `project_storage_paths::paths`。SQLite 的 vector 引用使用 `memory_algorithms::vector`，Lucid 的 sqlite 引用使用 `memory_sqlite::memory::sqlite`；没有实际引用的项不要加入。
