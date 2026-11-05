# 任务12 Node升级依赖门控

2026-11-06，macOS。状态：已完成。

已核对只读来源与接口、上游 tool_node_requirement.rs、正式 AppInstallation 与任务11公开版本接口及证据。前序编号文件删除为既有差异，不纳入提交；plan.md、upstream保持原样。使用仓库 RustCodeGraph 技能检查索引并检索来源和调用者。

交付公开 `apps::versions::node`：

- `node_candidate` 从指定安装实例的入口目录选择 node / Windows node.exe，缺失时使用同一授权主机已解析的 PATH 候选；不重新选择默认实例。
- `update_node_gap` 接收主机已授权、重新解析的实例、目标版本、HTTP客户端及该执行环境的Node探测边界；固定 npm 注册表读取目标版本 engines.node，HTTP/响应体15秒限时。调用方使用 node_candidate 与实际主机探测器实现该边界，不把展示DTO当授权。
- `ensure_update_node` 每次重新调用探测和元数据查询，不接受缓存预览。明确 Gap 返回 Err；Unknown 保留上游放行语义，单独返回，不冒充满足。生命周期执行者须在写锁内重新解析实例并紧邻升级调用；执行接线属于后续任务19/34，本任务不引入执行主机RPC。
- mcode自带Node、bun、Homebrew、原生布局与WSL保留不适用规则。WSL不使用宿主Windows Node。Windows/WSL仅显式环境规则测试，不宣称实机执行。
- 迁入比较符、空格比较符、caret/tilde、通配、区间与OR语义；未知语法不判为满足，OR中可确认满足的分支保留通过；上界计算使用checked_add避免元数据整数溢出panic。

测试与验证（所有 Cargo 使用同一动态原子领取槽位2）：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖 `.cargo/config.toml` 的 build.build-dir，防止中间产物逃逸。
- 红：`cargo test -p vw-agent-apps --lib cc_apps_node_requirement_gap -- --nocapture`，退出101；1有效测试失败、62过滤。可编译空签名对 >=24.16.0 返回None而非预期Some(false)，为本任务真实断言失败。
- `cargo fmt --all` 退出0；初次聚焦命令退出0，4通过、0失败、62过滤。
- 首次 `cargo clippy --all-targets --all-features -- -D warnings` 退出0，59.32秒。
- 补充共享执行门控的拒绝断言后，最终 `cargo fmt --all` 退出0；再次运行上述聚焦命令退出0，4通过、0失败、62过滤，0.02秒。
- 最终 `cargo clippy --all-targets --all-features -- -D warnings` 退出0，16.24秒，无警告。
- `git diff --check` 退出0；审查无无关格式化差异，无UI变化。

有效测试包括：范围成功/失败/未知；指定非默认实例旁Node与PATH回退、Windows node.exe及WSL/mcode/bun/brew/native分支；真实临时可执行Node脚本先24.16后22.0，重复探测由通过变为拒绝；真实loopback HTTP提供engines.node、空engines、503与损坏JSON，验证满足、Gap拒绝、未知放行；公开ensure接口验证接收选定实例且Node探测未知时不伪造满足。不访问外网或修改个人安装。

Cargo与夹具进程结束后释放本任务slot-2锁并保留缓存。下一步任务13实现安装配方；升级执行接线继续复用本门控。
