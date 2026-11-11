# ShiJu WASM 本地运行

完整应用入口是 `vw-desktop` 的 `shi-ju`，不是 `web_ime` 输入法验收示例。
需要 Rust 的 `wasm32-unknown-unknown` target、Python 3，以及与依赖匹配的
`wasm-bindgen 0.2.126`。在仓库根目录执行以下命令。

## 构建

使用仓库共享槽位，构建与中间产物均放在领取的槽位中。以下阶段结束后释放锁，
保留编译缓存；浏览器使用复制到 `/tmp/shiju-wasm-dist` 的独立产物。

先从 1–20 依次用 `mkdir target/rust-slot-locks/slot-N.lock` 原子领取一个空闲槽位，
将成功槽位的编号放入 `SHIJU_WASM_SLOT`；全部占用时等待重试。在自己的锁目录中
记录任务、会话与持有进程 PID。不要使用已经存在的锁。

```bash
export CARGO_TARGET_DIR="$PWD/target/rust-slot-${SHIJU_WASM_SLOT:?请先领取空闲槽位}"
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR/build"
cargo build -p vw-desktop --target wasm32-unknown-unknown --bin shi-ju
"${WASM_BINDGEN:-wasm-bindgen}" --target web --out-name shi_ju \
  --out-dir /tmp/shiju-wasm-dist \
  "$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/shi-ju.wasm"
```

`WASM_BINDGEN` 可指定匹配版本的工具路径。成功、失败或中断后，都必须先确认本阶段
Cargo、绑定工具及其子进程全部退出，再删除自己写入的占用记录，并用 `rmdir` 释放
自己的锁目录，保留编译缓存。不得清理其他任务的锁或缓存。

## 启动

```bash
python3 scripts/serve_shiju_wasm.py /tmp/shiju-wasm-dist
```

打开 <http://127.0.0.1:8765/>，按 `Ctrl+C` 停止。可用 `--port` 指定端口。
脚本从应用的 HTML 模板生成开发入口，只监听回环地址。
`wasm-bindgen` 初始化时已经调用应用入口，不要再次调用导出的 `main`。

Web 端沿用客户端网关配置，默认连接本机网关；如需认证，在设置中配置。
浏览器不提供本机账户存储、原生技能安装、桌面更新安装器或独立 MCP WebView 进程。
视频工作流的路径应填写网关服务器可访问的路径。

## 浏览器回归

环境中需可导入 Playwright，并安装 Chrome；已有运行时依赖可通过 `NODE_PATH` 指定。

```bash
node scripts/tests/test_shiju_wasm.mjs
```

测试真实的完整产品页面、设置搜索框中的中文 IME 提交和选区替换，不发送聊天消息，
不保存业务设置。`SHIJU_WASM_URL` 可指定服务地址。截图输出到 `/tmp/shiju-wasm-*.png`，
需检查实际中文显示与搜索结果；系统输入法候选窗口仍使用本机浏览器手动验证。
