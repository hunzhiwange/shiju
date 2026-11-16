# ShiJu WASM 本地运行

## 下载发布包运行

从 GitHub Release 下载 `shiju-web-<版本>.zip` 和同名 `.zip.sha256`，校验并解压后进入包目录：

```sh
npm run start
```

运行环境为 Node.js 22 或以上版本（含 npm），无需 npm install、Python 或 Rust。
打开 <http://127.0.0.1:8765/>；更换端口执行 `npm run start -- --port 8080`。
可直接将文件交给已有静态 HTTP 服务器；不要用 file:// 打开 index.html。
压缩包内 README.md 包含浏览器、网关连接和静态部署说明。

## 从源码生成 Web 文件

完整应用入口是 `vw-desktop` 的 `shi-ju`，不是 `web_ime` 输入法验收示例。
需要 Rust 的 `wasm32-unknown-unknown` target、Python 3，以及与依赖匹配的
`wasm-bindgen 0.2.126`。在仓库根目录执行以下命令。

## 构建

使用仓库共享槽位，构建与中间产物均放在领取的槽位中。以下阶段结束后释放锁，
保留编译缓存；浏览器使用复制到 `/tmp/shiju-wasm-dist` 的独立产物。

使用仓库入口自动领取并释放槽位，不手工分配缓存路径：

```bash
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -p vw-desktop --target wasm32-unknown-unknown --bin shi-ju && "${WASM_BINDGEN:-wasm-bindgen}" --target web --out-name shi_ju --out-dir /tmp/shiju-wasm-dist "$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/shi-ju.wasm"'
```

`WASM_BINDGEN` 可指定匹配版本的工具路径。独立产物目录应为空或属于当前任务。
入口负责子进程与锁生命周期；不要清理其他任务的锁或缓存。

## 打包和预览

在源码构建环境中执行：

```sh
python3 scripts/package_shiju_web.py --bindgen-dir /tmp/shiju-wasm-dist \
  --out-dir /tmp/shiju-web-release --version 0.6.0
unzip /tmp/shiju-web-release/shiju-web-0.6.0.zip -d /tmp/shiju-web-preview
cd /tmp/shiju-web-preview/shiju-web-0.6.0
npm run start
```

版本应与 `crates/vw-desktop/Cargo.toml` 一致。打包时会生成相对路径的 index.html、
ZIP 和 SHA-256，并收齐 snippets 中的输入法模块。发布包启动仅使用 Node 内置模块，
不依赖仓库路径。`wasm-bindgen` 初始化时已调用应用入口，无需再次调用 main。

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

## 手机浏览器

同一静态包按窗口宽度自动选择布局，支持会话内显式选择。
手机访问须使用已授权的 HTTPS 站点与可达远端网关；本地预览器仍只监听回环。
完整矩阵、自动化入口及实机记录要求见 [移动端浏览器验收](移动端浏览器验收.md)。
