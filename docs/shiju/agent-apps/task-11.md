# 任务11 最新版本查询

2026-11-06，macOS。状态：已完成。

核对来源快照、正式 vw-agent-apps、任务01–10实际代码及任务10证据；任务10公开本地版本探测已交付。使用 RustCodeGraph 读取来源版本选择与请求逻辑。不修改 upstream、只读 plan.md 或前序编号文件。

公开入口 `vw_agent_apps::apps::versions::latest::query_latest` 接收 AppId、本地版本与 HTTP 客户端，返回独立的 local_version、latest_version、update_available 和 unknown_reason。后续 API 接线归任务33。本任务无 UI 或安装执行。

- 固定 npm dist-tags 来源；Claude 仅当本地领先 latest 时纳入 next；OpenCode npm 失败回退 GitHub。
- Hermes 优先 GitHub release name 中语义版本，再看 tag；拒绝日历版本、草稿与预发布 release；失败回退 PyPI。
- agy 使用当前执行主机 OS/架构的官方 manifest；不支持平台明确返回 Unsupported。
- 每次请求含响应体读取限时15秒。网络/HTTP失败、无效JSON或版本、本地领先、本地开发构建或不可比较版本分别保守返回未知；保留本地原值，不推荐降级，不伪造最新版。
- semver 比较忽略普通 build metadata，拒绝 git describe、dirty/dev/nightly/canary/snapshot 本地构建。复用工作区已有 reqwest、semver、serde_json；tokio 仅测试依赖。
- 独立测试包含真实 loopback HTTP：成功dist-tags、HTTP503、损坏JSON、延迟响应超时；不请求外网或改动个人安装。元数据夹具覆盖 GitHub/PyPI/agy、Claude通道与脏tag。

Cargo阶段原子动态领取槽位2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖仓库 `.cargo/config.toml` 的 build.build-dir。

验证记录：

1. 红：`cargo test -p vw-agent-apps --lib cc_apps_latest_version_selection -- --nocapture`，退出101；3个有效测试全部断言失败，57过滤。可编译空实现丢失本地版本和更新，不以环境错误冒充红灯。
2. `cargo fmt --all`，退出0；审查无无关格式化差异。
3. 同一聚焦测试命令退出0；5通过、0失败、57过滤，0.16秒。
4. 首次 `cargo clippy --all-targets --all-features -- -D warnings` 退出101，发现本任务两个 collapsible_if，已合并明确的条件链。

5. 最终再次 `cargo fmt --all` 退出0；同一聚焦命令退出0，5通过、0失败、57过滤，0.16秒。
6. 最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出0，1分12秒，无警告。
7. `git diff --check` 退出0。审查实际变更仅本任务领域模块、独立测试、crate依赖及锁文件、本文；前序编号文件删除排除提交。无UI变化，无需UI渲染证据。

全部Cargo、HTTP夹具及产物进程结束后释放slot-2锁，保留缓存。下一步任务12继续版本依赖校验。
