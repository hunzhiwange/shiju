# ShiJu Web @VERSION@

本压缩包包含完整 Web 前端所需的 HTML、JavaScript、WASM 与输入法桥接模块，无需 Rust 或重新编译。

## 本地启动

解压后，在本目录打开终端，执行：

```sh
npm run start
```

也可使用 `npm start`。启动脚本需要 Node.js 22 或以上版本（含 npm），仅使用内置模块，无需 npm install，只监听本机回环地址。
打开 <http://127.0.0.1:8765/>；按 `Ctrl+C` 停止。更换端口：

```sh
npm run start -- --port 8080
```

也可以将本目录交给已有的静态 HTTP 服务器，无需运行本脚本。

不要直接双击 index.html 使用 `file://` 打开，浏览器会阻止模块或 WASM 加载。
推荐使用支持 WebGPU 的新版 Chrome 或 Edge。本地 HTTP 回环地址可用；部署到其他主机时使用 HTTPS，并确保服务器以 `application/wasm` 返回 `.wasm` 文件，以 JavaScript MIME 类型返回 `.js` 文件。文件使用相对路径，也可部署到静态站点的子目录。

## 网关与平台能力

静态启动可显示界面。代理请求等业务功能需要另行启动 ShiJu 网关服务；前端默认连接本机网关，也可在设置中配置网关地址和认证。跨域部署时需要网关允许对应的站点来源，不要把密钥写进静态文件。

浏览器不提供桌面原生账户存储、技能安装、桌面更新安装器或独立 MCP WebView 进程。工作流涉及的文件路径必须是网关服务器可访问的路径。

## 校验下载

与 ZIP 同时发布的 `.zip.sha256` 可在解压前核对：

```sh
sha256sum -c shiju-web-@VERSION@.zip.sha256
```

macOS 使用 `shasum -a 256 -c shiju-web-@VERSION@.zip.sha256`。
