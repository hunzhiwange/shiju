# 任务23：安装进程执行与结果

2026-11-06，macOS。状态：已完成。

依赖核对：批次22提交6b045e327的进程内共享写锁、批次19的UpdatePlan、批次12的NodeRequirement和13–17的固定配方均已交付。读取只读plan.md、来源与接口、前序证据、do-task-plans、RustCodeGraph和git-commit技能；图索引存在，新领域实现使用源文件核实。未修改upstream、来源或plan.md。

新增process.rs及独立process_tests.rs，模块沿既有lifecycle注册。安装只按AppId选择固定配方，升级在22写锁内重新发现并通过19重新生成/比较计划，拒绝AppId不匹配、过期计划及Node Gap后才允许启动。Node检查和发现由授权执行主机的服务端回调提供，不接受客户端命令；回调必须复用12的新鲜探测，Unknown沿用其建议性放行语义。安装不执行19的升级计划；升级严格锚定其选定实例。实际授权API和回调接线归34，未新造RPC或凭据权限。

持锁等待进程退出，同时读取stdout/stderr避免管道阻塞；每路4096字节工作缓冲，诊断计数最多8192。输出内容全部脱敏为固定字节摘要，不暴露任意安装日志、凭据或panic载荷。退出码非零直接失败，即使最后一行写success；退出成功才重新发现，必须存在可运行实例，升级还核实选定实例ID、应用、主机和环境。POSIX bash启用pipefail；Windows .cmd/npm使用临时UTF-8批处理、chcp65001、call、exit /b，拒绝含批处理展开/控制字符参数。临时批处理保留到进程退出；tempfile由unix依赖移至通用依赖，版本不变、无新库。Windows/WSL安装请求在不匹配本机环境时拒绝，不在macOS执行Windows配方。

Cargo原子mkdir领取slot-2。CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；CARGO_BUILD_BUILD_DIR同一绝对路径，覆盖.cargo/config.toml的build.build-dir=target。锁记录计划、任务、会话及持有进程；所有构建与槽位产物进程退出后已释放自有锁。缓存保留。

验证：
- 红灯命令：cargo test -p vw-agent-apps --lib cc_apps_lifecycle_process_result -- --nocapture，Cargo退出101，2有效测试，1通过1失败；最小执行签名返回process execution unavailable，真实进程等待/探测用例失败。不是工具或底座缺失。
- 初次绿灯4例中因两个异步夹具竞争同一Gemini锁出现TOOL_ACTION_IN_PROGRESS，改Node用例为独立Dsh。随后4例通过。首次Clippy发现新增verify_result未接线，已修复真实升级路径后重跑。
- cargo fmt --all，退出0；审查仅保留任务领域变更，无无关格式化差异。
- 最终聚焦命令：cargo test -p vw-agent-apps --lib cc_apps_lifecycle_process_result -- --nocapture，退出0，5通过0失败，101过滤。真实临时子进程覆盖退出后才探测、退出17失败且不探测、成功文本不能掩盖失败、stderr错误文本不误判、超量输出上限与敏感串不泄漏、空实例后验失败；纯Windows夹具覆盖UTF-8/call/退出码/拒绝展开，真实POSIX管道覆盖pipefail；Node Gap和过期计划在启动前拒绝；升级真实临时npm脚本记录精确argv i/-g/@openai/codex@latest并在完成标记后探测选定实例。
- 全仓命令：cargo clippy --all-targets --all-features -- -D warnings，退出0，无警告，32.61秒。
- git diff --check，退出0。未运行无过滤全量测试、fmt检查、wasm或Makefile。无UI变更。

未执行全局包安装、未更改HOME或访问个人凭据；Windows规则夹具不是Windows/WSL/Linux实机证据。实际应用安装验收保留46–59，取消和进程树清理归26。下一步24继续后续领域，34接入真实授权操作调用链。
