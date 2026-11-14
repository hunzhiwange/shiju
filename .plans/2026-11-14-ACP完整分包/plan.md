# ACP完整分包计划

目标：覆盖 vw-acp 全部生产源码，按依赖从小到大提取领域库，最终只保留薄导出与启动入口。

范围：提取路径、协议、策略、指标、租约、存储、输出、宿主 IO、注册安装、配置、队列、服务端，以及生命周期、异步控制、客户端、连接恢复、进程启动、Prompt、会话编排和全部 CLI；收敛直接消费者依赖。以 crates/libs/content-guard/Cargo.toml 的独立小库布局为参考，不复制无关依赖。

范围外：其它系统底层包的全面拆分、业务改动、重品牌、发布、构建、测试与本轮统一验收。不保留客户端、会话运行时或 CLI 业务实现于 vw-acp。

假设：本次只生成计划，后续执行只改代码；现有客户端与运行时存在生命周期类型耦合，将现有生命周期定义下沉即可解除客户端对运行时的反向依赖；其余按真实调用拆分，不创造泛化接口。

## 设计决策

- 选择按领域提取 crates/libs/acp-*；仅按文件拆包会产生碎片，整体搬走 vw-acp 则不能缩小编译单元。
- 每个新包不依赖 vw-acp；共享类型只有一份。原门面导出只用于现有调用者接线，不新增旧数据迁移层。
- 所有任务均串行：共享 Cargo.toml 与 lib.rs 会冲突，低依赖任务先执行。

## 架构说明

叶子 paths → types；types → policy/metrics/output/host-io/server；paths → lease；types/metrics/lease → storage；policy/host-io → registry → config；types/metrics/lease → queue。箭头表示先后层次，实际依赖只按源码需要声明。

生命周期 → client → connect → prompt → runtime → CLI app；异步控制 → connect/prompt；owner-process → runtime；CLI model → CLI render → CLI commands → CLI app。owner-env 的解析可依赖 owner-process，但执行入口只在 runtime。

## 开发策略

保持行为，仅移动实现、修改导入和工厂接线。用户明确取消本轮全部测试，因此不执行失败测试或通过测试，不新增行为和模拟层。现有单元测试随所属实现迁移并修复引用，但不运行。已实现不等于已验收。

## 公共执行约定

- 读取本计划与编号任务，按批次和显式依赖执行；线性顺序1至26，每批仅一个任务，无可并行批次。plan.md 生成后只读；prompt.md 仅为入口，不记录进度。
- 执行技能：/Users/xiangmin/.codex/skills/do-task-plans/SKILL.md。用户本轮“只写代码、无任何 Cargo、无测试、写完删除任务”的明确指令优先于技能及仓库默认验证规则。
- 常规、可逆、范围内决定自主完成，不逐项确认；先读取调用者和相邻实现，按现有模式接线。仅关键歧义或新增授权才集中询问。局部阻塞继续不受影响部分，不扩大功能或削弱安全。
- 每个新包新建 Cargo.toml 与 src/lib.rs，参考 content-guard 的版本0.6.0、edition 2024、MIT、doctest=false；依赖版本和 SDK features 从实际源码与原清单提取，不能整份复制 vw-acp 依赖。根 workspace 显式注册，vw-acp 增加 path 依赖，移除原实现并保留必要导出。
- 不以 #[path] 跨包引用原源码，不复制生产实现，不新增泛化 trait/开关。跨包私有符号只按实际调用需要调整；保留 Unix/Windows cfg。
- 迁移模块内和根部独立 *_tests.rs 的注册、super/crate 引用与 fixture 路径；不得让新库 dev-dependency 回指 vw-acp。原 tests/ 中公共门面集成测试可保留，避免误删覆盖。
- 工作区已有 Cargo.lock 和其它代码变更，禁止覆盖或纳入本任务提交。

## 公共静态审阅与资源规则

- 本轮禁止任何 Cargo 命令，包括 metadata、check、build、test、fmt、clippy、generate-lockfile、update 和通过脚本间接调用；禁止单元/集成/实机测试、GUI、安装器和发布脚本执行。不领槽位、不启动编译器或测试进程。
- 允许读取和编辑 Cargo.toml；不手工伪造 Cargo.lock。本轮保持其既有内容，新增包锁定与 --locked CI 验收交给未来单独授权的统一验证。
- 只做源码/清单阅读、rg 引用搜索、git diff 静态审阅与 git diff --check。静态审阅不能证明可编译或测试通过。
- 未来验证不属于本计划执行入口，不创建会误启动验证的编号任务。未来由用户另行启动统一验收；届时遵循仓库 scripts/cargo_slot.py 槽位入口，Rust 重编译串行，保留缓存，仅检查新包与直接受影响目标，测试按迁移测试族聚焦执行。

## 公共收尾规则

- 状态仅用未开始、已完成、已阻塞、待回归；本轮“已完成”仅指代码迁移和静态审阅完成，全部运行验收仍未执行，不能宣称测试通过。
- 每个任务记录迁移清单、真实依赖边、静态审阅结果及未来验收目标；在删除任务前将这些证据和“未编译、未测试、未实机验收、锁文件未更新”写入中文提交说明，避免证据随文件删除丢失。
- 有实际变更时使用 /Users/xiangmin/.codex/skills/git-commit/SKILL.md，只提交本任务变更；完成任务提交前删除对应编号文件，把删除与代码一并提交，保留 plan.md 与 prompt.md。仅任务文件状态或删除不单独提交。当前计划生成阶段不自动提交。
- 客观阻塞或提交失败保留任务文件并记录原因、尝试与所需输入；不因用户明确延期测试而标阻塞或待回归。代码未完成时不得删除任务。

## 完整覆盖与拆分尺度

- 本目录完整替代旧版 `.plans/2026-11-14-ACP底层分包/` 的执行入口；旧版保持只读，不混用两套任务。若旧任务已执行，先核对产物，复用已存在的新包，不重新复制实现。
- 全部原生产模块必须有唯一归属；最后生成逐文件映射，禁止留下“暂留编排”的未分配业务。vw-acp/src/lib.rs 仅显式导出，src/main.rs 仅设置入口运行环境并调用 acp-cli-app；version.rs 中版本解析迁入 CLI model，版本值从原入口显式传入，不能误读新包版本。
- 不为了拆分机械地一文件一包。client 的 actor/state/process 共享私有状态，作为一个 client 领域库；runtime 内共享运行状态的流程保留一个 runtime 库并按模块组织。后续静态发现纯函数叶子且已有真实跨包调用时，才进一步独立，不把大包简单换目录。
- 瘦门面不意味着所有消费者都经门面：只使用底层库的消费者直接依赖相应包；需要完整 ACP 的现有入口可以使用门面。新包之间不借道 vw-acp。
- 原有测试、fixture、include_str! 和发行资源逐项登记归属；本轮不执行。最终清单为后续统一验收提供包、目标、测试族与覆盖场景，不提供本轮可执行的 Cargo 命令。
