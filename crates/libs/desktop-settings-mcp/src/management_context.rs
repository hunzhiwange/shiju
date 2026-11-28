use vw_api_transport::TenantScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub gateway: String,
    pub tenant: TenantScope,
    pub host: String,
}
