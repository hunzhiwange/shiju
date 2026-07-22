# 运行时：原生、WebAssembly、Docker

VibeWindow 支持多种运行时环境：原生、Rust WebAssembly 和 Docker。

## 概述

运行时抽象层（Runtime Abstraction Layer）提供统一的接口，屏蔽不同运行时环境的差异。

## 原生运行时

原生运行时直接在主机操作系统上执行：

```rust
pub struct NativeRuntime {
    tokio_runtime: Runtime,
    thread_pool: ThreadPool,
}

impl Runtime for NativeRuntime {
    async fn spawn(&self, task: Task) -> JoinHandle;
    async fn block_on(&self, future: Future);
}
```

### 特点

- 最高性能
- 完全访问系统资源
- 需要手动资源管理

## WebAssembly 运行时

WebAssembly 运行时在沙箱环境中执行：

```rust
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    linker: wasmtime::Linker,
}

impl Runtime for WasmRuntime {
    async fn spawn(&self, module: Module) -> Result<Instance>;
    fn instantiate(&self, bytes: &[u8]) -> Result<Instance>;
}
```

### 特点

- 跨平台执行
- 沙箱安全隔离
- 启动速度快
- 内存安全

## Docker 运行时

Docker 运行时提供容器化执行：

```rust
pub struct DockerRuntime {
    docker: Client,
    image: String,
    network: Network,
}

impl Runtime for DockerRuntime {
    async fn create_container(&self, config: ContainerConfig) -> Result<Container>;
    async fn start(&self, id: ContainerId) -> Result<()>;
}
```

### 特点

- 完全隔离
- 可移植性强
- 资源限制
- 快速扩缩容

## 运行时选择

| 场景 | 推荐运行时 |
|------|-----------|
| 高性能任务 | 原生 |
| 沙箱执行 | WebAssembly |
| 复杂依赖 | Docker |