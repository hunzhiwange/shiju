# 任务 1：整理安全复现 demo

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 1】依赖：无。推荐执行顺序：本任务为唯一任务。

状态：未开始

目的：将两个指定提交对应的 Tokio trait 实例解析问题变成可独立分享、无依赖的安全回归 demo。

来源任务：用户提供的 d34b1406fc88b71adc2b5c99f22ec3be7c69cfd9、b4268346660f2e3b8b4965292b2aa4ea6193b8ea。

预计会话范围：仅复用现有测试、建立独立包、三方验证和写说明，不开展新的编译器修复。

## 文件

- 新建：`tools/relax-implied-bounds-demo/Cargo.toml`、`tests/implied_bounds.rs`、`README.md`（后两者均在该包目录内）。
- 只读：Relax `libs/relax/compiler/tests/sharded_list_trait_instance.rs`、`libs/relax/types/src/next_solver/compare_impl_item.rs`。
- 证据：本计划目录 `证据.md`，只记录命令、版本、摘要和脱敏错误，不提交二进制或缓存。
- 不修改父 Cargo.toml、Cargo.lock 或任何业务模块；独立包自身声明空 `[workspace]`，避免主工作区接线。

## 上下文

d34b1406 将原始及归一化后的 impl 头类型加入 wf_tys，用于 resolve_regions；b4268346 把测试换成安全引用。现有两个测试分别覆盖 static 和非 static impl 约束，已经构成最小复现，不再引入 Tokio。

## 测试计划

- 创建包 `relax-implied-bounds-demo`，edition 2021，版本 0.0.0，无依赖；只放 integration test，使用现有两个测试的完整类型关系，保留来源提交说明。
- 两个原测试名保持不变，均包含 `generic_subtrait_method`，不添加额外 `'static`、unsafe 或裸指针。
- 旧运行器：`/Users/xiangmin/worker/code/relax/target/relax-before-implied-bounds` 已确认存在。先记录摘要、版本和来源证据；不能仅凭文件名断言它就是修复前版本。来源无法证明时记录证据缺口，不伪造红灯。
- 运行目录均为 `/Users/xiangmin/worker/code/shiju`，以下命令在执行阶段运行。
- 原生对照：`python3 scripts/cargo_slot.py cargo test --offline --manifest-path tools/relax-implied-bounds-demo/Cargo.toml --test implied_bounds generic_subtrait_method -- --test-threads=1`。
- 失败验证：`RELAX_STD=/Users/xiangmin/worker/code/relax/libs/relax/compiler/std RELAX_O3=0 /Users/xiangmin/worker/code/relax/target/relax-before-implied-bounds test tools/relax-implied-bounds-demo/tests/implied_bounds.rs generic_subtrait_method`。
- 通过验证：`RELAX_STD=/Users/xiangmin/worker/code/relax/libs/relax/compiler/std RELAX_O3=0 /Users/xiangmin/worker/code/relax/target/release/relax test tools/relax-implied-bounds-demo/tests/implied_bounds.rs generic_subtrait_method`。
- 对照步骤显式使用彼此隔离的 RELAX_CACHE_DIR（例如系统临时目录下的两个新目录），记录实际路径；不污染当前桌面会话缓存。若错误进入调试暂停，保存诊断后停止自有测试进程。
- 预期旧运行器在 get_shard_id 方法比较时报告签名不兼容；当前运行器和原生 Cargo 均实际运行两项、失败零项。若是其它失败，先调查而不算红灯证据。
- 无 mock，使用真实标准库、类型检查和测试 runner。统一回归即本任务三方对照，不启动桌面。

## 步骤

1. 复核两仓库状态、来源提交、现有安全测试及运行器来源，先确认与所报告缺陷一致。
2. 在独立包中复用两个测试，核验本包路径未被父 workspace 通配规则误纳入。不复制 vendor、target、字体、Tokio 或 ShiJu 源码。
3. 按上述命令验证。README 使用从 demo 包目录执行的相对命令，Relax 可执行文件由使用者设置 RELAX 绝对路径；说明必须匹配该运行器的 std，不能硬编码本机用户名到可分享说明。
4. README 解释原症状、精确修复原理、两个提交的角色和命令，明确 demo 通过不等于桌面验收通过。
5. 统计新增文件字节数，提供仅 demo 目录的分享清单，不将 Git 数据库、二进制、缓存或绝对路径配置纳入 demo。不自动推送或改写历史；小 demo 不能证明既有 push 的体积原因。
6. 审查差异、保存当前证据并按公共收尾规则提交。产生独立 Cargo.lock 时只在确有需要时纳入本包，不碰仓库根锁文件。

## 验证

- 原生和当前 Relax：各两项测试通过、退出码 0。
- 旧运行器：来源可追溯，失败指向同一方法签名比较问题，记录退出码与关键输出。
- demo 不含 unsafe、第三方依赖或大型文件；能单独复制运行；Git 差异仅包含声明路径。
- 提交报告注明测试结果、demo 字节数及未验证事项。若旧运行器来源不能确认，明确待回归，保留编号文件，不把历史对话当新证据。
