//! 原入口版本元数据与版本解析接线。

pub use acp_cli_model::version::{ResolveAcpxVersionParams, UNKNOWN_VERSION};

const SOURCE: acp_cli_model::version::VersionSource = acp_cli_model::version::VersionSource {
    package_name: env!("CARGO_PKG_NAME"),
    module_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/src"),
};

pub fn resolve_vwacp_version(params: Option<ResolveAcpxVersionParams<'_>>) -> String {
    acp_cli_model::version::resolve_vwacp_version(params, SOURCE)
}

pub fn get_vwacp_version() -> String {
    acp_cli_model::version::get_vwacp_version(SOURCE)
}
