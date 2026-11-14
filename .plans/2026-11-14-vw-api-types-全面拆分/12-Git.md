# 任务12：Git

先读取 [公共规则](plan.md) 与 [源码审计](audit.md)。

状态：未开始
批次：【批次12】；线性顺序12；依赖：任务11代码落地，编号1至11的相关契约可用。 不并行。

目标：提取到 `crates/libs/api-git`（包 `vw-api-git`）。

## 文件与符号

- `crates/vw-api-types/src/git.rs`：GitFileStatus, GitChangedFileDto, GitStatusRequest, GitStatusDto, GitDiffScope, GitDiffRequest, GitDiffDto, ListGitBranchesRequest, GitBranchDto, ListGitBranchesResponse, GitCheckoutRequest, GitCheckoutResponse, GitCommitRequest, GitHunkSelectionDto, GitLineSelectionDto, GitCommitDto, GitCommitResponse, GitCommandRequest, GitCommandResponse, GitMergeRequest, GitMergeResponse, GitRepositoryRequest, GitHistoryDetailRequest, GitRemoteRequest, GitHistoryCommitDto, GitRepositoryDto, GitHistoryFileDto, GitHistoryDetailDto, GitRemoteAction, GitRemoteResponse。
- `crates/vw-api-types/src/git_tests.rs`：模块注册、测试及私有实现一起保留。

## 实施步骤

1. 保持行/hunk 提交选择、命令请求与远程操作协议。
2. 阅读上列完整源码、相邻测试及 audit.md 的逐文件依赖和直接消费者。确认类型身份与实际重导出，不凭计划候选库名称假定已实现。
3. 沿公共规则迁移定义、私有辅助与既有测试，原模块保留明确重导出；同步清单及所有直接使用者。根导出的同一类型必须来自同一职责库。
4. 静态检查依赖单向、模块路径、serde 函数字符串路径及测试 cfg；未迁移的调用路径由旧门面兼容，不让新库反向依赖旧门面。

## 静态完成证据

- `rg -n "vw_api_types|vw-api-types" crates`：对照 audit.md 更新本领域消费者；`rg -n "crate::|super::|serde|cfg|include_" crates/libs/api-git/src` 人工核对迁移后路径。
- `git diff --check` 与 `git diff --stat`，确认仅本任务文件；不运行任何 Cargo、测试或实机。
- 定义与断言无丢失，旧路径重导出不复制类型，所有新路径与清单存在，提交后按公共规则删卡。

## 未来验收（本次未验证）

- Git既有测试的序列化、缺省、拒绝输入与正常输出保持一致；涉及跨领域的字段类型可由消费者无转换使用。
- 未来授权时覆盖新库及受影响消费者编译、原定向测试与真实边界行为；本次只提供静态证据，不宣称编译成功。

## 实际依赖与调用接线

- 本卡源文件直接外依赖：serde, serde_json。生产与测试依赖分开核对。
- 实际调用包候选（含本领域模块引用；精确文件/行见 audit.md）：`crates/vw-agent`、`crates/vw-cli`、`crates/vw-desktop`、`crates/vw-gateway-client`。不得把模块名匹配当成已完成迁移。
- 未来定向行为入口示例（只列既有源码，本次不执行）：`git_dtos_use_stable_wire_names_and_empty_lists`、`git_commit_response_defaults_created_without_overriding_explicit_false`。
