# 取消授权 API

母任务 6／批次 41。批次 40 由 `c2ba747d8` 交付。[接口契约](接口契约.md) · [授权轮询](接通授权轮询API.md) · [主机内取消与清理](阻止取消后回调复活登录.md)

`DELETE /v1/agent-apps/auth/flows/{flow_id}?host_id=…&service=…` 复用管理 Basic 认证、租户头、可信主体令牌和对应服务的 `auth_flow_start` grant。流程仅在已授权的平台、公司、主机、主体与服务作用域内解析。在另一个有权限的租户或主体下查不到该流程，返回 `404 not_found`。缺失认证或操作权限时沿用管理链的 `401` 或 `403`。响应使用 `Cache-Control: no-store`。

取消先关闭流程的提交资格，再执行主机清理。Claude 的处理器等待原 CLI 退出及隔离目录和钥匙串清理结果，再确认取消。清理失败返回稳定错误，迟到的回填或兑换结果仍无法提交。取消成功与重复确认均返回空的 `204`，轮询返回 `cancelled`。其余已知终态返回 `409 conflict`。DSH 继续由原生适配器尽力发送上游取消，本地确认以本地清理结果为准。

## macOS 验证

macOS 27.0 arm64。测试使用真实回环 HTTP、管理策略文件、SQLite、临时 PostgreSQL 18 和 macOS 钥匙串。脚本子进程模拟外部 Claude CLI，产生受管目录文件并等待回填。成功用例在两个数据库后端都确认首次和重复 DELETE 返回 204，且进程已回收，隔离目录及测试专用钥匙串条目均已删除。随后验证迟到的回填和成功提交结果均被拒绝，账号列表为空。拒绝用例验证两个获准租户之间的隔离、另一获准主体、错误服务、缺失管理认证、只读主体以及已批准终态。实际供应商账号验收由后续批次完成。

Cargo 各阶段动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。格式化在源码快照执行，只回写本批 Rust 文件。

红灯阶段，SQLite 和 PostgreSQL 的所有者取消实际返回 501，预期 204。跨租户取消实际返回 501，预期 404。首次绿灯验证中，测试钥匙串条目缺少匹配账号资料，触发既有身份校验并返回 409。补齐夹具资料后重新运行全部验证。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯与绿灯 | `cargo fmt --all` | 0 | 通过 |
| 红灯 | `cargo test -p vw-agent --lib auth6_api_cancel -- --nocapture` | 101 | 3 项预期失败，11780 项过滤 |
| 绿灯 | `cargo test -p vw-agent --lib auth6_api_cancel -- --nocapture` | 0 | SQLite 2 项、PostgreSQL 1 项通过，0 项失败，11780 项过滤 |
| 授权 API 回归 | `cargo test -p vw-agent --lib auth6_api_ -- --nocapture` | 0 | 14 项通过，0 项失败，11769 项过滤 |
| 全工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0 | 通过 |

测试链接器仍有既有的 `__eh_frame` 大小提示。临时 PostgreSQL 已停止，槽位锁已释放，编译缓存已保留。
