//! ACP 二进制运行时入口。

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    acp_cli_app::run(acp_cli_model::version::VersionSource {
        package_name: env!("CARGO_PKG_NAME"),
        module_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/src"),
    })
    .await
}
