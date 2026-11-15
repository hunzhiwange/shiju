//! ACP 二进制启动壳：设置 Tokio 环境，传递原包元数据并调用 acp-cli-app。

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    acp_cli_app::run(acp_cli_model::version::VersionSource {
        package_name: env!("CARGO_PKG_NAME"),
        module_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/src"),
    })
    .await
}
