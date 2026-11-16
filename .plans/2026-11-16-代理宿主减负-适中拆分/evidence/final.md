# 任务9：迁移收益静态核对

## 范围与证据限制

按用户覆盖指令，仅核对源码清单和归属；未运行任何 Cargo 命令、单元测试或实机测试，全部验证延期。没有实际测试发现数、退出码、通过/失败/ignored 数，也没有编译收益证据。实现收尾后删除任务9文件，不因延期验证保留。plan.md/prompt.md 只读。

核对时 HEAD 为 `5cbe1f6c3`，开始工作区干净。读取 baseline.md、task-2.md 至 task-8.md；逐项查找 before-host-tests.txt 中19个名字的当前定义，写入 after-host-tests.txt。两份 txt 都是静态候选清单，不是 Cargo 输出。RustCodeGraph status 成功，但索引时间早于本轮迁移，故当前注册和新文件以源码读取为准。

## 逐组收益

| 任务 | 基线指定用例 | 完整迁出 | 领域断言部分迁出 | 等价去重 | 完整宿主保留 | 新领域定义 | 剩余宿主定义 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 提示词 | 3 | 1 | 2 | 0 | 0 | 3 | 2 |
| 3 目录与补丁 | 3 | 3 | 0 | 0 | 0 | 3 | 0 |
| 4 写入校验 | 3 | 3 | 0 | 0 | 0 | 3 | 0 |
| 5 取消 | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| 6 成功与重试 | 2 | 2 | 0 | 0 | 0 | 2 | 0 |
| 7 溢出预算 | 4 | 4 | 0 | 0 | 0 | 4 | 0 |
| 8 裁剪边界 | 3 | 3 | 0 | 0 | 0 | 3 | 0 |
| 合计 | 19 | 17 | 2 | 0 | 0 | 19 | 2 |

以上分类互斥；“完整宿主保留”不含部分迁出的两条薄宿主用例。17条原名在指定宿主来源已无定义；19条在目标文件各有一个同名定义。具名映射和当前行号见 after-host-tests.txt；断言逐项归属见 task-2.md 至 task-8.md。本轮不新增去重收益，不把既有下沉生产算法算成本轮实现迁移。

任务2剩余两条分别为 agents_context_base_system_excludes_instruction_content、system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist。领域验证组成/正文分离；宿主验证真实 AGENTS.md 读取、上下文和可见路径。后者两层均断言路径与上下文，但输入与端口不同，保留边界覆盖，不列为重复挂载或整条去重。

## 注册与真实调用核对

- session-engine：lib.rs → prompt.rs → host_split_2_tests（cfg(test)，path 指向本库 src）；lib.rs 直接注册 host_split_7_tests（cfg(test)）、host_split_8_tests（cfg(all(test, not(wasm32)))）。宿主 prompt.rs 委托同一领域 PromptHost 实现；compaction.rs re-export 同一 is_overflow/prune。
- gateway-config：lib.rs 在 cfg(all(test, not(wasm32))) 分别注册 host_split_3_tests、host_split_4_tests；宿主 config.rs 的 router/config_patch/handle_api_config_put 仍调用领域处理器。
- task-plan-runner：lib.rs 在 cfg(test) 分别注册 host_split_5_tests、host_split_6_tests、test_host。两组共享 Once 安装测试宿主；真实宿主 runner.rs 仍 ensure_host 后委托领域执行器。
- 宿主直接注册链沿用 baseline.md：prompt.rs 的外部 host_suites 文件仍属于宿主候选；config.rs、task_plan_run/mod.rs、compaction.rs 继续挂载各自宿主测试文件。领域新增模块均各注册一次，未发现本组漏注册或重复挂载，无需接线修复。
- 检索三个领域库 src 的 path/include 声明及 Cargo.toml，未发现反向加载宿主 Rust 测试源码或 vw-agent 依赖。task-plan-runner/prompts.rs 既有 include_str! 仍引用宿主 assets/runtime-skills 中两份技能 Markdown，这是资源耦合，不能声称完全无跨目录资源引用；不属于本轮迁移或测试反向挂载。tracing target 字符串和测试合成计划中的 vw-agent 文本也不构成依赖。
- 任务8通过当前 lib-test 可执行文件和精确同名过滤创建存储隔离子进程；本次仅读取源码，未启动进程。后续需确认子进程确实发现并运行同名测试，不能只以父进程成功代替有效数量核对。

## 物理源码辅助统计

沿用基线 splitlines() 口径（含空行和注释）。

| 文件/目录 | 基线 | 当前 | 减少 |
| --- | ---: | ---: | ---: |
| vw-agent/src/**/*.rs | 135833（1381文件） | 135228（1381文件） | 605 |
| 外部 host_suites/prompt_tests.rs | 100 | 75 | 25 |
| gateway/api/handlers/config_tests.rs | 1738 | 1619 | 119 |
| task_plan_run/runner_tests.rs | 1869 | 1565 | 304 |
| session/compaction_tests.rs | 1011 | 831 | 180 |
| gateway/api/handlers/config.rs | 149 | 147 | 2 |

宿主目录605行减少与以上目录内文件之和一致；外部宿主挂载文件另减25行，不重复并入目录统计。物理减少不代表编译时间、内存或产物大小减少。基线指定19条均存在，并非旧来源已经为空；原有领域算法和宿主包装接线属于既有迁移，本轮只计测试归属变化。没有可比冷/暖缓存编译数据，未清缓存，不作提速结论。

## 宿主剩余范围与统一验证

宿主仍有135228行 Rust 源码；本计划只处理19个指定用例。真实权限、鉴权、密钥存储失败、Hook 发布、查询失效、会话持久化、目标模式、多层子代理、ACP full access、摘要生成/checkpoint及其余裁剪/溢出场景继续留在宿主。未对这些剩余范围重新展开研究或声称已迁移。

后续统一获取宿主及三个领域库真实 lib-test 清单，核对上述定义的编译归属；执行领域 host_split_2 至8（预期源码数量分别3/3/3/1/2/4/3）和两条保留宿主用例，确认任务5/6共享宿主及任务8子进程行为；补必要公共检查。原基线、所有定向测试和公共检查均未运行，本次源码核对不构成编译或行为验收。
