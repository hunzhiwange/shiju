# 任务 3: 清理 using-superpowers 残留

批次：【批次 1】 <依赖批次 无>

状态：未开始

目的：去掉对已删除内置技能 `using-superpowers` 的测试依赖，改用仍存在且带附属文件的内置技能。

来源任务：无

预计会话范围：仅网关桌面技能测试与确认资产目录；不改技能运行时策略，不改 Compact/Full。

## 文件

- 修改：`crates/vw-agent/src/gateway/api/handlers/desktop_skills_tests.rs`
- 确认已删除（无需再删若已不存在）：`crates/vw-agent/assets/skills/using-superpowers/**`

## 上下文

- 用户已清理 `using-superpowers` 资产；仓库 `assets/skills/` 现为 create/do-task-plans、rustcodegraph、writing-skills 等。
- `install_built_in_skill_copies_nested_resources` 仍安装 `"using-superpowers"` 并断言 `references/codex-tools.md`，会失败。
- `rustcodegraph` 内置目录含 `SKILL.md` 与 `README.md`，适合作为“复制附属文件”的替代夹具。
- 本任务不引入、不恢复 using-superpowers；不把技能注入改为 Full。

## 测试计划

- 行为：内置技能安装测试对现存带附属文件的技能成功复制 `SKILL.md` 与至少一个非清单附属文件。
- 失败验证测试：先运行现有 `install_built_in_skill_copies_nested_resources`（仍指向 using-superpowers）。
- 失败验证命令：`cargo test -p vw-agent install_built_in_skill_copies_nested_resources -- --nocapture`
- 预期失败原因：找不到 `using-superpowers` 或安装路径断言失败。
- 通过验证命令：同上（改写后）；再跑 `cargo test -p vw-agent desktop_skills -- --nocapture` 聚焦相关用例。
- 模拟策略：真实 tempdir + `install_built_in_skill`；不 mock `include_dir`。

## 步骤

1. 运行失败验证，确认 using-superpowers 残留导致失败。
2. 将测试改为安装 `rustcodegraph`（或当前 assets 中确实带附属文件的内置技能），断言 `SKILL.md` 与 `README.md`（或实际附属路径）存在。
3. 全仓再搜 `using-superpowers`；若仅测试残留则清干净；若文档/注释提及删除或改写。
4. 跑通过验证。

## 验证

- 运行：`cargo test -p vw-agent install_built_in_skill_copies_nested_resources -- --nocapture`
- 预期：安装成功且附属文件存在；仓库无 `using-superpowers` 路径引用。
- 所需证据：测试通过输出；`rg using-superpowers` 无业务/测试命中（或仅历史计划目录可忽略）。

## 完成

完成后：内置技能嵌套安装测试不再依赖已删除的 using-superpowers。
