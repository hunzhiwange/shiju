# 任务14 Claude与OpenCode原生安装

2026-11-06，macOS。状态：已完成。

已核对只读总计划、来源与接口、任务13证据、正式AppId和npm固定配方。RustCodeGraph核对来源misc.rs的CLAUDE_INSTALL_UNIX、OPENCODE_INSTALL_UNIX、posix_install_command_for、installer_with_npm_fallback、build_tool_action_line及tool_action_shell_command_for_shell：POSIX/WSL官方脚本优先，Windows原生使用npm。前序正式crate、发现和Node契约存在；本任务不增加执行RPC，配方由后续授权主机执行器消费。

交付apps::lifecycle::native_primary::install_command：仅接受AppId及显式环境，只支持Claude和OpenCode。POSIX/WSL通过mktemp下载到文件，curl成功才运行bash；任何明确失败才进入任务13的精确npm回退，最终保留npm退出码。EXIT trap清理临时文件，路径加引号以适配含空格的TMPDIR。Windows返回结构化npm参数；执行器负责解析授权npm.cmd，批处理适配须保留call语义。本任务不宣称Windows/WSL实机安装证据，不改变upstream或plan.md，无UI改动。

共享槽位：原子mkdir动态取得slot-2；CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2；CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir，避免中间产物逃逸。

验证：
- cargo fmt --all：退出0；审查仅本任务Rust文件变化。
- 红：cargo test -p vw-agent-apps --lib cc_apps_primary_native_install -- --nocapture，退出101；2有效测试均因可编译空配方返回Err失败，69过滤，符合预期缺失行为。
- 实现后首次运行同命令，退出101，1通过1失败：夹具cwd断言遇macOS /var→/private/var规范化，修正为canonicalize后重新格式化和验证。
- 绿：cargo test -p vw-agent-apps --lib cc_apps_primary_native_install -- --nocapture，退出0，2通过0失败，69过滤；其中子进程用例实际执行16组（两应用×POSIX/WSL×四种结果）。
- 子进程测试替代curl与npm边界，真实bash、临时文件和cwd/env；验证固定URL/argv、原生成功不调用npm、安装器非零回退、curl写入可执行残缺脚本后失败也不执行、npm失败传播、含空格TMPDIR及所有结束路径临时文件清理。没有访问外网、全局安装或修改HOME/凭据。
- cargo clippy --all-targets --all-features -- -D warnings：退出0，1m05s，无警告（编译检查，无行为测试计数）。
- git diff --check及提交前暂存审查：退出0。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。

验证进程均已结束，释放自己的slot-2锁，保留构建缓存。仅提交本任务3个Rust文件和本证据，已有任务1–13编号删除保持原状。下一步任务15复用官方安装失败回退模式；真实应用安装验收仍按后续任务46/51执行。
